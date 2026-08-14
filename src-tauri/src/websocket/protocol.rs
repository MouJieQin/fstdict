use serde::{Deserialize};

/// Inbound message types from the CGEvent WebSocket server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum InboundMessage {
    #[serde(rename = "tauri_notification")]
    TauriNotification { data: NotificationData },

    #[serde(rename = "ocr_result")]
    OcrResult { data: OcrResultData },
}

#[derive(Debug, Deserialize)]
pub struct NotificationData {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct OcrResultData {
    pub ocr_txt: String,
}

/// Builds the initial connection handshake message.
pub fn build_connect_message() -> String {
    serde_json::json!({
        "type": "connect_cgevent_server",
        "data": {}
    })
    .to_string()
}