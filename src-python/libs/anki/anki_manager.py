"""
High-level Anki integration manager.
Handles batch sync operations with progress tracking and cancellation support.
"""
import hashlib
import json
import os
import queue
import asyncio
from typing import Callable, List, Dict

from libs.config.app_config import Utils
from libs.anki.anki_api import AnkiApi
from libs.log_config import logger


class AnkiManager:
    """Manages Anki card synchronization operations."""

    HTML_BACK_PREFIX = """
            <meta charset="utf-8">
            <style>
                body { 
                    overflow-x: hidden;
                    overflow-y: hidden;
                    margin: 0;
                    padding: 0; 
                }
                body::-webkit-scrollbar {
                    display: none;
                }
                iframe { width: 100%; height: 85vh; border: none; }
            </style>
        <body>
    """

    def __init__(self):
        self.anki_api = AnkiApi()
        self._div_prefix = '<div id="'
        self._div_suffix = "</div>"
        self._cancel_flag = False
        self._load_config()

    def _load_config(self) -> None:
        """Load Anki configuration from file."""
        if os.path.exists(Utils.ANKI_CONFIG_FILE):
            with open(Utils.ANKI_CONFIG_FILE, "r") as f:
                self.config = json.load(f)
        else:
            self.config = {
                "format": {
                    "default": {
                        "front": '<p style="font-size: 32px; font-weight: bold;">{keyword}</p>',
                    }
                }
            }
            self._save_config()

    def _save_config(self) -> None:
        with open(Utils.ANKI_CONFIG_FILE, mode="w", encoding="utf-8") as f:
            f.write(json.dumps(self.config, ensure_ascii=False, indent=4))

    @staticmethod
    def _generate_unique_id(text: str) -> str:
        """Generate a stable unique ID from a word string."""
        return hashlib.md5(text.strip().encode("utf-8")).hexdigest()

    def _extract_unique_id(self, front_html: str) -> str:
        """Extract the unique ID from card front HTML."""
        start = front_html.find(self._div_prefix)
        end = front_html.find('">', start)
        if start == -1 or end == -1:
            return ""
        return front_html[start + len(self._div_prefix): end]

    def _get_deck_card_index(self, deck_name: str) -> Dict[str, Dict]:
        """Get deck cards indexed by unique ID for fast lookup."""
        cards = self.anki_api.get_deck_cards(deck_name)
        index = {}
        for card in cards:
            uid = self._extract_unique_id(card["front"])
            index[uid] = card
        return index

    def set_cancel_flag(self, cancel: bool) -> None:
        """Set the cancellation flag to abort a running sync."""
        self._cancel_flag = cancel

    def is_cancelled(self) -> bool:
        return self._cancel_flag

    async def update_words_to_anki(
        self,
        session_id: str,
        deck_name: str,
        words: List[Dict],
        send_progress: Callable,
    ) -> None:
        """
        Batch sync words to an Anki deck.
        Runs the sync in a background thread and streams progress via callback.
        Supports cancellation via set_cancel_flag().
        """
        if self.is_cancelled():
            await send_progress({"type": "canceled"})
            return

        self._load_config()

        # Get front template for this deck
        deck_format = self.config["format"].get(deck_name, {})
        front_template = deck_format.get("front", "") or self.config["format"]["default"]["front"]

        await send_progress({"type": "trying_acquiring_cards_from_anki"})

        # Queue for thread-to-async progress communication
        progress_queue = queue.Queue()

        def sync_worker():
            """Runs in worker thread; performs the actual sync work."""
            logger.debug("Anki sync worker started")

            # Get existing cards
            try:
                existing_cards = self._get_deck_card_index(deck_name)
                logger.debug(f"Found {len(existing_cards)} existing cards in deck")
            except Exception as e:
                progress_queue.put(("error", "Failed to retrieve Anki cards. Ensure Anki is running and AnkiConnect is installed."))
                logger.error(e)
                return

            total = len(words)
            count = updated = created = update_errors = create_errors = 0

            try:
                for word in words:
                    if self.is_cancelled():
                        progress_queue.put(("canceled", count, total, updated, created, update_errors, create_errors))
                        return

                    word_text = word["word"]
                    unique_id = self._generate_unique_id(word_text)

                    # Build front HTML with unique ID wrapper
                    front_content = front_template.format(keyword=word_text)
                    front = f'{self._div_prefix}{unique_id}">\n{front_content}\n{self._div_suffix}'

                    # Build back HTML with embedded dictionary iframe
                    back = self.HTML_BACK_PREFIX
                    back += f'<iframe src="http://127.0.0.1:9595/#/dict/{session_id}?keyword={word_text}&env=anki"></iframe>\n'
                    back += "</body>\n</html>"

                    note_id = existing_cards.get(unique_id, {}).get("noteId")
                    success, _ = self.anki_api.upsert_note(
                        deck_name, note_id, front, back, timeout=5.0
                    )

                    count += 1
                    if success:
                        if note_id:
                            updated += 1
                        else:
                            created += 1
                    else:
                        if note_id:
                            update_errors += 1
                        else:
                            create_errors += 1

                    # Report progress every 10 words
                    if count % 10 == 0:
                        progress_queue.put(("progress", count, total, updated, created, update_errors, create_errors))

                progress_queue.put(("done", count, total, updated, created, update_errors, create_errors))

            except Exception as e:
                progress_queue.put(("error", "Anki sync failed"))
                logger.error(e)
                return

        # Start sync in background thread
        sync_task = asyncio.create_task(asyncio.to_thread(sync_worker), name=f"anki-{deck_name}")

        # Process progress messages asynchronously
        async def process_progress():
            while True:
                if progress_queue.empty():
                    if sync_task.done():
                        logger.debug("Anki sync task completed")
                        break
                    await asyncio.sleep(0.05)
                    continue

                msg = progress_queue.get_nowait()
                if msg[0] in ("canceled", "progress", "done"):
                    await send_progress({
                        "type": msg[0],
                        "data": {
                            "count": msg[1],
                            "total_count": msg[2],
                            "updated_count": msg[3],
                            "created_count": msg[4],
                            "update_error_count": msg[5],
                            "create_error_count": msg[6],
                        },
                    })
                elif msg[0] == "error":
                    await send_progress({"type": "error", "data": {"error_message": msg[1]}})
                    break

        await process_progress()


# Global singleton instance
anki_manager = AnkiManager()
