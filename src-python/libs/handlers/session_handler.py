"""
Message handlers for dictionary session WebSocket connections.
All database and blocking operations are offloaded to thread pools via asyncio.to_thread()
to prevent blocking the FastAPI event loop.
"""
import json
import asyncio
from fastapi import WebSocket

from libs.log_config import logger
from libs.core.dictionary import dictionary_searcher
from libs.common.session_manager import SessionManager
from libs.config.app_config import Utils
from libs.anki.anki_manager import anki_manager


class SessionMessageHandler:
    """Handles all messages from dictionary session WebSocket clients."""

    @staticmethod
    async def handle_message(
        websocket: WebSocket,
        session_id: int,
        connection_id: int,
        message_text: str
    ):
        """Route incoming session messages to the appropriate handler."""
        try:
            message = json.loads(message_text)
            message_type = message["type"]

            handler = _HANDLER_MAP.get(message_type)
            if handler:
                await handler(websocket, session_id, connection_id, message)
            else:
                logger.warning(f"Unknown session message type: {message_type}")

        except Exception as e:
            logger.error(f"Error handling session message: {e}", exc_info=True)

    # -----------------------------------------------------------------------
    # Window control messages
    # -----------------------------------------------------------------------

    # @staticmethod
    # async def _handle_note_is_editing(
    #     websocket: WebSocket, session_id: int, connection_id: int, message: dict
    # ):
    #     """Forward note editing state to iWin window manager."""
    #     msg = {
    #         "type": "note_is_editing",
    #         "data": {
    #             "session_id": session_id,
    #             "connection_id": connection_id,
    #             "is_editing": message["data"]["is_editing"],
    #         },
    #     }
    #     await Utils.iwin_ws_client.send(msg)

    # -----------------------------------------------------------------------
    # Dictionary search messages
    # -----------------------------------------------------------------------

    @staticmethod
    async def _handle_search_suggestions(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Handle keyword suggestion / autocomplete search."""
        keyword = message["data"]["keyword"]
        search_method = message["data"]["search_method"]
        dict_settings = message["data"]["dict_settings"]

        options = await asyncio.to_thread(
            dictionary_searcher.keyword_suggestions,
            keyword,
            search_method,
            dict_settings
        )

        response = {
            "type": "keyword_options_search",
            "data": {
                "keyword": keyword,
                "options": options,
            },
        }
        await SessionManager.send_to_connection(
            session_id, connection_id, json.dumps(response)
        )

    @staticmethod
    async def _handle_word_option_note(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Echo word option note back to client."""
        response = {
            "type": "keyword_options_search",
            "data": {
                "keyword": message["data"]["keyword"],
                "options": message["data"]["options"],
            },
        }
        await SessionManager.send_to_connection(
            session_id, connection_id, json.dumps(response)
        )

    @staticmethod
    async def _handle_lookup(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Handle keyword lookup request."""
        keyword = message["data"]["keyword"]
        folder_id = message["data"]["folder_id"]
        left_history = message["data"]["left_history"]
        dict_settings = message["data"]["dict_settings"]

        results = await asyncio.to_thread(
            dictionary_searcher.lookup, keyword, dict_settings
        )

        # Fallback: case-insensitive search for alphabetic keywords
        if not results and keyword.isalpha():
            results = await asyncio.to_thread(
                dictionary_searcher.lookup,
                keyword,
                dict_settings,
                ignorecase=True
            )

        # Fallback: redirect lookup for keywords with # suffix
        if not results and '#' in keyword:
            base_keyword = keyword.split("#")[0]
            msg = {
                "type": "lookup_keyword_request",
                "data": {"keyword": base_keyword},
            }
            await SessionManager.send_to_connection(
                session_id, connection_id, json.dumps(msg)
            )
            return

        is_favorited = False
        if folder_id:
            is_favorited = await asyncio.to_thread(
                Utils.db.is_word_favorited, keyword, folder_id
            )

        note = await asyncio.to_thread(Utils.db.get_word_note, keyword)

        if left_history and (results or note):
            await asyncio.to_thread(Utils.db.add_search_history, keyword)

        response = {
            "type": "lookup_keyword",
            "data": {
                "keyword": keyword,
                "result": results,
                "note": note,
                "is_word_favorited": is_favorited,
                "left_history": left_history,
            },
        }
        await SessionManager.send_to_connection(
            session_id, connection_id, json.dumps(response)
        )

    @staticmethod
    async def _handle_lookup_keyword_request(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Forward a lookup request to the session client."""
        keyword = message["data"]["keyword"]
        response = {
            "type": "lookup_keyword_request",
            "data": {"keyword": keyword},
        }
        await SessionManager.send_to_connection(
            session_id, connection_id, json.dumps(response)
        )

    # -----------------------------------------------------------------------
    # Session management
    # -----------------------------------------------------------------------

    @staticmethod
    async def _handle_update_config(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Update session configuration."""
        config = message["data"]["config"]
        logger.info(f"Updating session {session_id} config")

        await asyncio.to_thread(Utils.db.update_session_config, session_id, config)
        await SessionManager.broadcast_session(session_id, json.dumps(message))

    @staticmethod
    async def _handle_create_session(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Create a new session with the next available ID."""
        all_ids = await asyncio.to_thread(Utils.db.get_all_session_ids)

        # Find the first available ID between 1 and 99
        new_id = None
        for candidate in range(1, 100):
            if candidate not in all_ids:
                new_id = candidate
                break

        if new_id is None:
            logger.error("No available session ID (1-99 range exhausted)")
            return

        await asyncio.to_thread(
            Utils.db.update_session_config, new_id, message["data"]["config"]
        )

        response = {
            "type": "create_session",
            "data": {"session_id": new_id}
        }
        await SessionManager.send_to_connection(
            session_id, connection_id, json.dumps(response)
        )
        await SessionManager.broadcast_all_sessions_id_name()

    @staticmethod
    async def _handle_rename_session(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Rename a session."""
        new_name = message["data"]["name"]
        config = await asyncio.to_thread(Utils.db.get_session_config, session_id)

        if config:
            config["name"] = new_name
            await asyncio.to_thread(Utils.db.update_session_config, session_id, config)

            response = {
                "type": "session_config",
                "data": {"config": config}
            }
            await SessionManager.broadcast_session(session_id, json.dumps(response))
            await SessionManager.broadcast_all_sessions_id_name()

    @staticmethod
    async def _handle_remove_session(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Delete a session."""
        await asyncio.to_thread(Utils.db.delete_session, session_id)
        await SessionManager.broadcast_all_sessions_id_name()

    # -----------------------------------------------------------------------
    # Folder management
    # -----------------------------------------------------------------------

    @staticmethod
    async def _handle_create_folder(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Create a new favorite folder."""
        folder_name = message["data"]["folder_name"]
        folder_description = message["data"]["folder_description"]

        await asyncio.to_thread(
            Utils.db.create_folder, folder_name, folder_description
        )
        await SessionManager.send_folder_config(session_id, connection_id)

    @staticmethod
    async def _handle_delete_folder(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Delete a favorite folder."""
        folder_id = message["data"]["folder_id"]
        await asyncio.to_thread(Utils.db.delete_folder, folder_id)
        await SessionManager.send_folder_config(session_id, connection_id)

    @staticmethod
    async def _handle_update_folder(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Update folder name and description."""
        folder_id = message["data"]["folder_id"]
        folder_name = message["data"]["folder_name"]
        folder_description = message["data"]["folder_description"]

        await asyncio.to_thread(Utils.db.rename_folder, folder_id, folder_name)
        await asyncio.to_thread(
            Utils.db.update_folder_description, folder_id, folder_description
        )
        await SessionManager.send_folder_config(session_id, connection_id)

    @staticmethod
    async def _handle_folder_config(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Request current folder configuration."""
        await SessionManager.send_folder_config(session_id, connection_id)

    @staticmethod
    async def _handle_favorite_words_request(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Request list of favorited words in a folder."""
        folder_id = message["data"]["folder_id"]
        words = []

        if folder_id and await asyncio.to_thread(Utils.db.folder_exists, folder_id):
            words = await asyncio.to_thread(Utils.db.get_folder_words, folder_id)

        response = {
            "type": "favorite_words",
            "data": {"folder_id": folder_id, "words": words}
        }
        await SessionManager.send_to_connection(
            session_id, connection_id, json.dumps(response)
        )

    @staticmethod
    async def _handle_search_history_request(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Request search history for this session."""
        await SessionManager.send_search_history(session_id, connection_id)

    # -----------------------------------------------------------------------
    # Word favorites and notes
    # -----------------------------------------------------------------------

    @staticmethod
    async def _handle_toggle_favor(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Toggle favorite status of a word."""
        keyword = message["data"]["keyword"]
        folder_id = message["data"]["folder_id"]

        currently_favorited = await asyncio.to_thread(
            Utils.db.is_word_favorited, keyword, folder_id
        )
        is_favorited = not currently_favorited

        if is_favorited:
            await asyncio.to_thread(Utils.db.favorite_word, keyword, folder_id)
        else:
            await asyncio.to_thread(Utils.db.unfavorite_word, keyword, folder_id)

        response = {
            "type": "toggle_favor",
            "data": {
                "folder_id": folder_id,
                "keyword": keyword,
                "is_word_favorited": is_favorited,
            },
        }
        await SessionManager.send_to_connection(
            session_id, connection_id, json.dumps(response)
        )

    @staticmethod
    async def _handle_save_note(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Save a note for a word."""
        keyword = message["data"]["keyword"]
        note_content = message["data"]["note"]

        await asyncio.to_thread(Utils.db.save_word_note, keyword, note_content)

        response = {
            "type": "word_note",
            "data": {
                "keyword": keyword,
                "note": note_content,
            },
        }
        await SessionManager.send_to_connection(
            session_id, connection_id, json.dumps(response)
        )

    @staticmethod
    async def _handle_delete_note(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Delete a word's note."""
        keyword = message["data"]["keyword"]
        await asyncio.to_thread(Utils.db.delete_word_note, keyword)

        response = {
            "type": "word_note",
            "data": {
                "keyword": keyword,
                "note": "",
            },
        }
        await SessionManager.send_to_connection(
            session_id, connection_id, json.dumps(response)
        )

    # -----------------------------------------------------------------------
    # System & dictionary configuration
    # -----------------------------------------------------------------------

    @staticmethod
    async def _handle_update_system_config(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Update global system configuration."""
        system_config = message["data"]["system_config"]
        Utils.Config.init_config(system_config)
        await SessionManager.broadcast_all_system_config()

    @staticmethod
    async def _handle_update_dict_config(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Update dictionary set configuration."""
        dict_config = message["data"]["dict_config"]
        Utils.Config.init_dict_config(dict_config)
        await SessionManager.broadcast_all_dict_config()

    @staticmethod
    async def _handle_create_dict_set_option(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Create a new dictionary set preset."""
        option_name = message["data"]["option_name"]
        Utils.Config.create_dict_set_option(option_name)
        await SessionManager.broadcast_all_dict_config()

    @staticmethod
    async def _handle_remove_dict_set_option(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Remove a dictionary set preset."""
        option_name = message["data"]["option_name"]
        Utils.Config.remove_dict_set_option(option_name)
        await SessionManager.broadcast_all_system_config()

    @staticmethod
    async def _handle_rename_dict_set_option(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Rename a dictionary set preset."""
        old_name = message["data"]["old_option_name"]
        new_name = message["data"]["new_option_name"]
        Utils.Config.rename_dict_set_option(old_name, new_name)
        await SessionManager.broadcast_all_system_config()

    # -----------------------------------------------------------------------
    # Dictionary management
    # -----------------------------------------------------------------------

    @staticmethod
    async def _handle_add_dictionary(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Add a new dictionary from file path, with progress reporting."""
        dict_path = message["data"]["dict_path"]

        async def send_progress(msg_data: dict):
            response = {
                "type": "add_dictionary",
                "data": msg_data
            }
            await SessionManager.send_to_connection(
                session_id, connection_id, json.dumps(response)
            )

        await dictionary_searcher.add_dictionary(dict_path, send_progress)

        # Send completion signal
        await send_progress({"type": "done"})
        await SessionManager.broadcast_all_dict_info()
        await SessionManager.broadcast_all_system_config()

    @staticmethod
    async def _handle_show_dict_in_folder(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Open file manager and locate the dictionary file."""
        dict_name = message["data"]["dict_name"]
        Utils.reveal_dict_in_file_manager(dict_name)

    @staticmethod
    async def _handle_delete_dictionary(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Delete a dictionary."""
        dict_name = message["data"]["dict_name"]
        dictionary_searcher.remove_dictionary(dict_name)
        Utils.delete_dictionary(dict_name)
        await SessionManager.broadcast_all_dict_info()
        await SessionManager.broadcast_all_system_config()

    # -----------------------------------------------------------------------
    # Anki integration
    # -----------------------------------------------------------------------

    @staticmethod
    async def _handle_update_to_anki(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Start syncing favorite words to Anki deck."""
        anki_manager.set_cancel_flag(False)
        deck_name = message["data"]["deck_name"]
        folder_id = message["data"]["folder_id"]

        async def send_progress(progress_data: dict):
            response = {
                "type": "anki_progress",
                "deck_name": deck_name,
                "data": progress_data,
            }
            try:
                await SessionManager.send_to_connection(
                    session_id, connection_id, json.dumps(response)
                )
            except Exception as e:
                logger.error(f"Failed to send Anki progress: {e}", exc_info=True)

        words = await asyncio.to_thread(Utils.db.get_folder_words, folder_id)

        await anki_manager.update_words_to_anki(
            str(session_id), deck_name, words, send_progress
        )

    @staticmethod
    async def _handle_cancel_anki_update(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Cancel a running Anki sync operation."""
        logger.info("Anki update cancellation requested")
        anki_manager.set_cancel_flag(True)

    @staticmethod
    async def _handle_simulate_key_press(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Simulate a key press."""
        if Utils.fstdict_main_websocket:
            await Utils.fstdict_main_websocket.send_text(json.dumps(message))

    @staticmethod
    async def _handle_update_shortcut_config(
        websocket: WebSocket, session_id: int, connection_id: int, message: dict
    ):
        """Update global shortcut configuration."""
        shortcut_name = message["data"]["shortcut_name"]
        shortcuts = message["data"]["shortcuts"]

        await Utils.Config.update_shortcut(shortcut_name, shortcuts)
        await SessionManager.broadcast_all_system_config()


# ---------------------------------------------------------------------------
# Message type to handler routing table
# ---------------------------------------------------------------------------
_HANDLER_MAP = {
    # Window control
    # "note_is_editing": SessionMessageHandler._handle_note_is_editing,

    # Search
    "keyword_options_search": SessionMessageHandler._handle_search_suggestions,
    "word_option_note": SessionMessageHandler._handle_word_option_note,
    "lookup_keyword": SessionMessageHandler._handle_lookup,
    "lookup_keyword_request": SessionMessageHandler._handle_lookup_keyword_request,

    # Session management
    "session_config": SessionMessageHandler._handle_update_config,
    "create_session": SessionMessageHandler._handle_create_session,
    "rename_session": SessionMessageHandler._handle_rename_session,
    "remove_session": SessionMessageHandler._handle_remove_session,

    # Folder management
    "create_folder": SessionMessageHandler._handle_create_folder,
    "delete_folder": SessionMessageHandler._handle_delete_folder,
    "update_folder": SessionMessageHandler._handle_update_folder,
    "folder_config": SessionMessageHandler._handle_folder_config,
    "favorite_words_request": SessionMessageHandler._handle_favorite_words_request,
    "search_history_request": SessionMessageHandler._handle_search_history_request,

    # Word favorites & notes
    "toggle_favor": SessionMessageHandler._handle_toggle_favor,
    "save_word_note": SessionMessageHandler._handle_save_note,
    "delete_word_note": SessionMessageHandler._handle_delete_note,

    # System & dict config
    "update_system_config": SessionMessageHandler._handle_update_system_config,
    "update_dict_config": SessionMessageHandler._handle_update_dict_config,
    "create_dict_set_option": SessionMessageHandler._handle_create_dict_set_option,
    "remove_dict_set_option": SessionMessageHandler._handle_remove_dict_set_option,
    "rename_dict_set_option": SessionMessageHandler._handle_rename_dict_set_option,

    # Dictionary management
    "add_dictionary": SessionMessageHandler._handle_add_dictionary,
    "show_dict_in_folder": SessionMessageHandler._handle_show_dict_in_folder,
    "delete_dictionary": SessionMessageHandler._handle_delete_dictionary,

    # Anki integration
    "update_to_anki": SessionMessageHandler._handle_update_to_anki,
    "cancel_anki_update": SessionMessageHandler._handle_cancel_anki_update,

    # keyboard simulation
    "simulate_key_press": SessionMessageHandler._handle_simulate_key_press,

    # keyboard shortcuts
    "update_shortcut_config": SessionMessageHandler._handle_update_shortcut_config,
}
