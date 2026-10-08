pub mod database;
pub mod history;

pub use database::{database_path, open_connection};

pub use history::{clear_history, get_history, save_video};
