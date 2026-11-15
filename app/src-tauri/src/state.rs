//! State managed by Tauri and shared by every command.

use ploom_core::SessionHandle;

#[derive(Debug)]
pub(crate) struct AppState {
    pub(crate) session: SessionHandle,
}
