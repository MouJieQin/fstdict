"""
Router for all WebSocket endpoints.
"""
from fastapi import APIRouter, WebSocket, WebSocketDisconnect, HTTPException
import time
import asyncio
from libs.handlers.session_handler import SessionMessageHandler
from libs.handlers.main_handler import MainMessageHandler
from libs.handlers.helper_handler import HelperMessageHandler
from libs.common.session_manager import SessionManager
from libs.config.app_config import Utils
from libs.handlers.exit_handler import ExitHandler
from libs.log_config import logger

router = APIRouter()


async def delayed_exit_check(ws: WebSocket | None):
    await asyncio.sleep(1)
    if not ws:
        ExitHandler.clean_and_exit()


@router.websocket("/ws/fstdict/main")
async def fstdict_main_websocket(websocket: WebSocket):
    """WebSocket endpoint for the main application window."""
    await websocket.accept()
    try:
        Utils.fstdict_main_websocket = websocket
        await MainMessageHandler.register_shortcuts(websocket)
        while True:
            text = await websocket.receive_text()
            logger.debug(f"Main WebSocket received: {text}")
            await MainMessageHandler.handle_message(websocket, text)
    except WebSocketDisconnect:
        logger.info("Main WebSocket disconnected")
    except Exception as e:
        logger.error(f"Main WebSocket error: {e}", exc_info=True)
    finally:
        Utils.fstdict_main_websocket = None
        asyncio.create_task(delayed_exit_check(Utils.fstdict_main_websocket))


@router.websocket("/ws/fstdict/helper")
async def fstdict_helper_websocket(websocket: WebSocket):
    """WebSocket endpoint for the helper floating window."""
    await websocket.accept()
    try:
        Utils.fstdict_helper_websocket = websocket
        while True:
            text = await websocket.receive_text()
            logger.debug(f"Helper WebSocket received: {text}")
            await HelperMessageHandler.handle_message(websocket, text)
    except WebSocketDisconnect:
        logger.info("Helper WebSocket disconnected")
    except Exception as e:
        logger.error(f"Helper WebSocket error: {e}", exc_info=True)
    finally:
        Utils.fstdict_helper_websocket = None
        asyncio.create_task(delayed_exit_check(Utils.fstdict_helper_websocket))


@router.websocket("/ws/dictionary/session/{client_id}")
async def dictionary_session_websocket(websocket: WebSocket, client_id: str):
    """
    WebSocket endpoint for dictionary session clients.
    Each session can have multiple concurrent connections.
    """
    try:
        session_id = int(client_id)
    except ValueError:
        raise HTTPException(status_code=400, detail="Invalid session ID")

    await websocket.accept()
    connection_id = int(time.time() * 1000)

    # Register connection in session map
    if session_id not in Utils.session_websockets:
        Utils.session_websockets[session_id] = {}
    Utils.session_websockets[session_id][connection_id] = websocket

    try:
        # Send initial state to newly connected client
        await SessionManager.broadcast_all_system_config()
        await SessionManager.broadcast_all_dict_config()
        await SessionManager.broadcast_all_sessions_id_name()
        await SessionManager.send_session_config(session_id, connection_id, is_new_connection=True)
        await SessionManager.send_dict_info(session_id, connection_id)
        await SessionManager.send_folder_config(session_id, connection_id)
        await SessionManager.send_search_history(session_id, connection_id)

        # Message loop
        while True:
            text = await websocket.receive_text()
            logger.debug(f"Session {session_id} received: {text}")
            await SessionMessageHandler.handle_message(
                websocket, session_id, connection_id, text
            )

    except WebSocketDisconnect:
        logger.info(f"Session {session_id} connection {connection_id} disconnected")
    except Exception as e:
        logger.error(f"Session {session_id} WebSocket error: {e}", exc_info=True)
    finally:
        # Clean up connection from session map
        if session_id in Utils.session_websockets:
            if connection_id in Utils.session_websockets[session_id]:
                del Utils.session_websockets[session_id][connection_id]
