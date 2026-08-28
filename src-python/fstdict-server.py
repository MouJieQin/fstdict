#!/usr/bin/env python3
"""
FstDict API Server Entrypoint.
Starts the API server on port 5959 and frontend static server on port 9595.
"""
import os
import sys
import socket
import signal
import threading

import uvicorn
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse

from libs.log_config import logger
from libs.config.paths import BASE_DIR
from libs.api.download_router import router as download_router
from libs.api.connection_router import router as connection_router
from libs.api.command_router import router as command_router
from libs.api.ws_router import router as ws_router
from libs.ws_clients.iwin_client import IWinWsClient
from libs.ws_clients.cgevent_client import CgEventWsClient
from libs.handlers.cgevent_handler import CgEventHandler
from libs.handlers.iwin_handler import IWinMessageHandler
from libs.handlers.exit_handler import ExitHandler

from libs.common.utils import Utils

# Change working directory to script location
os.chdir(os.path.dirname(__file__))

# ---------------------------------------------------------------------------
# Main API Application
# ---------------------------------------------------------------------------
app = FastAPI(
    title="FstDict API Server",
    description="WebSocket-based dictionary API server"
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Register route modules
app.include_router(download_router)
app.include_router(connection_router)
app.include_router(command_router)
app.include_router(ws_router)

# ---------------------------------------------------------------------------
# Frontend Static File Server (SPA)
# ---------------------------------------------------------------------------
frontend_app = FastAPI(title="FstDict Frontend")
STATIC_DIR = BASE_DIR / "static"
ASSETS_DIR = STATIC_DIR / "assets"

if ASSETS_DIR.exists():
    frontend_app.mount("/assets", StaticFiles(directory=ASSETS_DIR), name="assets")


@frontend_app.get("/{full_path:path}")
async def serve_spa(full_path: str):
    """SPA routing fallback: serve index.html for all client-side routes."""
    file_path = STATIC_DIR / full_path
    if file_path.is_file():
        return FileResponse(file_path)
    return FileResponse(STATIC_DIR / "index.html")

# ---------------------------------------------------------------------------
# WebSocket Client
# ---------------------------------------------------------------------------
Utils.iwin_ws_client = IWinWsClient(
    "ws://127.0.0.1:9999/ws/fstdict", IWinMessageHandler.handle
)

Utils.cgevent_ws_client = CgEventWsClient(
    "ws://127.0.0.1:5995", Utils.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION,
    CgEventHandler.handle
)

# ---------------------------------------------------------------------------
# Graceful Shutdown Signal Handler
# ---------------------------------------------------------------------------


def _signal_handler(sig, frame):
    logger.info("Received shutdown signal, closing all connections...")
    ExitHandler.clean_and_exit()


signal.signal(signal.SIGINT, _signal_handler)
signal.signal(signal.SIGTERM, _signal_handler)


# ---------------------------------------------------------------------------
# Server Launchers
# ---------------------------------------------------------------------------


def run_api_server():
    """Run the main API server (blocking)."""
    uvicorn.run(
        app,
        host="127.0.0.1",
        port=5959,
        reload=False,
        access_log=False,
        log_level="info"
    )


def run_frontend_server():
    """Run the frontend static file server (blocking)."""
    uvicorn.run(
        frontend_app,
        host="127.0.0.1",
        port=9595,
        reload=False,
        access_log=False,
        log_level="info"
    )


def main():
    if getattr(sys, "frozen", False):
        # Run frontend server in background daemon thread
        fe_thread = threading.Thread(target=run_frontend_server, daemon=True)
        fe_thread.start()

        logger.info("Frontend server:    http://127.0.0.1:9595")
        logger.info("API server:         http://127.0.0.1:5959")

        # Run API server on main thread
        run_api_server()
    else:
        run_api_server()


if __name__ == "__main__":
    main()
