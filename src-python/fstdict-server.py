#!/usr/bin/env python3
# _*_coding:utf-8_*_

from libs.websocket_client import WsClient
from libs.message_handler import MessageHandler
from libs.session_manager import SessionManager
from libs.common import Utils
from libs.log_config import logger
import signal
import os
import time
import asyncio

from fastapi import (
    FastAPI,
    UploadFile,
    Form,
    File,
    Path,
    HTTPException,
    WebSocket,
    WebSocketDisconnect,
)
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from fastapi.middleware.cors import CORSMiddleware
import uvicorn
from starlette.responses import FileResponse
from pathlib import Path
from urllib.parse import unquote

os.chdir(os.path.dirname(__file__))


# 配置应用
app = FastAPI(
    title="Dictionary API Server", description="WebSocket-based dictionary API server"
)
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/api/download")
async def download(path: str):
    logger.info(f"original download path: {path}")
    path = unquote(path)
    logger.info(f"download path: {path}")
    path = path.replace("//", "/")
    file_path = "/".join([Utils.DICTIONARYS_PATH, path])
    if not os.path.isfile(file_path):
        try:
            dict_name, _, file_key = path.split("/", maxsplit=2)
            data_path = "/".join([Utils.DICTIONARYS_PATH, dict_name, "data"])
            Utils.fstd_engine.extract(dict_name, file_key, data_path)
            if not os.path.isfile(file_path):
                Utils.fstd_engine.extract(dict_name, "/" + file_key, data_path)
        except Exception as e:
            logger.error(f"Failed to parse download path: {e}", exc_info=False)
            raise HTTPException(status_code=400, detail="Is not a file or does not exist")

    if not os.path.isfile(file_path):
        raise HTTPException(status_code=400, detail="Is not a file or does not exist")

    fr = FileResponse(
        path=file_path,
        filename=Path(file_path).name,
    )
    return fr


# ==============================================
# WebSocket Client 连接 iWin 服务器
# ==============================================
# 全局单例（整个程序共用一个连接）
Utils.iwin_ws_client = WsClient(
    "ws://localhost:9999/ws/fstdict", MessageHandler.handle_iwin_message
)


@app.get("/api/connectiwin")
async def connectiwin():
    if Utils.iwin_ws_client.is_connected():
        return {"status": "connected"}
    # 🔥 关键：用 create_task 后台启动，不阻塞接口
    asyncio.create_task(Utils.iwin_ws_client.connect())
    return {"status": "connecting"}


class CommandRequest(BaseModel):
    type: str
    data: dict


@app.post("/api/command")
async def command_command(request: CommandRequest):
    logger.info(f"command command: {request.type}")
    logger.info(f"command data: {request.data}")
    res = await MessageHandler.handle_command_message(request.type, request.data)
    return res


@app.websocket("/ws/dictionary/session/{clientID}")
async def dictionary_session_websocket_endpoint(websocket: WebSocket, clientID: str):
    """Dictionary Session WebSocket端点"""
    try:
        session_id = int(clientID)
    except ValueError:
        raise HTTPException(status_code=400, detail="Invalid session ID")
    await websocket.accept()
    connection_id = int(time.time() * 1000)
    if session_id not in Utils.session_websockets:
        Utils.session_websockets[session_id] = {}
    Utils.session_websockets[session_id][connection_id] = websocket

    try:
        await SessionManager.send_session_config_to_session(
            session_id, connection_id, is_right_after_connection=True
        )
        await SessionManager.send_dict_info_to_session(session_id, connection_id)
        await SessionManager.send_system_config_to_session(session_id, connection_id)
        await SessionManager.send_search_history_to_session(session_id, connection_id)
        # await SessionManager.send_favorite_words_to_session(session_id, connection_id)
        while True:
            text = await websocket.receive_text()
            logger.info(f"session {session_id} WebSocket收到消息: {text}")
            await MessageHandler.handle_session_message(
                websocket, session_id, connection_id, text
            )

    except WebSocketDisconnect:
        logger.info(f"session {session_id} WebSocket断开连接")
    except Exception as e:
        logger.error(f"session {session_id} WebSocket错误: {e}", exc_info=True)
    finally:
        if connection_id in Utils.session_websockets[session_id]:
            del Utils.session_websockets[session_id][connection_id]


# ================= 正确的信号处理 =================
def signal_handler(sig, frame):
    logger.info("🛑 Ctrl+C 退出，正在关闭所有连接...")
    # 同步调用停止重连
    Utils.iwin_ws_client.set_do_not_retry()
    logger.info("✅ 所有连接已关闭，程序退出")
    os._exit(0)  # 强制安全退出


signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)

# ==============================================
# 启动应用
# ==============================================
if __name__ == "__main__":

    # 启动服务器
    uvicorn.run(
        app,
        host="127.0.0.1",
        port=5959,
        reload=False,
        access_log=False,
        # log_level="critical",
    )
