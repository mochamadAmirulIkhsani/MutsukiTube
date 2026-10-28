use crate::errors::*;
use crate::ipc::{IpcRequest, IpcResponse, MAX_IPC_LINE_BYTES};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, oneshot};
use tokio::time::{Duration, timeout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    Cold,
    Running,
    Failed,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessTimeouts {
    pub startup_ms: u64,
    pub info_ms: u64,
    pub search_ms: u64,
    pub episodes_ms: u64,
    pub heights_ms: u64,
    pub stream_ms: u64,
    pub shutdown_ms: u64,
    pub max_pending: usize,
}

impl Default for ProcessTimeouts {
    fn default() -> Self {
        Self {
            startup_ms: 5_000,
            info_ms: 10_000,
            search_ms: 30_000,
            episodes_ms: 60_000,
            heights_ms: 60_000,
            stream_ms: 60_000,
            shutdown_ms: 5_000,
            max_pending: 64,
        }
    }
}

impl ProcessTimeouts {
    pub fn for_action(&self, action: &str) -> u64 {
        match action {
            "info" => self.info_ms,
            "search" => self.search_ms,
            "episodes" => self.episodes_ms,
            "heights" => self.heights_ms,
            "stream" => self.stream_ms,
            _ => self.episodes_ms,
        }
    }
}

struct ProcessState {
    child: Option<Child>,
    stdin: Option<tokio::process::ChildStdin>,
    stdout_task: Option<tokio::task::JoinHandle<()>>,
    stderr_task: Option<tokio::task::JoinHandle<()>>,
    pending: HashMap<String, oneshot::Sender<Result<IpcResponse, AnimeError>>>,
    counter: u64,
    status: ProcessStatus,
    stderr_tail: Vec<String>,
    restarts: u32,
    last_start: Option<std::time::Instant>,
    max_restarts: u32,
}

pub struct ProviderProcess {
    pub provider_id: String,
    pub executable: PathBuf,
    pub executable_args: Vec<String>,
    pub expected_protocol: u32,
    pub timeouts: ProcessTimeouts,
    state: Arc<Mutex<ProcessState>>,
}

impl ProviderProcess {
    pub fn new(
        provider_id: impl Into<String>,
        executable: PathBuf,
        executable_args: Vec<String>,
        expected_protocol: u32,
        timeouts: ProcessTimeouts,
    ) -> Arc<Self> {
        let state = Arc::new(Mutex::new(ProcessState {
            child: None,
            stdin: None,
            stdout_task: None,
            stderr_task: None,
            pending: HashMap::new(),
            counter: 0,
            status: ProcessStatus::Cold,
            stderr_tail: Vec::new(),
            restarts: 0,
            last_start: None,
            max_restarts: 5,
        }));
        Arc::new(Self {
            provider_id: provider_id.into(),
            executable,
            executable_args,
            expected_protocol,
            timeouts,
            state,
        })
    }

    pub async fn request(
        &self,
        action: &str,
        params: Value,
        deadline_ms: Option<u64>,
    ) -> Result<IpcResponse, AnimeError> {
        let rust_timeout_ms = deadline_ms.unwrap_or_else(|| self.timeouts.for_action(action));
        let rust_timeout_ms = rust_timeout_ms.max(100);

        let (request_id, rx) = {
            let mut state = self.state.lock().await;
            self.ensure_started(&mut state).await?;

            if state.pending.len() >= self.timeouts.max_pending {
                return Err(AnimeError::new(
                    CATEGORY_PENDING_LIMIT,
                    format!(
                        "provider {} has too many in-flight requests",
                        self.provider_id
                    ),
                    true,
                ));
            }

            state.counter += 1;
            let request_id = format!("req-{}", state.counter);
            let (tx, rx) = oneshot::channel();
            state.pending.insert(request_id.clone(), tx);

            let ipc_request = IpcRequest::new(request_id.clone(), action, params)
                .with_deadline(deadline_ms.unwrap_or_else(|| self.timeouts.for_action(action)));

            let mut bytes = serde_json::to_vec(&ipc_request)
                .map_err(|e| AnimeError::new(CATEGORY_MALFORMED_RESPONSE, e.to_string(), false))?;
            bytes.push(b'\n');

            let write_result = if let Some(stdin) = state.stdin.as_mut() {
                let write = stdin.write_all(&bytes).await;
                if write.is_err() {
                    None
                } else {
                    Some(stdin.flush().await)
                }
            } else {
                None
            };

            if write_result.is_none() || write_result.unwrap().is_err() {
                state.pending.remove(&request_id);
                state.status = ProcessStatus::Failed;
                if let Some(child) = state.child.as_mut() {
                    let _ = child.kill().await;
                }
                state.child = None;
                state.stdin = None;
                self.abort_tasks(&mut state);
                return Err(AnimeError::new(
                    CATEGORY_PROCESS_EXITED,
                    format!("failed to write to provider {}", self.provider_id),
                    true,
                ));
            }

            (request_id, rx)
        };

        match timeout(Duration::from_millis(rust_timeout_ms), rx).await {
            Ok(Ok(Ok(response))) => {
                if response.protocol_version != self.expected_protocol {
                    return Err(AnimeError::new(
                        CATEGORY_PROTOCOL_VERSION_MISMATCH,
                        format!(
                            "provider {} protocol {} does not match expected {}",
                            self.provider_id, response.protocol_version, self.expected_protocol
                        ),
                        false,
                    ));
                }
                if response.success {
                    Ok(response)
                } else if let Some(err) = response.error {
                    Err(crate::errors::AnimeError {
                        category: err.code,
                        message: err.message,
                        retryable: err.retryable,
                    })
                } else {
                    Err(AnimeError::new(
                        CATEGORY_UNSUPPORTED,
                        format!(
                            "provider {} returned an unsuccessful response without an error",
                            self.provider_id
                        ),
                        false,
                    ))
                }
            }
            Ok(Ok(Err(err))) => Err(err),
            Ok(Err(_)) => {
                let mut state = self.state.lock().await;
                state.pending.remove(&request_id);
                Err(AnimeError::new(
                    CATEGORY_PROCESS_EXITED,
                    format!(
                        "provider {} closed while request was in flight",
                        self.provider_id
                    ),
                    true,
                ))
            }
            Err(_) => {
                let mut state = self.state.lock().await;
                state.pending.remove(&request_id);
                if let Some(child) = state.child.as_mut() {
                    let _ = child.kill().await;
                }
                state.child = None;
                state.stdin = None;
                state.status = ProcessStatus::Failed;
                self.abort_tasks(&mut state);
                Err(AnimeError::new(
                    CATEGORY_TIMEOUT,
                    format!(
                        "provider {} timed out after {} ms",
                        self.provider_id, rust_timeout_ms
                    ),
                    true,
                ))
            }
        }
    }

    async fn ensure_started(&self, state: &mut ProcessState) -> Result<(), AnimeError> {
        if let Some(child) = state.child.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    state.child = None;
                    state.stdin = None;
                    state.status = ProcessStatus::Failed;
                    self.abort_tasks(state);
                }
                Ok(None) => {
                    if state.status == ProcessStatus::Running || state.status == ProcessStatus::Cold
                    {
                        state.status = ProcessStatus::Running;
                        return Ok(());
                    }
                }
                Err(e) => {
                    state.child = None;
                    state.stdin = None;
                    state.status = ProcessStatus::Failed;
                    self.abort_tasks(state);
                    return Err(AnimeError::new(
                        CATEGORY_PROCESS_EXITED,
                        e.to_string(),
                        true,
                    ));
                }
            }
        }

        if state.status == ProcessStatus::Failed {
            if state.restarts >= state.max_restarts {
                return Err(AnimeError::new(
                    CATEGORY_PROCESS_EXITED,
                    format!("provider {} exceeded restart limit", self.provider_id),
                    false,
                ));
            }
            if let Some(last) = state.last_start {
                if last.elapsed() < Duration::from_millis(250) {
                    return Err(AnimeError::new(
                        CATEGORY_PROCESS_EXITED,
                        format!("provider {} is restarting too quickly", self.provider_id),
                        true,
                    ));
                }
            }
        }

        let mut child = Command::new(&self.executable)
            .args(&self.executable_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                AnimeError::new(
                    CATEGORY_PROCESS_SPAWN_FAILED,
                    format!("failed to spawn {}: {}", self.executable.display(), e),
                    false,
                )
            })?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        if stdin.is_none() || stdout.is_none() {
            let _ = child.kill().await;
            return Err(AnimeError::new(
                CATEGORY_PROCESS_SPAWN_FAILED,
                format!("provider {} did not expose piped stdio", self.provider_id),
                false,
            ));
        }

        let stdout = stdout.unwrap();
        let stderr = stderr.unwrap();
        let stdin = stdin.unwrap();

        let state_arc = Arc::clone(&self.state);
        let provider_id = self.provider_id.clone();
        let max_bytes = MAX_IPC_LINE_BYTES;
        let stdout_handle = tokio::spawn(stdout_reader(
            state_arc.clone(),
            stdout,
            max_bytes,
            provider_id.clone(),
        ));
        let stderr_handle = tokio::spawn(stderr_reader(state_arc, stderr, provider_id.clone()));

        state.child = Some(child);
        state.stdin = Some(stdin);
        state.stdout_task = Some(stdout_handle);
        state.stderr_task = Some(stderr_handle);
        state.status = ProcessStatus::Running;
        state.restarts += 1;
        state.last_start = Some(std::time::Instant::now());
        Ok(())
    }

    fn abort_tasks(&self, state: &mut ProcessState) {
        if let Some(handle) = state.stdout_task.take() {
            handle.abort();
        }
        if let Some(handle) = state.stderr_task.take() {
            handle.abort();
        }
    }

    pub async fn shutdown(&self) {
        let (child, stdin, stdout_task, stderr_task) = {
            let mut state = self.state.lock().await;
            state.status = ProcessStatus::ShuttingDown;
            let child = state.child.take();
            let stdin = state.stdin.take();
            let stdout_task = state.stdout_task.take();
            let stderr_task = state.stderr_task.take();
            state.pending.clear();
            (child, stdin, stdout_task, stderr_task)
        };

        drop(stdin);

        if let Some(mut child) = child {
            let wait = timeout(
                Duration::from_millis(self.timeouts.shutdown_ms),
                child.wait(),
            )
            .await;
            if wait.is_err() {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
        }
        if let Some(handle) = stdout_task {
            handle.abort();
        }
        if let Some(handle) = stderr_task {
            handle.abort();
        }
    }

    pub fn stderr_tail(&self) -> Vec<String> {
        self.state
            .try_lock()
            .map(|s| s.stderr_tail.clone())
            .unwrap_or_default()
    }

    pub fn status(&self) -> ProcessStatus {
        self.state
            .try_lock()
            .map(|s| s.status)
            .unwrap_or(ProcessStatus::Failed)
    }
}

async fn stdout_reader(
    state: Arc<Mutex<ProcessState>>,
    stdout: tokio::process::ChildStdout,
    max_bytes: usize,
    provider_id: String,
) {
    let mut reader = BufReader::new(stdout);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                fail_all_pending(
                    &state,
                    AnimeError::new(
                        CATEGORY_PROCESS_EXITED,
                        format!("provider {} closed stdout", provider_id),
                        true,
                    ),
                )
                .await;
                mark_failed(&state).await;
                break;
            }
            Ok(_) => {
                if line.len() > max_bytes {
                    fail_all_pending(
                        &state,
                        AnimeError::new(
                            CATEGORY_OVERSIZED_RESPONSE,
                            format!("provider {} sent an oversized response", provider_id),
                            false,
                        ),
                    )
                    .await;
                    kill_child(&state).await;
                    mark_failed(&state).await;
                    break;
                }

                let trimmed = line.trim_end_matches(['\n', '\r']).trim();
                if trimmed.is_empty() {
                    continue;
                }

                match serde_json::from_str::<IpcResponse>(trimmed) {
                    Ok(response) => {
                        let sender = {
                            let mut st = state.lock().await;
                            if response.request_id.is_empty() {
                                None
                            } else {
                                st.pending.remove(&response.request_id)
                            }
                        };
                        if let Some(tx) = sender {
                            let _ = tx.send(Ok(response));
                        }
                    }
                    Err(_) => {
                        let request_id =
                            serde_json::from_str::<Value>(trimmed).ok().and_then(|v| {
                                v.get("requestId")
                                    .and_then(Value::as_str)
                                    .map(str::to_string)
                            });
                        if let Some(id) = request_id {
                            let sender = {
                                let mut st = state.lock().await;
                                st.pending.remove(&id)
                            };
                            if let Some(tx) = sender {
                                let _ = tx.send(Err(AnimeError::new(
                                    CATEGORY_MALFORMED_RESPONSE,
                                    format!("provider {} sent malformed JSON", provider_id),
                                    false,
                                )));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                fail_all_pending(
                    &state,
                    AnimeError::new(
                        CATEGORY_PROCESS_EXITED,
                        format!("provider {} stdout read failed: {}", provider_id, e),
                        true,
                    ),
                )
                .await;
                mark_failed(&state).await;
                break;
            }
        }
    }
}

async fn stderr_reader(
    state: Arc<Mutex<ProcessState>>,
    stderr: tokio::process::ChildStderr,
    provider_id: String,
) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\n', '\r']).to_string();
                if trimmed.is_empty() {
                    continue;
                }
                let mut st = state.lock().await;
                st.stderr_tail.push(trimmed.clone());
                if st.stderr_tail.len() > 50 {
                    let excess = st.stderr_tail.len() - 50;
                    st.stderr_tail.drain(0..excess);
                }
                drop(st);
                if debug_logging_enabled() {
                    eprintln!("[mutsukitube-anime][{}] stderr: {}", provider_id, trimmed);
                }
            }
            Err(_) => break,
        }
    }
}

async fn fail_all_pending(state: &Arc<Mutex<ProcessState>>, err: AnimeError) {
    let senders: Vec<oneshot::Sender<Result<IpcResponse, AnimeError>>> = {
        let mut st = state.lock().await;
        st.pending.drain().map(|(_, tx)| tx).collect()
    };
    for tx in senders {
        let _ = tx.send(Err(err.clone()));
    }
}

async fn mark_failed(state: &Arc<Mutex<ProcessState>>) {
    let mut st = state.lock().await;
    st.status = ProcessStatus::Failed;
}

pub fn debug_logging_enabled() -> bool {
    std::env::var("RUST_LOG")
        .map(|value| {
            let value = value.to_lowercase();
            value.contains("debug") || value.contains("trace")
        })
        .unwrap_or(false)
        || std::env::var("MUTSUKITUBE_DEBUG")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
}

async fn kill_child(state: &Arc<Mutex<ProcessState>>) {
    let mut st = state.lock().await;
    if let Some(child) = st.child.as_mut() {
        let _ = child.kill().await;
    }
    st.child = None;
    st.stdin = None;
}
