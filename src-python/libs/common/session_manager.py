"""
WebSocket session management: broadcasting and targeted messaging.
"""
import json
from libs.log_config import logger
from libs.common.utils import Utils


class SessionManager:
    """Manages WebSocket session connections and message distribution."""

    @staticmethod
    async def broadcast_all(message: str) -> None:
        """Broadcast a message to all sessions and all connections."""
        for session_id in list(Utils.session_websockets.keys()):
            await SessionManager.broadcast_session(session_id, message)

    @staticmethod
    async def broadcast_session(session_id: int, message: str) -> None:
        """Broadcast a message to all connections within a specific session."""
        session_id = int(session_id)
        if session_id not in Utils.session_websockets:
            return

        invalid_connections = []
        session_connections = Utils.session_websockets[session_id]

        for conn_id, websocket in session_connections.items():
            try:
                await websocket.send_text(message)
            except Exception as e:
                logger.error(f"Failed to broadcast to session {session_id}: {e}")
                invalid_connections.append(conn_id)

        # Clean up dead connections
        for conn_id in invalid_connections:
            del session_connections[conn_id]

    @staticmethod
    async def send_to_connection(
        session_id: int, connection_id: int, message: str
    ) -> None:
        """Send a message to a specific connection within a session."""
        if session_id not in Utils.session_websockets:
            return
        if connection_id not in Utils.session_websockets[session_id]:
            return

        websocket = Utils.session_websockets[session_id][connection_id]
        try:
            await websocket.send_text(message)
        except Exception as e:
            logger.error(f"Failed to send message to connection {connection_id}: {e}")
            del Utils.session_websockets[session_id][connection_id]

    # --- Initial state senders ---

    @staticmethod
    async def send_dict_info(session_id: int, connection_id: int) -> None:
        msg = {"type": "dict_info", "data": Utils.DICT_INFO}
        await SessionManager.send_to_connection(session_id, connection_id, json.dumps(msg))

    @staticmethod
    async def broadcast_all_dict_info() -> None:
        msg = {"type": "dict_info", "data": Utils.DICT_INFO}
        await SessionManager.broadcast_all(json.dumps(msg))

    @staticmethod
    async def send_folder_config(session_id: int, connection_id: int) -> None:
        folder_info = Utils.db.get_all_folder_info()
        msg = {"type": "folder_config", "data": {"folders": {"folder_info": folder_info}}}
        await SessionManager.send_to_connection(session_id, connection_id, json.dumps(msg))

    @staticmethod
    async def broadcast_all_system_config() -> None:
        msg = {"type": "system_config", "data": {"system_config": Utils.CONFIG}}
        await SessionManager.broadcast_all(json.dumps(msg))

    @staticmethod
    async def broadcast_all_dict_config() -> None:
        msg = {"type": "dict_config", "data": {"dict_config": Utils.DICT_CONFIG}}
        await SessionManager.broadcast_all(json.dumps(msg))

    @staticmethod
    async def broadcast_all_sessions_id_name() -> None:
        sessions = Utils.db.get_all_sessions()
        session_list = []
        for session in sessions:
            name = session["config"].get("name", str(session["id"]))
            session_list.append({"id": session["id"], "name": name})

        msg = {"type": "sessions_name_id", "data": {"sessions_name_id": session_list}}
        await SessionManager.broadcast_all(json.dumps(msg))

    @staticmethod
    async def send_session_config(
        session_id: int, connection_id: int, is_new_connection: bool = False
    ) -> None:
        config = Utils.db.get_session_config(session_id)
        if config is None:
            return

        if "default_folder" not in config:
            config["default_folder"] = {"id": None}

        msg = {
            "type": "session_config",
            "data": {
                "config": config,
                "is_right_after_connection": is_new_connection,
            },
        }
        await SessionManager.send_to_connection(session_id, connection_id, json.dumps(msg))

    @staticmethod
    async def send_search_history(session_id: int, connection_id: int) -> None:
        history = Utils.db.get_search_history()
        msg = {"type": "search_history", "data": {"words": history}}
        await SessionManager.send_to_connection(session_id, connection_id, json.dumps(msg))
