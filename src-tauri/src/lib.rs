mod osc;
#[cfg(feature = "ndi")]
mod ndi;

use std::sync::Mutex;
use tauri::Manager;
use tokio::sync::oneshot;

pub struct AppState {
    pub osc_stop_tx: Mutex<Option<oneshot::Sender<()>>>,
    pub current_osc_port: Mutex<u16>,
    pub is_osc_running: Mutex<bool>,

    #[cfg(feature = "ndi")]
    pub ws_stop_tx: Mutex<Option<oneshot::Sender<()>>>,
    #[cfg(feature = "ndi")]
    pub ndi_engine: Mutex<Option<std::sync::Arc<ndi::sender::NdiSenderEngine>>>,
    #[cfg(feature = "ndi")]
    pub ndi_stream_name: Mutex<String>,
    #[cfg(feature = "ndi")]
    pub is_ndi_running: Mutex<bool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Initialize app state in IDLE state (waiting for UI commands)
            app.manage(AppState {
                osc_stop_tx: Mutex::new(None),
                current_osc_port: Mutex::new(9000),
                is_osc_running: Mutex::new(false),

                #[cfg(feature = "ndi")]
                ws_stop_tx: Mutex::new(None),
                #[cfg(feature = "ndi")]
                ndi_engine: Mutex::new(None),
                #[cfg(feature = "ndi")]
                ndi_stream_name: Mutex::new("Andragon Output".to_string()),
                #[cfg(feature = "ndi")]
                is_ndi_running: Mutex::new(false),
            });

            println!("[Desktop Application] Native backend initialized in idle state.");

            Ok(())
        })
        .invoke_handler({
            #[cfg(feature = "ndi")]
            {
                tauri::generate_handler![
                    osc::start_osc,
                    osc::stop_osc,
                    osc::get_osc_status,
                    ndi::start_ndi,
                    ndi::stop_ndi,
                    ndi::get_ndi_status
                ]
            }
            #[cfg(not(feature = "ndi"))]
            {
                tauri::generate_handler![
                    osc::start_osc,
                    osc::stop_osc,
                    osc::get_osc_status
                ]
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

