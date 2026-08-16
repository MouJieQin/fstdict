"""
CGEvent WebSocket client.
Manages event registration for global keyboard shortcuts and text selection monitoring.
"""
import typing
from libs.ws_clients.base_client import BaseWebSocketClient
from libs.log_config import logger


class CgEventWsClient(BaseWebSocketClient):
    """WebSocket client for communicating with the CGEvent server."""

    def __init__(self, uri: str, register_events: typing.List[str], message_handler: typing.Callable):
        super().__init__(uri, message_handler)
        self._register_events = register_events.copy()

    async def _on_connected(self) -> None:
        """Automatically register all configured events after connection."""
        for event in self._register_events:
            await self.send_register_request(event)

    def set_register_events_right_after_connection(self, events: typing.List[str]) -> None:
        """Update the list of events to auto-register on next connection."""
        self._register_events = events.copy()

    async def send_register_request(self, event: str) -> None:
        """Send an event registration request to the CGEvent server."""
        msg = {
            "type": "register_request",
            "data": {"event": event}
        }
        await self.send_json(msg)
        logger.info(f"Registered CGEvent: {event}")

    async def send_unregister_request(self, event: str) -> None:
        """Send an event unregistration request to the CGEvent server."""
        msg = {
            "type": "unregister_request",
            "data": {"event": event}
        }
        await self.send_json(msg)
        logger.info(f"Unregistered CGEvent: {event}")
