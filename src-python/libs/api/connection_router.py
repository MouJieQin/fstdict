"""
Router for managing external WebSocket connections (CGEvent).
"""
import asyncio
from fastapi import APIRouter

from libs.config.app_config import Utils
from libs.log_config import logger

router = APIRouter()


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
