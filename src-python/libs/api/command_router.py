"""
Router for general command API endpoints.
"""
from fastapi import APIRouter
from pydantic import BaseModel

from libs.handlers.command_handler import CommandHandler
from libs.log_config import logger

router = APIRouter()


class CommandRequest(BaseModel):
    """Request model for command API."""
    type: str
    data: dict


@router.post("/api/command")
async def handle_command(request: CommandRequest):
    """Process a command request and return the result."""
    logger.info(f"Received command: {request.type}")
    logger.debug(f"Command data: {request.data}")
    result = await CommandHandler.handle(request.type, request.data)
    return result
