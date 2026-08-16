"""
iWin WebSocket client.
Extends base client with iWin-specific client ID management and message formatting.
"""
import typing
from libs.ws_clients.base_client import BaseWebSocketClient
from libs.log_config import logger


class IWinWsClient(BaseWebSocketClient):
    """WebSocket client for communicating with the iWin window manager."""

    def __init__(self, uri: str, message_handler: typing.Callable):
        super().__init__(uri, message_handler)
        self._client_id: str = ""

    def set_client_id(self, client_id: str) -> None:
        """Set the client ID assigned by the iWin server."""
        self._client_id = client_id
        logger.info(f"iWin client ID set: {client_id}")

    async def send(self, data: typing.Dict) -> None:
        """Send a message with client_id automatically injected."""
        if self.is_connected():
            if "data" not in data:
                data["data"] = {}
            data["data"]["client_id"] = self._client_id
            await self.send_json(data)
        else:
            logger.warning("Cannot send to iWin: connection not active")
