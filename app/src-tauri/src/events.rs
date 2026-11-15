//! Forwards the session's channels to the webview as the `ploom://snapshot` and `ploom://activity`
//! events (`EVENTS` in `app/ui/src/lib/api/types.ts`).

use ploom_core::SessionHandle;
use ploom_core::model::{ActivityEvent, Snapshot};
use tauri::{AppHandle, Emitter as _, Manager as _};
use tokio::sync::{broadcast, watch};

use crate::state::AppState;

pub(crate) const SNAPSHOT: &str = "ploom://snapshot";
pub(crate) const ACTIVITY: &str = "ploom://activity";

pub(crate) fn forward(app: &AppHandle, session: &SessionHandle) {
    tauri::async_runtime::spawn(forward_snapshots(app.clone(), session.subscribe_snapshot()));
    tauri::async_runtime::spawn(forward_activity(app.clone(), session.subscribe_activity()));
}

async fn forward_snapshots(app: AppHandle, mut snapshots: watch::Receiver<Snapshot>) {
    while snapshots.changed().await.is_ok() {
        // `watch` keeps only the latest value, so a slow webview simply
        // skips intermediate snapshots.
        let snapshot = snapshots.borrow_and_update().clone();
        if let Err(e) = app.emit(SNAPSHOT, snapshot) {
            tracing::warn!("emitting {SNAPSHOT} failed: {e}");
        }
    }
}

async fn forward_activity(app: AppHandle, mut events: broadcast::Receiver<ActivityEvent>) {
    use broadcast::error::RecvError;

    // Highest id delivered so far; events are numbered monotonically.
    let mut last_id: Option<u64> = None;
    loop {
        match events.recv().await {
            Ok(event) => emit_activity(&app, &mut last_id, event),
            Err(RecvError::Lagged(skipped)) => {
                // Recover the skipped events from the session's history so
                // the feed has no gaps (anything older than the history
                // window is gone anyway).
                tracing::debug!("activity forwarder lagged by {skipped} events; replaying history");
                let history = app
                    .try_state::<AppState>()
                    .map(|state| state.session.activity_history())
                    .unwrap_or_default();
                for event in history {
                    emit_activity(&app, &mut last_id, event);
                }
            }
            Err(RecvError::Closed) => break,
        }
    }
}

fn emit_activity(app: &AppHandle, last_id: &mut Option<u64>, event: ActivityEvent) {
    if last_id.is_some_and(|last| event.id <= last) {
        return;
    }
    *last_id = Some(event.id);
    if let Err(e) = app.emit(ACTIVITY, event) {
        tracing::warn!("emitting {ACTIVITY} failed: {e}");
    }
}
