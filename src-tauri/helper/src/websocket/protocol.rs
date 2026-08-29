use serde::Deserialize;

/// Inbound message types from the CGEvent WebSocket server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum InboundMessage {
    #[serde(rename = "tauri_notification")]
    TauriNotification { data: NotificationData },

    #[serde(rename = "ocr_result")]
    OcrResult { data: OcrResultData },

    #[serde(rename = "kHandlerTextSelection")]
    TextSelection { data: TextSelectionData },

    #[serde(rename = "kCGEventLeftMouseDown")]
    LeftMouseDown,

    #[serde(rename = "exit_request")]
    ExitRequest,
}

#[derive(Debug, Deserialize)]
pub struct NotificationData {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct OcrResultData {
    pub ocr_txt: String,
}

#[derive(Debug, Deserialize)]
pub struct TextSelectionData {
    pub text_selected: String,
}

/// Builds a registration or unregistration request for a mouse event.
pub fn build_event_request(request_type: &str, event: &str, window: &str) -> String {
    serde_json::json!({
        "type": request_type,
        "data": {
            "event": event,
            "window": window
        }
    })
    .to_string()
}

/// Builds the initial connection handshake message.
pub fn build_connect_message() -> String {
    serde_json::json!({
        "type": "connect_cgevent_server",
        "data": {}
    })
    .to_string()
}
