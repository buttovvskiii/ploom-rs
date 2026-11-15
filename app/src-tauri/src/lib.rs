//! Ploom desktop app: the Tauri 2 shell around [`ploom_core`].

mod commands;
mod events;
mod state;

use std::time::Duration;

use ploom_core::{SessionConfig, SessionHandle};
use tauri::{Manager as _, RunEvent};
use tracing_subscriber::EnvFilter;

use crate::state::AppState;

const DISCONNECT_ON_EXIT: Duration = Duration::from_secs(2);

/// Builds and runs the app.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn,ploom_core=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_catalog,
            commands::get_activity,
            commands::scan,
            commands::connect,
            commands::disconnect,
            commands::set_vibration,
            commands::set_lock,
            commands::set_auto_start,
            commands::set_limit_settings,
            commands::apply_profile,
            commands::preview_profile,
            commands::read_logs,
            commands::read_diagnostics,
            commands::send_raw,
        ])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            // `SessionHandle::start` spawns its supervisor with `tokio::spawn`,
            // so it has to run inside Tauri's Tokio runtime.
            let session = {
                let runtime = tauri::async_runtime::handle();
                let _guard = runtime.inner().enter();
                SessionHandle::start(&SessionConfig {
                    data_dir,
                    auto_connect: true,
                })
            };
            events::forward(app.handle(), &session);
            app.manage(AppState { session });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build the Tauri application");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            disconnect_on_exit(app);
        }
    });
}

fn disconnect_on_exit(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let session = state.session.clone();
    let result = tauri::async_runtime::block_on(async move {
        tokio::time::timeout(DISCONNECT_ON_EXIT, session.disconnect()).await
    });
    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("disconnect on exit failed: {e}"),
        Err(_) => tracing::warn!("disconnect on exit timed out"),
    }
}
