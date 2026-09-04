"""
Message handler for the helper floating window WebSocket.
"""
import json
import threading
import urllib.request
from fastapi import WebSocket

from libs.log_config import logger
from libs.config.app_config import Utils


class HelperMessageHandler:
    """Handles messages from the helper floating window WebSocket."""

    @staticmethod
    async def handle_message(websocket: WebSocket, data: str):
        """Parse and route incoming helper window messages."""
        try:
            message = json.loads(data)
            msg_type = message["type"]

            if msg_type == "register_request":
                await HelperMessageHandler._handle_register_request(message["data"])
            elif msg_type == "unregister_request":
                await HelperMessageHandler._handle_unregister_request(message["data"])
            elif msg_type == "connect_cgevent_server":
                HelperMessageHandler._trigger_cgevent_connection()
            else:
                logger.warning(f"Unknown helper message type: {msg_type}")

        except Exception as e:
            logger.error(f"Error handling helper message: {e}", exc_info=True)

    @staticmethod
    async def _handle_register_request(data: dict):
        """Register a window for a specific CGEvent type."""
        event_type = data["event"]
        window = data["window"]

        if event_type not in Utils.cgevent_register_map:
            Utils.cgevent_register_map[event_type] = []

        # If this is the first registration for this event, register with server
        if not Utils.cgevent_register_map[event_type]:
            await Utils.cgevent_ws_client.send_register_request(event_type)

        if window not in Utils.cgevent_register_map[event_type]:
            Utils.cgevent_register_map[event_type].append(window)

    @staticmethod
    async def _handle_unregister_request(data: dict):
        """Unregister a window from a specific CGEvent type."""
        event_type = data["event"]
        window = data["window"]

        if event_type in Utils.cgevent_register_map:
            if window in Utils.cgevent_register_map[event_type]:
                Utils.cgevent_register_map[event_type].remove(window)

            # If no more registrations for this event, unregister from server
            if not Utils.cgevent_register_map[event_type]:
                await Utils.cgevent_ws_client.send_unregister_request(event_type)

    @staticmethod
    def _trigger_cgevent_connection():
        """Trigger CGEvent server connection via HTTP API (runs in background thread)."""
        def connect_task():
            try:
                url = "http://127.0.0.1:5959/api/connectcgevent"
                with urllib.request.urlopen(url) as resp:
                    logger.info(json.loads(resp.read()))
            except Exception as e:
                logger.error(f"Failed to trigger CGEvent connection: {e}")

        thread = threading.Thread(target=connect_task, daemon=True)
        thread.start()
