"""
SQLite database layer for user data: sessions, favorites, history, notes.
All methods are synchronous (SQLite) and should be called via asyncio.to_thread().
"""
import sqlite3
import json
from typing import Optional, List, Dict, Any
from libs.log_config import logger


class FstDictDatabase:
    """Database access layer for FstDict user data."""

    def __init__(self, db_path: str):
        self.conn = sqlite3.connect(db_path, check_same_thread=False)
        self.conn.row_factory = sqlite3.Row
        self._enable_foreign_keys()
        self._create_tables()
        self._create_indexes()

    def _enable_foreign_keys(self) -> None:
        """Enable foreign key constraints."""
        with self.conn:
            self.conn.execute("PRAGMA foreign_keys = ON")
            cursor = self.conn.execute("PRAGMA foreign_keys")
            if cursor.fetchone()[0] != 1:
                raise RuntimeError("Failed to enable foreign key constraints")

    def _create_tables(self) -> None:
        """Create all database tables if they do not exist."""
        with self.conn:
            c = self.conn.cursor()

            # Sessions table
            c.execute("""
                CREATE TABLE IF NOT EXISTS sessions (
                    id INTEGER PRIMARY KEY,
                    config TEXT
                )
            """)

            # Words table
            c.execute("""
                CREATE TABLE IF NOT EXISTS words (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    word TEXT NOT NULL UNIQUE,
                    query_count INTEGER DEFAULT 0,
                    created_at TIMESTAMP DEFAULT (datetime('now','localtime'))
                )
            """)

            # Folders table
            c.execute("""
                CREATE TABLE IF NOT EXISTS folders (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT,
                    created_at TIMESTAMP DEFAULT (datetime('now','localtime'))
                )
            """)

            # Word favorites junction table
            c.execute("""
                CREATE TABLE IF NOT EXISTS word_favorites (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    word_id INTEGER NOT NULL,
                    folder_id INTEGER NOT NULL,
                    created_at TIMESTAMP DEFAULT (datetime('now','localtime')),
                    FOREIGN KEY (word_id) REFERENCES words(id) ON DELETE CASCADE,
                    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE CASCADE,
                    UNIQUE(word_id, folder_id)
                )
            """)

            # Search history table
            c.execute("""
                CREATE TABLE IF NOT EXISTS word_search_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    word_id INTEGER NOT NULL,
                    searched_at TIMESTAMP DEFAULT (datetime('now','localtime')),
                    FOREIGN KEY (word_id) REFERENCES words(id) ON DELETE CASCADE
                )
            """)

            # Word notes table
            c.execute("""
                CREATE TABLE IF NOT EXISTS word_notes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    word_id INTEGER NOT NULL,
                    content TEXT NOT NULL,
                    updated_at TIMESTAMP DEFAULT (datetime('now','localtime')),
                    FOREIGN KEY (word_id) REFERENCES words(id) ON DELETE CASCADE,
                    UNIQUE(word_id)
                )
            """)

    def _create_indexes(self) -> None:
        """Create performance indexes."""
        with self.conn:
            c = self.conn.cursor()
            c.execute("CREATE INDEX IF NOT EXISTS idx_word ON words(word);")
            c.execute("CREATE INDEX IF NOT EXISTS idx_folder_name ON folders(name);")
            c.execute("CREATE INDEX IF NOT EXISTS idx_word_favorites_word_id ON word_favorites(word_id);")
            c.execute("CREATE INDEX IF NOT EXISTS idx_word_favorites_folder_id ON word_favorites(folder_id);")
            c.execute("CREATE INDEX IF NOT EXISTS idx_word_search_history_word_id ON word_search_history(word_id);")
            c.execute("CREATE INDEX IF NOT EXISTS idx_word_notes_word_id ON word_notes(word_id);")

    def close(self) -> None:
        """Close the database connection."""
        self.conn.close()

    # --- Session operations ---

    def create_session(self, session_id: int, config: Dict[str, Any]) -> None:
        with self.conn:
            self.conn.execute(
                "INSERT INTO sessions (id, config) VALUES (?, ?)",
                (session_id, json.dumps(config, ensure_ascii=False))
            )

    def delete_session(self, session_id: int) -> None:
        with self.conn:
            self.conn.execute("DELETE FROM sessions WHERE id = ?", (session_id,))

    def get_session_config(self, session_id: int) -> Optional[Dict[str, Any]]:
        cursor = self.conn.execute(
            "SELECT config FROM sessions WHERE id = ?", (session_id,)
        )
        row = cursor.fetchone()
        return json.loads(row["config"]) if row else None

    def update_session_config(self, session_id: int, config: Dict[str, Any]) -> None:
        if not self.session_exists(session_id):
            self.create_session(session_id, config)
            return
        with self.conn:
            self.conn.execute(
                "UPDATE sessions SET config = ? WHERE id = ?",
                (json.dumps(config, ensure_ascii=False), session_id)
            )

    def session_exists(self, session_id: int) -> bool:
        cursor = self.conn.execute(
            "SELECT id FROM sessions WHERE id = ?", (session_id,)
        )
        return cursor.fetchone() is not None

    def get_all_session_ids(self) -> List[int]:
        cursor = self.conn.execute("SELECT id FROM sessions ORDER BY id")
        return [row["id"] for row in cursor.fetchall()]

    def get_all_sessions(self) -> List[Dict]:
        cursor = self.conn.execute("SELECT * FROM sessions ORDER BY id")
        return [
            {"id": row["id"], "config": json.loads(row["config"])}
            for row in cursor.fetchall()
        ]

    # --- Word operations ---

    def get_or_create_word(self, word: str) -> int:
        """Get word ID, creating the word entry if it doesn't exist."""
        word = word.strip().lower()
        with self.conn:
            cursor = self.conn.execute(
                "SELECT id FROM words WHERE word = ?", (word,)
            )
            row = cursor.fetchone()
            if row:
                return row["id"]

            cursor.execute("INSERT INTO words (word) VALUES (?)", (word,))
            if cursor.lastrowid:
                return int(cursor.lastrowid)
            else:
                raise ValueError("Failed to insert word into database")

    def add_search_history(self, word: str) -> None:
        """Record a search query in history."""
        word_id = self.get_or_create_word(word)
        with self.conn:
            self.conn.execute(
                "INSERT INTO word_search_history (word_id) VALUES (?)", (word_id,)
            )
            self.conn.execute(
                "UPDATE words SET query_count = query_count + 1 WHERE id = ?",
                (word_id,)
            )

    # --- Folder operations ---

    def create_folder(self, name: str, description: str = "") -> int:
        with self.conn:
            cursor = self.conn.execute(
                "INSERT INTO folders (name, description) VALUES (?, ?)",
                (name, description)
            )
            if cursor.lastrowid:
                return int(cursor.lastrowid)
            else:
                raise ValueError("Failed to insert folder into database")

    def get_all_folders(self) -> List[Dict]:
        cursor = self.conn.execute(
            "SELECT * FROM folders ORDER BY created_at DESC"
        )
        return [dict(row) for row in cursor.fetchall()]

    def get_folder_word_count(self, folder_id: int) -> int:
        cursor = self.conn.execute(
            "SELECT COUNT(*) FROM word_favorites WHERE folder_id = ?",
            (folder_id,)
        )
        return cursor.fetchone()[0]

    def get_all_folder_info(self) -> List[Dict]:
        folders = self.get_all_folders()
        for folder in folders:
            folder["words_count"] = self.get_folder_word_count(folder["id"])
        return folders

    def get_folder_id_by_name(self, folder_name: str) -> Optional[int]:
        cursor = self.conn.execute(
            "SELECT id FROM folders WHERE name = ?", (folder_name,)
        )
        row = cursor.fetchone()
        return row["id"] if row else None

    def folder_exists(self, folder_id: int) -> bool:
        cursor = self.conn.execute(
            "SELECT 1 FROM folders WHERE id = ?", (folder_id,)
        )
        return cursor.fetchone() is not None

    def rename_folder(self, folder_id: int, new_name: str) -> bool:
        try:
            with self.conn:
                cursor = self.conn.execute(
                    "UPDATE folders SET name = ? WHERE id = ?",
                    (new_name.strip(), folder_id)
                )
                return cursor.rowcount > 0
        except sqlite3.IntegrityError:
            return False

    def update_folder_description(self, folder_id: int, description: str) -> bool:
        with self.conn:
            cursor = self.conn.execute(
                "UPDATE folders SET description = ? WHERE id = ?",
                (description, folder_id)
            )
            return cursor.rowcount > 0

    def delete_folder(self, folder_id: int) -> bool:
        with self.conn:
            cursor = self.conn.execute(
                "DELETE FROM folders WHERE id = ?", (folder_id,)
            )
            return cursor.rowcount > 0

    # --- Favorite operations ---

    def favorite_word(self, word: str, folder_id: int) -> bool:
        try:
            word_id = self.get_or_create_word(word)
            with self.conn:
                self.conn.execute(
                    "INSERT INTO word_favorites (word_id, folder_id) VALUES (?, ?)",
                    (word_id, folder_id)
                )
            return True
        except sqlite3.IntegrityError:
            return False

    def unfavorite_word(self, word: str, folder_id: int) -> bool:
        word = word.strip().lower()
        with self.conn:
            cursor = self.conn.execute("""
                DELETE FROM word_favorites
                WHERE word_id = (SELECT id FROM words WHERE word = ?) AND folder_id = ?
            """, (word, folder_id))
            return cursor.rowcount > 0

    def is_word_favorited(self, word: str, folder_id: int) -> bool:
        word = word.strip().lower()
        cursor = self.conn.execute("""
            SELECT 1 FROM word_favorites
            WHERE word_id = (SELECT id FROM words WHERE word = ?) AND folder_id = ?
        """, (word, folder_id))
        return cursor.fetchone() is not None

    def get_folder_words(self, folder_id: int) -> List[Dict]:
        cursor = self.conn.execute("""
            SELECT w.word, w.created_at, w.query_count, wf.created_at as favorited_at
            FROM word_favorites wf
            JOIN words w ON wf.word_id = w.id
            WHERE wf.folder_id = ?
            ORDER BY wf.created_at DESC
        """, (folder_id,))
        return [dict(row) for row in cursor.fetchall()]

    def get_folder_words_by_name(self, folder_name: str) -> List[Dict]:
        folder_id = self.get_folder_id_by_name(folder_name)
        if folder_id is None:
            return []
        return self.get_folder_words(folder_id)

    # --- Search history ---

    def get_search_history(self, limit: int = 100) -> List[Dict]:
        cursor = self.conn.execute("""
            SELECT DISTINCT w.word, w.query_count, MAX(h.searched_at) as last_searched
            FROM word_search_history h
            JOIN words w ON h.word_id = w.id
            GROUP BY w.id
            ORDER BY last_searched DESC
            LIMIT ?
        """, (limit,))
        return [dict(row) for row in cursor.fetchall()]

    # --- Word notes ---

    def save_word_note(self, word: str, content: str) -> bool:
        word_id = self.get_or_create_word(word)
        try:
            with self.conn:
                self.conn.execute("""
                    INSERT OR REPLACE INTO word_notes (word_id, content, updated_at)
                    VALUES (?, ?, datetime('now','localtime'))
                """, (word_id, content.strip()))
            return True
        except Exception as e:
            logger.error(f"Failed to save note: {e}")
            return False

    def get_word_note(self, word: str) -> Optional[str]:
        word = word.strip().lower()
        cursor = self.conn.execute("""
            SELECT n.content FROM word_notes n
            JOIN words w ON n.word_id = w.id
            WHERE w.word = ?
        """, (word,))
        row = cursor.fetchone()
        return row["content"] if row else None

    def delete_word_note(self, word: str) -> bool:
        word = word.strip().lower()
        with self.conn:
            cursor = self.conn.execute("""
                DELETE FROM word_notes
                WHERE word_id = (SELECT id FROM words WHERE word = ?)
            """, (word,))
            return cursor.rowcount > 0
