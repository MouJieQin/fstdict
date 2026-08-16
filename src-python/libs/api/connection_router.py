"""
Router for managing external WebSocket connections (iWin and CGEvent).
"""
import asyncio
from fastapi import APIRouter

from libs.common.utils import Utils
from libs.log_config import logger

router = APIRouter()


@router.get("/api/connectiwin")
async def connect_iwin():
    """Establish connection to the iWin server in the background."""
    if Utils.iwin_ws_client.is_connected():
        return {"status": "connected"}

    asyncio.create_task(Utils.iwin_ws_client.connect())
    logger.info("Started iWin WebSocket connection task")
    return {"status": "connecting"}


@router.get("/api/connectcgevent")
async def connect_cgevent():
    """Establish connection to the CGEvent server in the background."""
    if Utils.cgevent_ws_client.is_connected():
        return {"status": "connected"}
    if Utils.cgevent_ws_client.is_connecting():
        return {"status": "connecting"}

    asyncio.create_task(Utils.cgevent_ws_client.connect())
    logger.info("Started CGEvent WebSocket connection task")
    return {"status": "connecting"}
