use std::sync::Arc;
use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::accept_async;
use super::sender::NdiSenderEngine;

pub async fn run_frame_ws_server(
    ndi: Arc<NdiSenderEngine>,
    mut rx_stop: oneshot::Receiver<()>,
) {
    let addr = "127.0.0.1:9001";
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[WebSocket] Failed to bind server to {}: {}", addr, e);
            return;
        }
    };

    println!("[WebSocket] Canvas Frame Receiver active on ws://{}", addr);

    loop {
        tokio::select! {
            _ = &mut rx_stop => {
                println!("[WebSocket] Stopping Canvas Frame Receiver Server.");
                break;
            }
            res = listener.accept() => {
                if let Ok((stream, _)) = res {
                    let ndi_ref = Arc::clone(&ndi);
                    tauri::async_runtime::spawn(async move {
                        if let Ok(mut ws_stream) = accept_async(stream).await {
                            println!("[WebSocket] Web client connected for NDI frame streaming.");
                            while let Some(Ok(msg)) = ws_stream.next().await {
                                if msg.is_binary() {
                                    let data = msg.into_data();
                                    if data.len() >= 8 {
                                        let width = u32::from_le_bytes(data[0..4].try_into().unwrap());
                                        let height = u32::from_le_bytes(data[4..8].try_into().unwrap());
                                        let rgba_pixels = &data[8..];

                                        if rgba_pixels.len() as u32 == width * height * 4 {
                                            ndi_ref.send_rgba_frame(width, height, rgba_pixels);
                                        }
                                    }
                                }
                            }
                            println!("[WebSocket] Web client disconnected.");
                        }
                    });
                }
            }
        }
    }
}
