"""
Message handler for the main application WebSocket.
"""
import json
import asyncio
from fastapi import WebSocket

from libs.log_config import logger
from libs.common.utils import Utils
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
    async def handle_message(websocket: WebSocket, data: str):
        """Parse and route incoming main window messages."""
        try:
            message = json.loads(data)
            msg_type = message["type"]

            if msg_type == "double_copy":
                selected_text = message["data"]["text"]
                await MainMessageHandler._broadcast_text_selection(selected_text)

            elif msg_type == "shortcut_triggered":
                shortcut = message["data"]["shortcut"]
                await MainMessageHandler._handle_shortcut_triggered(websocket, shortcut)

            else:
                logger.warning(f"Unknown main message type: {msg_type}")

        except Exception as e:
            logger.error(f"Error handling main message: {e}", exc_info=True)

    @staticmethod
    async def _broadcast_text_selection(text: str):
        """Forward selected text to the helper window."""
        msg = {
            "type": "kHandlerTextSelection",
            "data": {"text_selected": text}
        }
        if Utils.fstdict_helper_websocket:
            await Utils.fstdict_helper_websocket.send_text(json.dumps(msg))

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
            await MainMessageHandler._toggle_selection_monitoring()
        elif shortcut_name == "screenshot_ocr":
            await MainMessageHandler._handle_ocr_request(websocket)
        else:
            logger.warning(f"Unhandled shortcut: {shortcut}")

    @staticmethod
    async def _toggle_selection_monitoring():
        """Toggle text selection monitoring on/off."""
        enabled = Utils.CONFIG["app"]["helper_selection"]["enabled"]
        enabled = not enabled
        Utils.CONFIG["app"]["helper_selection"]["enabled"] = enabled
        Utils.Config.syncConfig()

        if enabled:
            await Utils.cgevent_ws_client.send_register_request("kHandlerTextSelection")
            if "kHandlerTextSelection" in Utils.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION:
                Utils.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION.remove("kHandlerTextSelection")
            logger.info("Text selection monitoring enabled")
            notification = "Text selection monitoring enabled"
        else:
            await Utils.cgevent_ws_client.send_unregister_request("kHandlerTextSelection")
            if "kHandlerTextSelection" not in Utils.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION:
                Utils.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION.append("kHandlerTextSelection")
            logger.info("Text selection monitoring disabled")
            notification = "Text selection monitoring disabled"

        Utils.cgevent_ws_client.set_register_events_right_after_connection(
            Utils.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION
        )

        # Send notification to helper window
        tmsg = {
            "type": "tauri_notification",
            "data": {"message": notification}
        }
        if Utils.fstdict_helper_websocket:
            await Utils.fstdict_helper_websocket.send_text(json.dumps(tmsg))

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

        # On macOS, send result to helper window; otherwise send back to caller
        import sys
        if sys.platform == "darwin":
            if Utils.fstdict_helper_websocket:
                await Utils.fstdict_helper_websocket.send_text(json.dumps(msg))
        else:
            await websocket.send_text(json.dumps(msg))
