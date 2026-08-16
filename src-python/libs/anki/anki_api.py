"""
AnkiConnect API client.
Provides low-level interface for communicating with Anki via AnkiConnect plugin.
"""
import json
import urllib.request
from typing import List, Dict, Optional, Tuple, Any


class AnkiApi:
    """Client for the AnkiConnect HTTP API."""

    API_URL = "http://localhost:8765"
    API_VERSION = 6

    @staticmethod
    def invoke(action: str, timeout: Optional[float] = None, **params) -> Any:
        """
        Generic method to call an AnkiConnect API action.
        Raises Exception on API error.
        """
        request_data = json.dumps({
            "action": action,
            "version": AnkiApi.API_VERSION,
            "params": params
        }).encode("utf-8")

        request = urllib.request.Request(
            AnkiApi.API_URL,
            data=request_data,
            headers={"Content-Type": "application/json"},
        )

        with urllib.request.urlopen(request, timeout=timeout) as response:
            result = json.load(response)
            if result.get("error"):
                raise Exception(f"Anki error: {result['error']}")
            return result.get("result")

    @staticmethod
    def get_deck_cards(deck_name: str) -> List[Dict]:
        """
        Get all cards in a deck with front, back, cardId, and noteId.
        Returns empty list if deck is empty or not found.
        """
        card_ids = AnkiApi.invoke("findCards", query=f'deck:"{deck_name}"')
        if not card_ids:
            return []

        cards_info = AnkiApi.invoke("cardsInfo", cards=card_ids)
        note_ids = [card["note"] for card in cards_info]
        notes_info = AnkiApi.invoke("notesInfo", notes=note_ids)

        result = []
        for card, note in zip(cards_info, notes_info):
            front = note["fields"].get("Front", {}).get("value", "")
            back = note["fields"].get("Back", {}).get("value", "")
            result.append({
                "cardId": card["cardId"],
                "noteId": note["noteId"],
                "front": front,
                "back": back,
            })
        return result

    @staticmethod
    def upsert_note(
        deck_name: str,
        note_id: Optional[int] = None,
        front: str = "",
        back: str = "",
        timeout: Optional[float] = None,
    ) -> Tuple[bool, str]:
        """
        Create or update a note in Anki.
        - If note_id is provided: update existing note (preserves review history)
        - If note_id is None: create new note in the deck
        - Creates deck automatically if it doesn't exist
        Returns (success: bool, message: str)
        """
        # Ensure deck exists
        try:
            decks = AnkiApi.invoke("deckNames", timeout=timeout)
        except Exception as e:
            return False, f"Failed to retrieve deck list: {str(e)}"

        if deck_name not in decks:
            try:
                AnkiApi.invoke("createDeck", deck=deck_name, timeout=timeout)
            except Exception as e:
                return False, f"Failed to create deck: {str(e)}"

        # Update existing note
        if note_id:
            try:
                note_info = AnkiApi.invoke("notesInfo", notes=[note_id], timeout=timeout)[0]
                old_front = note_info["fields"].get("Front", {}).get("value", "")
                old_back = note_info["fields"].get("Back", {}).get("value", "")

                if old_front == front and old_back == back:
                    return True, f"Note {note_id} unchanged"

                AnkiApi.invoke(
                    "updateNoteFields",
                    note={"id": note_id, "fields": {"Front": front, "Back": back}},
                    timeout=timeout
                )
                return True, f"Note {note_id} updated successfully"
            except Exception as e:
                return False, f"Update failed: {str(e)}"

        # Create new note
        else:
            try:
                note = {
                    "deckName": deck_name,
                    "modelName": "Basic",
                    "fields": {"Front": front, "Back": back},
                    "options": {
                        "allowDuplicate": False,
                        "duplicateScope": "deck",
                        "duplicateScopeDeckName": deck_name,
                    },
                    "tags": [],
                }
                new_id = AnkiApi.invoke("addNote", note=note, timeout=timeout)
                return True, f"Note created successfully, ID: {new_id}"
            except Exception as e:
                return False, f"Creation failed: {str(e)}"
