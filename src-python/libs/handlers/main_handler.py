"""
Message handler for the main application WebSocket.
"""
import json
import asyncio
from fastapi import WebSocket

from libs.log_config import logger
from libs.config.app_config import Utils
from libs.core.ocr_engine import ocr_engine


class MainMessageHandler:
    """Handles messages from the main application window WebSocket."""

    @staticmethod
    async def register_shortcuts(websocket: WebSocket):
        """Register global shortcuts based on the provided data."""
        try:
            sc_list = []
            for shortcut, sc_name in Utils.shortcut_map.items():
                sc_list.append(shortcut)
            msg = {
                "type": "register_shortcuts",
                "data": {"shortcuts": sc_list}
            }
            await websocket.send_text(json.dumps(msg))
        except Exception as e:
            logger.error(f"Error registering shortcuts: {e}", exc_info=True)

    @staticmethod
    async def toggle_selection_capture(websocket: WebSocket):
        """Toggle text selection capture."""
        enabled = Utils.CONFIG["app"]["helper_selection"]["enabled"]
        await MainMessageHandler._toggle_selection_capture(websocket, enabled)

    @staticmethod
    async def handle_message(websocket: WebSocket, data: str):
        """Parse and route incoming main window messages."""
        try:
            message = json.loads(data)
            msg_type = message["type"]

            if msg_type == "double_copy":
                selected_text = message["data"]["text"]
                await MainMessageHandler._broadcast_text_selection(websocket, selected_text)

            elif msg_type == "text_selection":
                selected_text = message["data"]["text_selected"]
                await MainMessageHandler._broadcast_text_selection(websocket, selected_text)

            elif msg_type == "shortcut_triggered":
                shortcut = message["data"]["shortcut"]
                await MainMessageHandler._handle_shortcut_triggered(websocket, shortcut)

            elif msg_type == "hide_helper_main_window":
                await MainMessageHandler._hide_helper_main_window()

            elif msg_type == "hide_helper_selection_window":
                await MainMessageHandler._hide_helper_selection_window()

            else:
                logger.warning(f"Unknown main message type: {msg_type}")

        except Exception as e:
            logger.error(f"Error handling main message: {e}", exc_info=True)

    @staticmethod
    async def _send_message_to_helper(websocket: WebSocket, message: dict):
        """Send a message to the helper window."""
        if Utils.fstdict_helper_websocket:
            # Send to helper window on macOS mode
            await Utils.fstdict_helper_websocket.send_text(json.dumps(message))
        else:
            # Send to main window on non-macOS mode(linux, windows, dev-non-macOS mode on macOS)
            await websocket.send_text(json.dumps(message))

    @staticmethod
    async def _broadcast_text_selection(websocket: WebSocket, text: str):
        """Forward selected text to the helper window."""
        msg = {
            "type": "text_selection",
            "data": {"text_selected": text}
        }
        await MainMessageHandler._send_message_to_helper(websocket, msg)

    @staticmethod
    async def _handle_shortcut_triggered(websocket: WebSocket, shortcut: str):
        """Handle a shortcut triggered event."""
        logger.info(f"Shortcut triggered: {shortcut}")

        if shortcut not in Utils.shortcut_map:
            logger.warning(f"Shortcut '{shortcut}' not found in shortcut map")
            return

        shortcut_name = Utils.shortcut_map[shortcut]
        # Handle specific shortcuts
        if shortcut_name == "toggle_selection":
            await websocket.send_text(json.dumps({"type": "check_accessibility", "data": {}}))
            await MainMessageHandler._toggle_selection_monitoring(websocket)
        elif shortcut_name == "screenshot_ocr":
            await websocket.send_text(json.dumps({"type": "check_screen_recording", "data": {}}))
            await MainMessageHandler._handle_ocr_request(websocket)
        else:
            logger.warning(f"Unhandled shortcut: {shortcut}")

    @staticmethod
    async def _toggle_selection_monitoring(websocket: WebSocket):
        """Toggle text selection monitoring on/off."""
        enabled = Utils.CONFIG["app"]["helper_selection"]["enabled"]
        enabled = not enabled
        Utils.CONFIG["app"]["helper_selection"]["enabled"] = enabled
        Utils.Config.syncConfig()

        if enabled:
            logger.info("Text selection monitoring enabled")
            notification = "Text selection monitoring enabled"
        else:
            logger.info("Text selection monitoring disabled")
            notification = "Text selection monitoring disabled"

        Utils.cgevent_ws_client.set_register_events_right_after_connection(
            Utils.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION
        )

        await MainMessageHandler._toggle_selection_capture(websocket, enabled)
        await MainMessageHandler._send_notification(websocket, notification)

    @staticmethod
    async def _send_notification(websocket: WebSocket, notification: str):
        """Send notification to helper window."""
        tmsg = {
            "type": "tauri_notification",
            "data": {"message": notification}
        }
        await MainMessageHandler._send_message_to_helper(websocket, tmsg)

    @staticmethod
    async def _toggle_selection_capture(websocket: WebSocket, enabled: bool):
        """Toggle text selection capture on/off."""
        msg = {
            "type": "toggle_selection_capture",
            "data": {"enabled": enabled}
        }
        await websocket.send_text(json.dumps(msg))

    @staticmethod
    async def _handle_ocr_request(websocket: WebSocket):
        """Process an OCR request. Runs OCR in thread pool to avoid blocking."""
        if ocr_engine.is_ocring():
            return

        # Run blocking OCR operation in thread pool
        ocr_result = await asyncio.to_thread(ocr_engine.ocr)
        logger.info(f"OCR result: {ocr_result}")

        msg = {
            "type": "ocr_result",
            "data": {"ocr_txt": ocr_result}
        }

        await MainMessageHandler._send_message_to_helper(websocket, msg)

    @staticmethod
    async def _try_send_macos_helper_message(msg: dict):
        """Try to send a message to the helper process on macOS mode."""
        if Utils.fstdict_helper_websocket:
            await Utils.fstdict_helper_websocket.send_text(json.dumps(msg))

    @staticmethod
    async def _hide_helper_main_window():
        """Hide the helper main window."""
        msg = {
            "type": "hide_helper_main_window",
            "data": {}
        }
        await MainMessageHandler._try_send_macos_helper_message(msg)

    @staticmethod
    async def _hide_helper_selection_window():
        """Hide the helper selection window."""
        msg = {
            "type": "hide_helper_selection_window",
            "data": {}
        }
        await MainMessageHandler._try_send_macos_helper_message(msg)
