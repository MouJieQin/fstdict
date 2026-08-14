use std::time::Duration;

use fstdict_common::window::notification::show_notification;
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::tungstenite::Utf8Bytes;

use super::protocol::{build_connect_message, build_event_request, InboundMessage};

/// Reconnection delay after WebSocket disconnect (milliseconds).
const RECONNECT_DELAY_MS: u64 = 2000;

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
    // Create merger ONCE outside the reconnection loop to avoid moving receivers twice
    loop {
        info!("Connecting to CGEvent WebSocket: {}", ws_url);

        match connect_async(ws_url).await {
            Ok((ws_stream, _)) => {
                info!("CGEvent WebSocket connected");
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
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
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

async fn dispatch_message<S>(app: &AppHandle, event: InboundMessage, write: &mut S)
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
