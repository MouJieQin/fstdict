import json
import time
import threading
import asyncio
from typing import Dict, List, Optional, Any
from websockets.asyncio.client import ClientConnection
from fastapi import WebSocket


from libs.log_config import logger
from libs.common import Utils
from libs.session_manager import SessionManager
from libs.mdict_searcher import MdictSearcher
from libs.websocket_client import WsClient
from libs.anki.anki_manager import AnkiManager


mdict_searcher = MdictSearcher()
anki_manager = AnkiManager()


class MessageHandler:
    """消息处理器，处理不同类型的WebSocket消息"""

    @staticmethod
    async def handle_command_message(command_type: str, data: dict) -> Dict:
        """处理命令消息"""
        if command_type == "lookup_keyword_request":
            session_id = data["session_id"]
            await SessionManager.broadcast_session(
                session_id, json.dumps({"type": "lookup_keyword_request", "data": data})
            )
            return {"success": True}
        elif command_type == "acquire_words_from_folder":
            return MessageHandler._handle_acquire_words_from_folder(data)
        elif command_type == "favorite_words_to_folder":
            folder_name = data["folder_name"]
            folder_id = Utils.db.get_folder_id_by_name(folder_name)
            if folder_id is None:
                logger.error(f"文件夹 {folder_name} 不存在")
                return {"success": False, "message": f"文件夹 {folder_name} 不存在"}
            words = data["words"]
            for word in words:
                if not Utils.db.is_word_favorited(word, folder_id):
                    Utils.db.favorite_word(word, folder_id)
            return {"success": True}
        elif command_type == "check_running":
            return {"success": True}
        else:
            logger.warning(f"未知的命令类型: {command_type}")
            return {"success": False}

    @staticmethod
    def _handle_acquire_words_from_folder(data: dict) -> Dict:
        """获取收藏夹下的所有单词（Anki 格式）"""
        folder_name = data["folder_name"]
        words = Utils.db.get_folder_words_by_name(folder_name)
        for word in words:
            word["note"] = Utils.db.get_word_note(word["word"])
            word["definition"] = Utils.dict_db.query(word["word"])
        return {"success": True, "data": {"words": words}}

    @staticmethod
    async def handle_iwin_message(ws: ClientConnection, data: str):
        """处理iWin消息"""
        try:
            message = json.loads(data)
            command_type = message["type"]
            if command_type == "client_id":
                client_id = message["data"]["client_id"]
                Utils.iwin_ws_client.set_client_id(client_id)
                logger.info(f"设置 iWin 客户端 ID: {client_id}")
            elif command_type == "toggle_floating_pin":
                session_id = message["data"]["session_id"]
                # connection_id = message["data"]["connection_id"]
                msg = {
                    "type": "toggle_floating_pin",
                    "data": {"is_pinned": message["data"]["is_pinned"]},
                }
                await SessionManager.broadcast_session(session_id, json.dumps(msg))
            elif command_type == "close_fixed_window":
                session_id = message["data"]["session_id"]
                await SessionManager.broadcast_session(session_id, data)
            else:
                logger.warning(f"未知的iWin命令类型: {command_type}")
        except Exception as e:
            logger.error(f"处理iWin消息时出错: {e}", exc_info=True)

    @staticmethod
    async def handle_session_message(
        websocket: WebSocket, session_id: int, connection_id: int, message_text: str
    ):
        """处理会话WebSocket消息"""
        try:
            message = json.loads(message_text)
            message_type = message["type"]

            handlers = {
                "toggle_floating_pin": MessageHandler._handle_toggle_floating_pin,
                "keyword_options_search": MessageHandler._handle_keyword_options_search,
                "lookup_keyword": MessageHandler._handle_lookup,
                "session_config": MessageHandler._handle_session_config,
                "create_folder": MessageHandler._handle_create_folder,
                "delete_folder": MessageHandler._handle_delete_folder,
                "update_to_anki": MessageHandler._handle_update_to_anki,
                "cancel_anki_update": MessageHandler._handle_cancel_anki_update,
                "update_folder": MessageHandler._handle_update_folder,
                "system_config": MessageHandler._handle_system_config,
                "toggle_favor": MessageHandler._handle_toggle_favor,
                "save_word_note": MessageHandler._handle_save_word_note,
                "delete_word_note": MessageHandler._handle_delete_word_note,
                "lookup_keyword_request": MessageHandler._handle_lookup_keyword_request,
                "favorite_words_request": MessageHandler._handle_favorite_words_request,
                "search_history_request": MessageHandler._handle_search_history_request,
            }

            if message_type in handlers:
                await handlers[message_type](
                    websocket, session_id, connection_id, message
                )
            else:
                logger.warning(f"未知的会话消息类型: {message_type}")

        except Exception as e:
            logger.error(f"处理会话消息时出错: {e}", exc_info=True)

    @staticmethod
    async def _handle_toggle_floating_pin(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        msg = {
            "type": "toggle_floating_pin",
            "data": {
                "session_id": session_id,
                "connection_id": connection_id,
                "is_pinned": message["data"]["is_pinned"],
            },
        }
        await Utils.iwin_ws_client.send(msg)

    @staticmethod
    async def _handle_keyword_options_search(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        keyword = message["data"]["keyword"]
        search_method = message["data"]["search_method"]
        options = mdict_searcher.keyword_options_search(
            keyword, search_method, dict_names=message["data"]["dict_settings"]
        )
        msg = {
            "type": "keyword_options_search",
            "data": {
                "keyword": keyword,
                "options": options,
            },
        }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def _handle_lookup(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        keyword = message["data"]["keyword"]
        folder_id = message["data"]["folder_id"]
        left_history = message["data"]["left_history"]
        results = mdict_searcher.mdx_lookup(
            keyword, dict_names=message["data"]["dict_settings"]
        )
        is_word_favorited = False
        if folder_id:
            is_word_favorited = Utils.db.is_word_favorited(keyword, folder_id)
        note = Utils.db.get_word_note(keyword)
        if left_history and (results or note):
            Utils.db.add_search_history(keyword)
        msg = {
            "type": "lookup_keyword",
            "data": {
                "keyword": keyword,
                "result": results,
                "note": note,
                "is_word_favorited": is_word_favorited,
                "left_history": left_history,
            },
        }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def _handle_session_config(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        logger.info(f"收到会话配置: {message['data']['config']}")
        Utils.db.update_session_config(session_id, message["data"]["config"])
        await SessionManager.broadcast_session(session_id, json.dumps(message))

    @staticmethod
    async def _handle_create_folder(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        folder_name = message["data"]["folder_name"]
        folder_description = message["data"]["folder_description"]
        Utils.db.create_folder(folder_name, folder_description)
        await SessionManager.send_system_config_to_session(session_id, connection_id)

    @staticmethod
    async def _handle_delete_folder(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        folder_id = message["data"]["folder_id"]
        Utils.db.delete_folder(folder_id)
        await SessionManager.send_system_config_to_session(session_id, connection_id)

    @staticmethod
    async def _handle_update_to_anki(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        anki_manager.set_cancel_flag(False)
        deck_name = message["data"]["deck_name"]
        folder_id = message["data"]["folder_id"]
        async def send_progress(progress_msg: dict):
            msg = {
                "type": "anki_progress",
                "deck_name": deck_name,
                "data":progress_msg,
            }
            try:
                await SessionManager.send_msg_to_session_by_id(
                    session_id, connection_id, json.dumps(msg)
                )
                logger.info(f"发送进度消息到会话 {session_id}: {msg}")
            except Exception as e:
                logger.error(f"发送进度消息到会话 {session_id} 失败: {e}", exc_info=True)
        words = Utils.db.get_folder_words(folder_id)

        await anki_manager.update_words_to_anki(str(session_id), deck_name, words,send_progress)
        logger.info("@@@@@@@@@@@@@@@@@@@@@@@@exit _handle_update_to_anki")


    @staticmethod
    async def _handle_cancel_anki_update(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        logger.info(f"收到取消 Anki 更新请求: {message}")
        anki_manager.set_cancel_flag(True)

    @staticmethod
    async def _handle_update_folder(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        folder_id = message["data"]["folder_id"]
        folder_name = message["data"]["folder_name"]
        folder_description = message["data"]["folder_description"]
        Utils.db.rename_folder(folder_id, folder_name)
        Utils.db.update_folder_description(folder_id, folder_description)
        await SessionManager.send_system_config_to_session(session_id, connection_id)

    @staticmethod
    async def _handle_system_config(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        folder_info = Utils.db.get_all_folder_info()
        await SessionManager.send_system_config_to_session(session_id, connection_id)

    @staticmethod
    async def _handle_toggle_favor(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        keyword = message["data"]["keyword"]
        folder_id = message["data"]["folder_id"]
        is_word_favorited = not Utils.db.is_word_favorited(keyword, folder_id)
        if is_word_favorited:
            Utils.db.favorite_word(keyword, folder_id)
        else:
            Utils.db.unfavorite_word(keyword, folder_id)
        msg = {
            "type": "toggle_favor",
            "data": {
                "folder_id": folder_id,
                "keyword": keyword,
                "is_word_favorited": is_word_favorited,
            },
        }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def _handle_save_word_note(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        keyword = message["data"]["keyword"]
        note = message["data"]["note"]
        Utils.db.save_word_note(keyword, note)
        msg = {
            "type": "word_note",
            "data": {
                "keyword": keyword,
                "note": note,
            },
        }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def _handle_delete_word_note(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        keyword = message["data"]["keyword"]
        Utils.db.delete_word_note(keyword)
        msg = {
            "type": "word_note",
            "data": {
                "keyword": keyword,
                "note": "",
            },
        }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def _handle_lookup_keyword_request(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        keyword = message["data"]["keyword"]
        msg = {
            "type": "lookup_keyword_request",
            "data": {
                "keyword": keyword,
            },
        }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def _handle_favorite_words_request(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        folder_id = message["data"]["folder_id"]
        msg = {"type": "favorite_words", "data": {"folder_id": folder_id, "words": []}}
        if folder_id and Utils.db.is_folder_exist(folder_id):
            words = Utils.db.get_folder_words(folder_id)
            msg = {
                "type": "favorite_words",
                "data": {
                    "folder_id": folder_id,
                    "words": words
                    },
            }
        await SessionManager.send_msg_to_session_by_id(
            session_id, connection_id, json.dumps(msg)
        )

    @staticmethod
    async def _handle_search_history_request(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        await SessionManager.send_search_history_to_session(session_id, connection_id)
