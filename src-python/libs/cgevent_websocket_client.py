import asyncio
import websockets
import json
import typing
from websockets.exceptions import ConnectionClosed
from websockets.asyncio.client import ClientConnection

from libs.log_config import logger


class CgeventWsClient:
    def __init__(self, uri: str, register_events: typing.List[str], message_handler):
        self.uri = uri
        self.ws: typing.Optional[ClientConnection] = None
        self.message_handler = message_handler
        self._retry_count = 0
        self._do_not_retry = False
        self._register_events = register_events

    def is_connected(self):
        # websockets >=11.0 uses 'open' property to check connection status
        return self.ws is not None

    def set_do_not_retry(self):
        self._do_not_retry = True

    def set_register_events_right_after_connection(self, events: typing.List[str]):
        self.register_events = events

    async def close(self):
        """外部调用：立刻关闭连接并退出循环"""
        if self.ws is not None:
            await self.ws.close()
            self.ws = None
            logger.info(f"✅ 已关闭 {self.uri} WebSocket 连接")

    async def connect(self):
        """自动重连的 WebSocket 客户端"""
        while True:
            if self._do_not_retry:
                return
            self._retry_count += 1
            if self._retry_count > 5:
                logger.error("❌连接尝试次数超过最大5次")
                self._retry_count = 0
                return

            try:
                self.ws = await websockets.connect(self.uri, ping_interval=30)
                logger.info(f"✅ 已连接 {self.uri} WS服务器: {self.uri}")
                self._retry_count = 0
                for event in self._register_events:
                    await self.send_register_request(event)
                # 监听消息
                while True:
                    try:
                        msg = await self.ws.recv()
                        logger.info(f"\n📩 从 {self.uri} WebSocket 收到: {msg}")
                        await self.message_handler(self.ws, str(msg))

                    except ConnectionClosed:
                        logger.warning(f"🔌 {self.uri} WebSocket 连接断开")
                        break

            except Exception as e:
                await self.close()  # 确保连接被正确关闭
                logger.error(f"❌ {self.uri} WS 错误: {e}，5秒后重连")

            # 退出连接，准备重连
            self.ws = None
            logger.info("等待 5 秒后重连...")
            await asyncio.sleep(5)

    async def send(self, msg: typing.Dict):
        """发送消息到"""
        if self.is_connected():
            await self.ws.send(json.dumps(msg))  # type: ignore
            print(f"✅ 发给 {self.uri} WebSocket: {msg}")
        else:
            print(f"❌ 未连接 {self.uri}")

    async def send_register_request(self, event: str):
        """发送注册请求"""
        msg = {
            "type": "register_request",
            "data": {
                "event": event
            }
        }
        await self.send(msg)

    async def send_unregister_request(self, event: str):
        """发送注销请求"""
        msg = {
            "type": "unregister_request",
            "data": {
                "event": event
            }
        }
        await self.send(msg)
