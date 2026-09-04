use serde::Deserialize;

/// Inbound message types from the Python WebSocket server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum InboundMessage {
    #[serde(rename = "tauri_notification")]
    TauriNotification { data: NotificationData },

    #[serde(rename = "ocr_result")]
    OcrResult { data: OcrResultData },

    #[serde(rename = "simulate_key_press")]
    SimulateKeyPress { data: SimulateKeyData },

    #[serde(rename = "register_shortcut")]
    RegisterShortcut { data: RegisterShortcutData },

    #[serde(rename = "register_shortcuts")]
    RegisterShortcuts { data: RegisterShortcutsData },

    #[serde(rename = "unregister_shortcut")]
    UnregisterShortcut { data: RegisterShortcutData },

    #[serde(rename = "check_accessibility")]
    CheckAccessibility,

    #[serde(rename = "check_screen_recording")]
    CheckScreenRecording,

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
pub struct SimulateKeyData {
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterShortcutsData {
    pub shortcuts: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterShortcutData {
    pub shortcut: String,
}

/// Builds the initial connection handshake message.
pub fn build_connect_message() -> String {
    serde_json::json!({
        "type": "connect_cgevent_server",
        "data": {}
    })
    .to_string()
}
