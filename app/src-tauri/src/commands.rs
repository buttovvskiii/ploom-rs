//! One `#[tauri::command]` per entry of the `Commands` table in `app/ui/src/lib/api/types.ts`.

#![allow(clippy::needless_pass_by_value)]

use ploom_core::model::{
    ActivityEvent, Catalog, CommandError, Diagnostics, DiscoveredDevice, FramePreview,
    LimitSettings, LogsReport, ProfileTarget, RawExchange, Snapshot,
};
use tauri::State;

use crate::state::AppState;

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub(crate) fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    state.session.snapshot()
}

#[tauri::command]
pub(crate) fn get_catalog(state: State<'_, AppState>) -> Catalog {
    state.session.catalog()
}

#[tauri::command]
pub(crate) fn get_activity(state: State<'_, AppState>) -> Vec<ActivityEvent> {
    state.session.activity_history()
}

#[tauri::command]
pub(crate) async fn scan(state: State<'_, AppState>) -> CommandResult<Vec<DiscoveredDevice>> {
    Ok(state.session.scan().await?)
}

#[tauri::command]
pub(crate) async fn connect(state: State<'_, AppState>, id: Option<String>) -> CommandResult<()> {
    Ok(state.session.connect(id).await?)
}

#[tauri::command]
pub(crate) async fn disconnect(state: State<'_, AppState>) -> CommandResult<()> {
    Ok(state.session.disconnect().await?)
}

#[tauri::command]
pub(crate) async fn set_vibration(state: State<'_, AppState>, on: bool) -> CommandResult<()> {
    Ok(state.session.set_vibration(on).await?)
}

#[tauri::command]
pub(crate) async fn set_lock(state: State<'_, AppState>, locked: bool) -> CommandResult<()> {
    Ok(state.session.set_lock(locked).await?)
}

#[tauri::command]
pub(crate) async fn set_auto_start(state: State<'_, AppState>, enabled: bool) -> CommandResult<()> {
    Ok(state.session.set_auto_start(enabled).await?)
}

#[tauri::command]
pub(crate) fn set_limit_settings(
    state: State<'_, AppState>,
    settings: LimitSettings,
) -> CommandResult<()> {
    Ok(state.session.set_limit_settings(settings)?)
}

#[tauri::command]
pub(crate) async fn apply_profile(
    state: State<'_, AppState>,
    target: ProfileTarget,
) -> CommandResult<()> {
    Ok(state.session.apply_profile(target).await?)
}

#[tauri::command]
pub(crate) async fn preview_profile(
    state: State<'_, AppState>,
    target: ProfileTarget,
) -> CommandResult<FramePreview> {
    Ok(state.session.preview_profile(target).await?)
}

#[tauri::command]
pub(crate) async fn read_logs(state: State<'_, AppState>) -> CommandResult<LogsReport> {
    Ok(state.session.read_logs().await?)
}

#[tauri::command]
pub(crate) async fn read_diagnostics(state: State<'_, AppState>) -> CommandResult<Diagnostics> {
    Ok(state.session.read_diagnostics().await?)
}

#[tauri::command]
pub(crate) async fn send_raw(
    state: State<'_, AppState>,
    hex: String,
    confirm_dangerous: bool,
) -> CommandResult<RawExchange> {
    Ok(state.session.send_raw(&hex, confirm_dangerous).await?)
}
