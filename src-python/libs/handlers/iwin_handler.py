"""
Message handler for incoming iWin WebSocket messages.
"""
import json
from websockets.asyncio.client import ClientConnection

from libs.log_config import logger
from libs.common.utils import Utils
from libs.common.session_manager import SessionManager


class IWinMessageHandler:
    """Handles messages received from the iWin server."""

    @staticmethod
    async def handle(ws: ClientConnection, data: str):
        """Parse and route incoming iWin messages."""
        try:
            message = json.loads(data)
            msg_type = message["type"]

            if msg_type == "client_id":
                client_id = message["data"]["client_id"]
                Utils.iwin_ws_client.set_client_id(client_id)

            elif msg_type == "toggle_floating_pin":
                session_id = message["data"]["session_id"]
                msg = {
                    "type": "toggle_floating_pin",
                    "data": {"is_pinned": message["data"]["is_pinned"]},
                }
                await SessionManager.broadcast_session(session_id, json.dumps(msg))

            elif msg_type == "close_fixed_window":
                session_id = message["data"]["session_id"]
                await SessionManager.broadcast_session(session_id, data)

            else:
                logger.warning(f"Unknown iWin message type: {msg_type}")

        except Exception as e:
            logger.error(f"Error handling iWin message: {e}", exc_info=True)
