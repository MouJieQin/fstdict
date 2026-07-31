import asyncio
import websockets
import json
import typing
from websockets.exceptions import ConnectionClosed
from websockets.asyncio.client import ClientConnection

from libs.log_config import logger


class WsClient:
    def __init__(self, uri: str, message_handler):
        self.uri = uri
        self.ws: typing.Optional[ClientConnection] = None
        self.message_handler = message_handler
        self._client_id = ""
        self._retry_count = 0
        self._connected_task = None  # 保存连接任务
        self._do_not_retry = False

    def is_connected(self):
        # websockets >=11.0 uses 'open' property to check connection status
        return self.ws is not None

    # and getattr(self.ws, 'open', False)

    def set_do_not_retry(self):
        self._do_not_retry = True

    def set_client_id(self, client_id: str):
        self._client_id = client_id

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
                # 去掉 async with，改用手动管理，才能被外部 close() 打断
                self.ws = await websockets.connect(self.uri, ping_interval=30)
                logger.info(f"✅ 已连接 {self.uri} WS服务器: {self.uri}")
                self._retry_count = 0
                msg = {"type": "register_request",
                       "data": {
                           "event": "globalKeyboardShortCut"
                       }}
                await self.send(msg)
                msg["data"]["event"] = "handlerEventTextSelection"
                await self.send(msg)
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
            msg["data"]["client_id"] = self._client_id
            await self.ws.send(json.dumps(msg))  # type: ignore
            print(f"✅ 发给 {self.uri} WebSocket: {msg}")
        else:
            print(f"❌ 未连接 {self.uri}")
