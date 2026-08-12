use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::tungstenite::Utf8Bytes;

use super::protocol::{build_connect_message, build_event_request, InboundMessage};
use crate::window::commands::{
    hide_window_if_unpinned_and_outside, show_main_panel, show_selection_panel,
};
use crate::window::notification::show_notification;

/// Reconnection delay after WebSocket disconnect (milliseconds).
const RECONNECT_DELAY_MS: u64 = 2000;

/// Starts the WebSocket client loop with automatic reconnection.
///
/// The client maintains two outbound channels:
/// - `outbound_main_rx`: messages originating from the main panel
/// - `outbound_selection_rx`: messages originating from the selection panel
pub async fn start_cgevent_ws_client(
    ws_url: &str,
    app_handle: AppHandle,
    outbound_main_rx: mpsc::Receiver<String>,
    outbound_selection_rx: mpsc::Receiver<String>,
) {
    // Create merger ONCE outside the reconnection loop to avoid moving receivers twice
    let mut outbound_merged = OutboundMerger::new(outbound_main_rx, outbound_selection_rx);

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

                if !crate::window::positioning::is_cursor_over_window(&app_clone, "helper-main") {
                    let _ = show_main_panel(&app_clone);
                }

                let _ = app_clone.emit_to("helper-main", "cgevent-ocr", data.ocr_txt);
            });

            // Register mouse-down listener for the main panel
            let req =
                build_event_request("register_request", "kCGEventLeftMouseDown", "helper-main");
            let _ = write.send(WsMessage::Text(Utf8Bytes::from(req))).await;
        }

        InboundMessage::TextSelection { data } => {
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                if crate::window::positioning::is_cursor_over_window(
                    &app_clone,
                    "selection-float-search",
                ) {
                    return;
                }

                let _ = show_selection_panel(&app_clone);
                let _ = app_clone.emit_to(
                    "selection-float-search",
                    "cgevent-select",
                    data.text_selected,
                );
            });

            // Register mouse-down listener for the selection panel
            let req = build_event_request(
                "register_request",
                "kCGEventLeftMouseDown",
                "selection-float-search",
            );
            let _ = write.send(WsMessage::Text(Utf8Bytes::from(req))).await;
        }

        InboundMessage::LeftMouseDown => {
            handle_mouse_down(app, write).await;
        }
    }
}

async fn handle_mouse_down<S>(app: &AppHandle, write: &mut S)
where
    S: SinkExt<WsMessage> + Unpin,
    S::Error: std::fmt::Display,
{
    // Check selection panel
    let app_clone = app.clone();
    let (sel_tx, sel_rx) = tokio::sync::oneshot::channel::<bool>();
    let _ = app.run_on_main_thread(move || {
        let hidden = hide_window_if_unpinned_and_outside(&app_clone, "selection-float-search");
        let _ = sel_tx.send(hidden);
    });

    if let Ok(true) = sel_rx.await {
        let req = build_event_request(
            "unregister_request",
            "kCGEventLeftMouseDown",
            "selection-float-search",
        );
        let _ = write.send(WsMessage::Text(Utf8Bytes::from(req))).await;
    }

    // Check main panel
    let app_main = app.clone();
    let (main_tx, main_rx) = tokio::sync::oneshot::channel::<bool>();
    let _ = app.run_on_main_thread(move || {
        let hidden = hide_window_if_unpinned_and_outside(&app_main, "helper-main");
        let _ = main_tx.send(hidden);
    });

    if let Ok(true) = main_rx.await {
        let req = build_event_request("unregister_request", "kCGEventLeftMouseDown", "helper-main");
        let _ = write.send(WsMessage::Text(Utf8Bytes::from(req))).await;
    }
}

/// Helper that merges two MPSC receivers into a single stream.
struct OutboundMerger {
    main: mpsc::Receiver<String>,
    selection: mpsc::Receiver<String>,
}

impl OutboundMerger {
    fn new(main: mpsc::Receiver<String>, selection: mpsc::Receiver<String>) -> Self {
        Self { main, selection }
    }

    /// Receives the next available message from either channel.
    async fn recv(&mut self) -> Option<String> {
        tokio::select! {
            msg = self.main.recv() => msg,
            msg = self.selection.recv() => msg,
        }
    }
}
