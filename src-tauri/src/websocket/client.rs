use std::time::Duration;

use super::keyboard::simulate_key_press;
#[cfg(target_os = "macos")]
use crate::commands::{check_accessibility, check_screen_recording, show_permission_window};
use crate::shortcuts::global::{register_global_shortcut, unregister_global_shortcut};
use fstdict_common::window::notification::show_notification;
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::tungstenite::Utf8Bytes;

use super::protocol::{build_connect_message, InboundMessage};

/// Reconnection delay after WebSocket disconnect (milliseconds).
const RECONNECT_DELAY_MS: u64 = 200;

/// Starts the WebSocket client loop with automatic reconnection.
///
/// The client maintains two outbound channels:
/// - `outbound_main_rx`: messages originating from the main window
pub async fn start_ws_client(
    ws_url: &str,
    app_handle: AppHandle,
    outbound_main_rx: mpsc::Receiver<String>,
) {
    // Create merger ONCE outside the reconnection loop to avoid moving receivers twice
    let mut outbound_merged = OutboundMerger::new(outbound_main_rx);
    let mut reconnect_count = 0;
    loop {
        info!("Connecting to Python WebSocket: {}", ws_url);
        reconnect_count += 1;

        match connect_async(ws_url).await {
            Ok((ws_stream, _)) => {
                info!("Python WebSocket connected");
                let (mut write, mut read) = ws_stream.split();

                // Send connection handshake
                let handshake = WsMessage::Text(Utf8Bytes::from(build_connect_message()));
                let _ = write.send(handshake).await;
                // Main event loop
                loop {
                    tokio::select! {
                        // Inbound: messages from the C++ server
                        msg = read.next() => {
                            match msg {
                                Some(Ok(WsMessage::Text(text))) => {
                                    handle_inbound_message(&app_handle, &text, &mut write).await;
                                }
                                Some(Ok(WsMessage::Close(_))) => {
                                    info!("WebSocket closed by server");
                                    let app_clone = app_handle.clone();
                                    let _ = app_handle.run_on_main_thread(move || {
                                        info!("Received exit request from WebSocket. Exiting application.");
                                        app_clone.exit(0);
                                    });
                                    break;
                                }
                                Some(Err(e)) => {
                                    error!("WebSocket read error: {}", e);
                                    break;
                                }
                                None => {
                                    info!("WebSocket stream ended");
                                    break;
                                }
                                _ => {}
                            }
                        }
                       // Outbound: messages from Tauri commands / UI
                        Some(payload) = outbound_merged.recv() => {
                            let msg = WsMessage::Text(Utf8Bytes::from(payload));
                            if let Err(e) = write.send(msg).await {
                                error!("Failed to send WebSocket message: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("WebSocket connection failed: {}", e);
            }
        }

        // Wait before attempting reconnection
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS * reconnect_count)).await;
    }
}

async fn handle_inbound_message<S>(app: &AppHandle, text: &str, write: &mut S)
where
    S: SinkExt<WsMessage> + Unpin,
    S::Error: std::fmt::Display,
{
    match serde_json::from_str::<InboundMessage>(text) {
        Ok(event) => dispatch_message(app, event, write).await,
        Err(e) => error!("Failed to parse WebSocket JSON: {} | raw: {}", e, text),
    }
}

async fn dispatch_message<S>(app: &AppHandle, event: InboundMessage, _write: &mut S)
where
    S: SinkExt<WsMessage> + Unpin,
    S::Error: std::fmt::Display,
{
    match event {
        InboundMessage::TauriNotification { data } => {
            info!("Received notification: {}", data.message);
            let app_clone = app.clone();
            let msg = data.message;
            let _ = app.run_on_main_thread(move || {
                if let Err(e) = show_notification(&app_clone, msg) {
                    error!("show_notification error: {}", e);
                }
            });
        }

        InboundMessage::OcrResult { data } => {
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                if data.ocr_txt.is_empty() {
                    let _ = show_notification(&app_clone, "No valid OCR result detected".into());
                    return;
                }
                let _ = app_clone.emit_to("main", "cgevent-ocr", data.ocr_txt);
            });
        }

        InboundMessage::SimulateKeyPress { data } => {
            let key = data.key;
            let _ = app.run_on_main_thread(|| simulate_key_press(key));
        }

        InboundMessage::RegisterShortcut { data } => {
            let shortcut = data.shortcut;
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                register_global_shortcut(&app_clone, &shortcut);
            });
        }
        InboundMessage::RegisterShortcuts { data } => {
            let shortcuts = data.shortcuts;
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                info!("Registering multiple shortcuts: {:?}", shortcuts);
                for shortcut in shortcuts {
                    register_global_shortcut(&app_clone, &shortcut);
                }
            });
        }
        InboundMessage::UnregisterShortcut { data } => {
            let shortcut = data.shortcut;
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                unregister_global_shortcut(&app_clone, &shortcut);
            });
        }

        InboundMessage::CheckAccessibility => {
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                let granted = check_accessibility();
                if !granted {
                    let _ = show_permission_window(app_clone);
                }
            });
        }
        InboundMessage::CheckScreenRecording => {
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                let granted = check_screen_recording();
                if !granted {
                    let _ = show_permission_window(app_clone);
                }
            });
        }

        InboundMessage::ExitRequest => {
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                info!("Received exit request from WebSocket. Exiting application.");
                app_clone.exit(0);
            });
        }
    }
}

/// Helper that merges MPSC receivers into a single stream.
struct OutboundMerger {
    main: mpsc::Receiver<String>,
}

impl OutboundMerger {
    fn new(main: mpsc::Receiver<String>) -> Self {
        Self { main }
    }

    /// Receives the next available message from either channel.
    async fn recv(&mut self) -> Option<String> {
        tokio::select! {
            msg = self.main.recv() => msg,
        }
    }
}
