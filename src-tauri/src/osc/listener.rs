use rosc::{OscPacket, OscType};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::net::UdpSocket;
use tokio::sync::oneshot;

#[derive(Clone, Debug, Serialize)]
pub struct OscPayload {
    pub address: String,
    pub args: Vec<OscArg>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum OscArg {
    Float(f32),
    Int(i32),
    String(String),
    Bool(bool),
}

pub async fn run_osc_listener(
    port: u16,
    app: AppHandle,
    mut rx_stop: oneshot::Receiver<()>,
) {
    let addr = format!("0.0.0.0:{}", port);
    let socket = match UdpSocket::bind(&addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[OSC] Failed to bind socket to {}: {}", addr, e);
            return;
        }
    };

    println!("[OSC] Listening on UDP port {}", port);
    let mut buf = [0u8; 65535];

    loop {
        tokio::select! {
            _ = &mut rx_stop => {
                println!("[OSC] Stopping listener on port {}", port);
                break;
            }
            res = socket.recv_from(&mut buf) => {
                match res {
                    Ok((size, _src)) => {
                        if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                            handle_packet(packet, &app);
                        }
                    }
                    Err(e) => {
                        eprintln!("[OSC] Error receiving packet: {}", e);
                    }
                }
            }
        }
    }
}

fn handle_packet(packet: OscPacket, app: &AppHandle) {
    match packet {
        OscPacket::Message(msg) => {
            let args = msg
                .args
                .into_iter()
                .map(|arg| match arg {
                    OscType::Float(f) => OscArg::Float(f),
                    OscType::Int(i) => OscArg::Int(i),
                    OscType::String(s) => OscArg::String(s),
                    OscType::Bool(b) => OscArg::Bool(b),
                    _ => OscArg::String(format!("{:?}", arg)),
                })
                .collect();

            let payload = OscPayload {
                address: msg.addr,
                args,
            };

            let _ = app.emit("osc-event", payload);
        }
        OscPacket::Bundle(bundle) => {
            for inner in bundle.content {
                handle_packet(inner, app);
            }
        }
    }
}
