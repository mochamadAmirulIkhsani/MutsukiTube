pub mod errors;
pub mod ipc;
pub mod manager;
pub mod models;
pub mod process;
pub mod registry;

pub use errors::{AnimeError, ErrorCategory, *};
pub use ipc::{IpcErrorBody, IpcRequest, IpcResponse};
pub use manager::AnimeManager;
pub use models::{Anime, Episode, PROTOCOL_VERSION, PluginInfo, StreamInfo};
pub use process::{ProcessStatus, ProcessTimeouts, ProviderProcess};
pub use registry::{
    ProviderConfig, ProviderRegistry, discover_provider_binary, plugin_dir_from_env,
};

pub const CRATE_NAME: &str = "mutsukitube-anime";
