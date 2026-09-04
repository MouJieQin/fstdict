"""
Handler for HTTP command API requests.
"""
import json
from typing import Dict

from libs.log_config import logger
from libs.config.app_config import Utils
from libs.common.session_manager import SessionManager


class CommandHandler:
    """Handles command requests from the HTTP API."""

    @staticmethod
    async def handle(command_type: str, data: dict) -> Dict:
        """Route command to the appropriate handler."""
        if command_type == "lookup_keyword_request":
            session_id = data["session_id"]
            msg = {"type": "lookup_keyword_request", "data": data}
            await SessionManager.broadcast_session(session_id, json.dumps(msg))
            return {"success": True}

        elif command_type == "acquire_words_from_folder":
            return CommandHandler._get_folder_words(data)

        elif command_type == "favorite_words_to_folder":
            return CommandHandler._favorite_words_to_folder(data)

        elif command_type == "check_running":
            return {"success": True}

        else:
            logger.warning(f"Unknown command type: {command_type}")
            return {"success": False, "message": "Unknown command type"}

    @staticmethod
    def _get_folder_words(data: dict) -> Dict:
        """Retrieve all words from a folder in Anki-compatible format."""
        folder_name = data["folder_name"]
        words = Utils.db.get_folder_words_by_name(folder_name)
        for word in words:
            word["note"] = Utils.db.get_word_note(word["word"])
            word["definition"] = "unknown"
        return {"success": True, "data": {"words": words}}

    @staticmethod
    def _favorite_words_to_folder(data: dict) -> Dict:
        """Add a list of words to a favorite folder."""
        folder_name = data["folder_name"]
        folder_id = Utils.db.get_folder_id_by_name(folder_name)

        if folder_id is None:
            logger.error(f"Folder '{folder_name}' does not exist")
            return {"success": False, "message": f"Folder '{folder_name}' does not exist"}

        words = data["words"]
        for word in words:
            if not Utils.db.is_word_favorited(word, folder_id):
                Utils.db.favorite_word(word, folder_id)

        return {"success": True}
