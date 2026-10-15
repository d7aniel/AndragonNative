pub mod listener;

use serde::Serialize;
use tauri::{AppHandle, State};
use tokio::sync::oneshot;
use crate::AppState;

#[derive(Serialize)]
pub struct OscStatus {
    pub running: bool,
    pub port: u16,
}

#[tauri::command]
pub async fn start_osc(port: u16, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let mut running_guard = state.is_osc_running.lock().unwrap();
    let mut port_guard = state.current_osc_port.lock().unwrap();

    // Stop active listener if running
    let mut stop_tx_guard = state.osc_stop_tx.lock().unwrap();
    if let Some(tx) = stop_tx_guard.take() {
        let _ = tx.send(());
    }

    let (tx, rx) = oneshot::channel();
    *stop_tx_guard = Some(tx);
    *port_guard = port;
    *running_guard = true;

    tauri::async_runtime::spawn(listener::run_osc_listener(port, app, rx));
    println!("[OSC] Started listening on UDP port {}", port);

    Ok(())
}

#[tauri::command]
pub async fn stop_osc(state: State<'_, AppState>) -> Result<(), String> {
    let mut stop_tx_guard = state.osc_stop_tx.lock().unwrap();
    if let Some(tx) = stop_tx_guard.take() {
        let _ = tx.send(());
    }

    let mut running_guard = state.is_osc_running.lock().unwrap();
    *running_guard = false;
    println!("[OSC] Listener stopped by UI command.");

    Ok(())
}

#[tauri::command]
pub fn get_osc_status(state: State<'_, AppState>) -> OscStatus {
    OscStatus {
        running: *state.is_osc_running.lock().unwrap(),
        port: *state.current_osc_port.lock().unwrap(),
    }
}
