use std::sync::{Arc, Mutex};

use crate::state::AppState;

use mutsukitube_anime::AnimeManager;

#[derive(Clone)]
pub struct AppContext {
    pub state: Arc<Mutex<AppState>>,
    pub handle: tokio::runtime::Handle,
    pub anime_manager: Arc<AnimeManager>,
}
