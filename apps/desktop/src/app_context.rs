use std::sync::{Arc, Mutex};

use crate::state::AppState;

#[derive(Clone)]
pub struct AppContext {
    pub state: Arc<Mutex<AppState>>,
    pub handle: tokio::runtime::Handle,
}
