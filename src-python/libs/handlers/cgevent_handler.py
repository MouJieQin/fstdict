"""
Message handler for incoming CGEvent WebSocket messages.
"""
import json
from websockets.asyncio.client import ClientConnection

from libs.log_config import logger
from libs.common.utils import Utils


class CgEventHandler:
    """Handles messages received from the CGEvent server."""

    @staticmethod
    async def handle(ws: ClientConnection, data: str):
        """Parse and route incoming CGEvent messages."""
        try:
            message = json.loads(data)
            msg_type = message["type"]

            if msg_type == "CGEvent":
                await CgEventHandler._handle_cgevent(message["data"])
            else:
                logger.warning(f"Unknown CGEvent message type: {msg_type}")

        except Exception as e:
            logger.error(f"Error handling CGEvent message: {e}", exc_info=True)

    @staticmethod
    async def _handle_cgevent(event_data: dict):
        """Process a CGEvent notification."""
        event_type = event_data["type"]

        if event_type == "kHandlerTextSelection":
            selected_text = event_data["text_selected"]
            await CgEventHandler._broadcast_text_selection(selected_text)

        elif event_type == "kCGEventLeftMouseDown":
            await CgEventHandler._broadcast_mouse_down()

        else:
            logger.warning(f"Unknown CGEvent type: {event_type}")

    @staticmethod
    async def _broadcast_text_selection(text: str):
        """Forward text selection event to the helper window."""
        msg = {
            "type": "kHandlerTextSelection",
            "data": {"text_selected": text}
        }
        if Utils.fstdict_helper_websocket:
            await Utils.fstdict_helper_websocket.send_text(json.dumps(msg))
        logger.info(f"Text selected: {text}")

    @staticmethod
    async def _broadcast_mouse_down():
        """Forward mouse down event to the helper window."""
        msg = {"type": "kCGEventLeftMouseDown", "data": {}}
        if Utils.fstdict_helper_websocket:
            await Utils.fstdict_helper_websocket.send_text(json.dumps(msg))
