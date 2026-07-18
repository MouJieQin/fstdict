#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import sys
import os
import socket
from libs.websocket_client import WsClient
from libs.message_handler import MessageHandler
from libs.session_manager import SessionManager
from libs.common import Utils
from libs.log_config import logger
import signal
import time
import asyncio
import threading

from fastapi import (
    FastAPI,
    HTTPException,
    WebSocket,
    WebSocketDisconnect,
)
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse
from pydantic import BaseModel
from fastapi.middleware.cors import CORSMiddleware
import uvicorn
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
                Utils.fstd_engine.extract_if_exists(dict_name, "/" + file_key, data_path)
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
        await SessionManager.send_folder_config_to_session(session_id, connection_id)
        await SessionManager.send_search_history_to_session(session_id, connection_id)
        await SessionManager.send_system_config()
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

# ========== 前端静态文件应用（9595端口）==========
frontend_app = FastAPI(title="FST Dict Frontend")

# 静态资源目录（Vue build 产物）
STATIC_DIR = Utils.BASE_DIR / "static"

frontend_app.mount("/assets", StaticFiles(directory=STATIC_DIR / "assets"), name="assets")


@frontend_app.get("/{full_path:path}")
async def serve_spa(full_path: str):
    """SPA 路由 fallback 到 index.html"""
    file_path = STATIC_DIR / full_path
    if file_path.is_file():
        return FileResponse(file_path)
    return FileResponse(STATIC_DIR / "index.html")


# ================= 正确的信号处理 =================
def signal_handler(sig, frame):
    logger.info("🛑 Ctrl+C 退出，正在关闭所有连接...")
    # 同步调用停止重连
    Utils.iwin_ws_client.set_do_not_retry()
    logger.info("✅ 所有连接已关闭，程序退出")
    os._exit(0)  # 强制安全退出


signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)


# ========== 双端口启动 ==========
def is_port_available(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        try:
            s.bind(("127.0.0.1", port))
            return True
        except OSError:
            return False


def run_api_server():
    uvicorn.run(app, host="127.0.0.1", port=5959, reload=False,
                access_log=False, log_level="info")


def run_frontend_server():
    uvicorn.run(frontend_app, host="127.0.0.1", port=9595, reload=False,
                access_log=False, log_level="info")


def main():
    if getattr(sys, "frozen", False):
        # 启动前检查
        # if not is_port_available(5959):
        #     logger.error("❌ 端口 5959 已被占用，词典后端可能已经在运行")
        #     sys.exit(1)
        # if not is_port_available(9595):
        #     logger.error("❌ 端口 9595 已被占用，词典前端可能已经在运行")
        #     sys.exit(1)
        # 前端静态服务放子线程（daemon=True，主线程退出它自动结束）
        fe_thread = threading.Thread(target=run_frontend_server, daemon=True)
        fe_thread.start()

        logger.info("✅ Frontend:     http://127.0.0.1:9595")
        logger.info("✅ API server:   http://127.0.0.1:5959")

        # API 服务放主线程（阻塞运行，程序主循环在这里）
        run_api_server()
    else:
        # if not is_port_available(5959):
        #     logger.error("❌ 端口 5959 已被占用，词典后端可能已经在运行")
        #     sys.exit(1)

        run_api_server()


if __name__ == "__main__":
    main()
