pub mod sender;
pub mod server;

use std::sync::Arc;
use serde::Serialize;
use tauri::State;
use tokio::sync::oneshot;
use crate::AppState;
use sender::NdiSenderEngine;

#[derive(Serialize)]
pub struct NdiStatus {
    pub running: bool,
    pub stream_name: String,
}

#[tauri::command]
pub async fn start_ndi(stream_name: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut running_guard = state.is_ndi_running.lock().unwrap();
    let mut name_guard = state.ndi_stream_name.lock().unwrap();

    // Stop existing WebSocket server / NDI sender if active
    let mut stop_tx_guard = state.ws_stop_tx.lock().unwrap();
    if let Some(tx) = stop_tx_guard.take() {
        let _ = tx.send(());
    }

    let ndi_engine = Arc::new(NdiSenderEngine::new(&stream_name));
    let ndi_clone = Arc::clone(&ndi_engine);

    let (tx, rx) = oneshot::channel();
    *stop_tx_guard = Some(tx);
    *state.ndi_engine.lock().unwrap() = Some(ndi_engine);
    *name_guard = stream_name.clone();
    *running_guard = true;

    tauri::async_runtime::spawn(server::run_frame_ws_server(ndi_clone, rx));
    println!("[NDI] Started streaming under name '{}'", stream_name);

    Ok(())
}

#[tauri::command]
pub async fn stop_ndi(state: State<'_, AppState>) -> Result<(), String> {
    let mut stop_tx_guard = state.ws_stop_tx.lock().unwrap();
    if let Some(tx) = stop_tx_guard.take() {
        let _ = tx.send(());
    }

    *state.ndi_engine.lock().unwrap() = None;
    let mut running_guard = state.is_ndi_running.lock().unwrap();
    *running_guard = false;
    println!("[NDI] Stream stopped by UI command.");

    Ok(())
}

#[tauri::command]
pub fn get_ndi_status(state: State<'_, AppState>) -> NdiStatus {
    NdiStatus {
        running: *state.is_ndi_running.lock().unwrap(),
        stream_name: state.ndi_stream_name.lock().unwrap().clone(),
    }
}
