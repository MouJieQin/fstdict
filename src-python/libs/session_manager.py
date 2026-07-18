import json
import time
from typing import Optional
from fastapi import WebSocket
from libs.log_config import logger
from libs.common import Utils


class SessionManager:
    """会话管理器，负责会话的创建、删除、配置更新等操作"""

    @staticmethod
    async def broadcast_all(message: str):
        """向所有会话的所有WebSocket连接广播消息"""
        for session_id in Utils.session_websockets:
            await SessionManager.broadcast_session(session_id, message)

    @staticmethod
    async def broadcast_session(session_id: int, message: str):
        """向特定会话的所有WebSocket连接广播消息"""
        session_id = int(session_id)
        if session_id not in Utils.session_websockets:
            return

        invalid_keys = []
        for key, websocket in Utils.session_websockets[session_id].items():
            try:
                await websocket.send_text(message)
            except Exception as e:
                logger.error(f"会话广播错误: {e}")
                invalid_keys.append(key)

        # 移除无效连接
        for key in invalid_keys:
            del Utils.session_websockets[session_id][key]

    @staticmethod
    async def send_msg_to_session_by_id(
        session_id: int, connection_id: int, message: str
    ):
        """向特定会话的WebSocket连接发送消息"""
        if (
            session_id not in Utils.session_websockets
            or connection_id not in Utils.session_websockets[session_id]
        ):
            return

        websocket = Utils.session_websockets[session_id][connection_id]
        try:
            await websocket.send_text(message)
        except Exception as e:
            logger.error(f"会话广播错误: {e}")
            del Utils.session_websockets[session_id][connection_id]

    @staticmethod
    async def send_dict_info_to_session(session_id: int, connection_id: int):
        """向特定会话的WebSocket连接发送词典信息"""
        msg = {
            "type": "dict_info",
            "data": Utils.DICT_INFO,
        }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def send_folder_config_to_session(session_id: int, connection_id: int):
        folder_info = Utils.db.get_all_folder_info()
        """向特定会话的WebSocket连接发送文件夹信息"""
        msg = {
            "type": "folder_config",
            "data": {"folders": {"folder_info": folder_info}},
        }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def send_system_config():
        """发送系统配置到SPA"""
        msg = {
            "type": "system_config",
            "data": {
                "system_config": Utils.CONFIG,
            },
        }
        print(msg)
        await SessionManager.broadcast_all(json.dumps(msg))

    @staticmethod
    async def send_favorite_words_to_session(session_id: int, connection_id: int):
        """向特定会话的WebSocket连接发送收藏的单词"""
        folder_id = Utils.db.get_default_folder_id(session_id)
        msg = {"type": "favorite_words", "data": {"words": []}}
        if folder_id and Utils.db.is_folder_exist(folder_id):
            words = Utils.db.get_folder_words(folder_id)
            msg = {
                "type": "favorite_words",
                "data": {"words": words},
            }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def send_search_history_to_session(session_id: int, connection_id: int):
        """向特定会话的WebSocket连接发送搜索历史"""
        words_history = Utils.db.get_search_history()
        msg = {"type": "search_history", "data": {"words": words_history}}
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def send_session_config_to_session(
        session_id: int, connection_id: int, is_right_after_connection: bool = False
    ):
        """向特定会话的WebSocket连接发送会话配置"""
        config = Utils.db.get_session_config(session_id)
        if config is None:
            return
        if "default_folder" not in config.keys():
            config["default_folder"] = {"id": None}
        msg = {
            "type": "session_config",
            "data": {
                "config": config,
                "is_right_after_connection": is_right_after_connection,
            },
        }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )
