"""
Base class for auto-reconnecting WebSocket clients.
Implements common connection lifecycle, retry logic and message sending.
"""
import asyncio
import json
import typing
from websockets.exceptions import ConnectionClosed
from websockets.asyncio.client import ClientConnection
import websockets

from libs.log_config import logger


class BaseWebSocketClient:
    def __init__(self, uri: str, message_handler: typing.Callable):
        self.uri = uri
        self.ws: typing.Optional[ClientConnection] = None
        self.message_handler = message_handler
        self._retry_count = 0
        self._do_not_retry = False
        self._max_retries = 5
        self._retry_delay = 5  # seconds

    def is_connected(self) -> bool:
        """Check if the WebSocket connection is active and open."""
        return self.ws is not None

    def is_connecting(self) -> bool:
        """Check if connection attempts are in progress."""
        return self._retry_count != 0

    def set_do_not_retry(self) -> None:
        """Disable auto-reconnect (used during graceful shutdown)."""
        self._do_not_retry = True

    async def close(self) -> None:
        """Close the connection immediately."""
        if self.ws is not None:
            await self.ws.close()
            self.ws = None
            logger.info(f"Closed WebSocket connection to {self.uri}")

    async def connect(self) -> None:
        """Main connection loop with automatic reconnection."""
        while True:
            if self._do_not_retry:
                return

            self._retry_count += 1
            if self._retry_count > self._max_retries:
                logger.error(f"Max connection attempts ({self._max_retries}) reached for {self.uri}")
                self._retry_count = 0
                return

            try:
                self.ws = await websockets.connect(self.uri, ping_interval=30)
                logger.info(f"Connected to WebSocket server: {self.uri}")
                self._retry_count = 0

                await self._on_connected()

                # Message listening loop
                while True:
                    try:
                        msg = await self.ws.recv()
                        logger.debug(f"Received from {self.uri}: {msg}")
                        await self.message_handler(self.ws, str(msg))
                    except ConnectionClosed:
                        logger.warning(f"WebSocket connection closed: {self.uri}")
                        break

            except Exception as e:
                await self.close()
                logger.error(f"WebSocket error for {self.uri}: {e}. Retrying in {self._retry_delay}s...")

            self.ws = None
            await asyncio.sleep(self._retry_delay)

    async def _on_connected(self) -> None:
        """Hook for post-connection setup. Override in subclasses."""
        pass

    async def send_json(self, data: typing.Dict) -> None:
        """Send a dictionary as JSON to the server."""
        if self.is_connected():
            await self.ws.send(json.dumps(data))  # type: ignore
            logger.debug(f"Sent to {self.uri}: {data}")
        else:
            logger.warning(f"Cannot send message: not connected to {self.uri}")
