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
        self._migrate_schema()   # <-- NEW: runs AFTER create_tables
        self._create_indexes()

    def _enable_foreign_keys(self) -> None:
        """Enable foreign key constraints."""
        with self.conn:
            self.conn.execute("PRAGMA foreign_keys = ON")
            cursor = self.conn.execute("PRAGMA foreign_keys")
            if cursor.fetchone()[0] != 1:
                raise RuntimeError("Failed to enable foreign key constraints")

    # ------------------------------------------------------------------
    # Schema migration
    # ------------------------------------------------------------------
    def _migrate_schema(self) -> None:
        """
        Idempotent schema migrations.

        CREATE TABLE IF NOT EXISTS never touches an existing table, so any
        column added to a table definition only takes effect for FRESH
        databases. For databases created before the column existed we
        ALTER here.

        Strategy for word_search_history.session_id:
          - existing rows keep the column NULL = "global history" (not
            bound to any session). They stay visible through the original
            get_search_history() and through the new per-session queries
            when include_global=True (the default), so no history
            disappears after the migration.
          - new rows written with add_search_history(word, session_id)
            carry their real session id.
        """
        with self.conn:
            cols = {
                row["name"]
                for row in self.conn.execute("PRAGMA table_info(word_search_history)")
            }
            if "session_id" not in cols:
                logger.info("Migrating word_search_history: adding session_id column")
                self.conn.execute(
                    "ALTER TABLE word_search_history ADD COLUMN session_id INTEGER"
                )
                # NULL is a valid value for the new column on all existing rows.
                self.conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_word_search_history_session_id "
                    "ON word_search_history(session_id)"
                )
            logger.debug("Schema check passed: word_search_history.session_id present")

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
            # session_id semantics:
            #   NULL  -> global history (legacy rows / calls that did not
            #            specify a session)
            #   <id>  -> history originated from that session
            # Deliberately NOT a FOREIGN KEY to sessions(id): session ids
            # are logical ids owned by the frontend and may legitimately be
            # created/removed out of band; a hard FK would drop history.
            c.execute("""
                CREATE TABLE IF NOT EXISTS word_search_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    word_id INTEGER NOT NULL,
                    session_id INTEGER,
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

    # --- Search history ---

    def add_search_history(self, word: str, session_id: Optional[int] = None) -> None:
        """
        Record a search query in history.

        Backward compatible: session_id defaults to None, which stores a
        NULL -> the row belongs to the GLOBAL history. Existing callers
        (add_search_history(word)) keep working unchanged.
        """
        word_id = self.get_or_create_word(word)
        with self.conn:
            self.conn.execute(
                "INSERT INTO word_search_history (word_id, session_id) VALUES (?, ?)",
                (word_id, session_id),
            )
            self.conn.execute(
                "UPDATE words SET query_count = query_count + 1 WHERE id = ?",
                (word_id,),
            )

    def get_search_history(self, limit: int = 100, session_id: int | None = None) -> list[dict]:
        """
        History filtered to ONE session.
        session_id : the session whose history is requested.

        Original interface, unchanged: returns the full history across ALL
        sessions (global + every session). Kept for backward compatibility.
        """
        where = ""
        if session_id is None:
            args: tuple = (limit,)
        else:
            where = "WHERE h.session_id = ?"
            args: tuple = (session_id, limit)

        cursor = self.conn.execute(f"""
            SELECT DISTINCT
                w.word,
                w.created_at,
                w.query_count,
                MAX(h.searched_at) AS last_searched
            FROM word_search_history h
            JOIN words w ON h.word_id = w.id
            {where}
            GROUP BY w.id
            ORDER BY last_searched DESC
            LIMIT ?
        """, args)
        return [dict(row) for row in cursor.fetchall()]

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
                SELECT
                    w.word,
                    w.created_at,
                    w.query_count,
                    MAX(h.searched_at) AS last_searched,
                    wf.created_at AS favorited_at
                FROM word_favorites wf
                JOIN words w ON wf.word_id = w.id
                LEFT JOIN word_search_history h ON w.id = h.word_id
                WHERE wf.folder_id = ?
                GROUP BY w.id, wf.id
                ORDER BY wf.created_at DESC
            """, (folder_id,))
        return [dict(row) for row in cursor.fetchall()]

    def get_folder_words_by_name(self, folder_name: str) -> List[Dict]:
        folder_id = self.get_folder_id_by_name(folder_name)
        if folder_id is None:
            return []
        return self.get_folder_words(folder_id)

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

    def get_note_words(self) -> List[Dict]:
        cursor = self.conn.execute("""
            SELECT
                w.word,
                w.created_at,
                w.query_count,
                MAX(h.searched_at) AS last_searched,
                wn.updated_at
            FROM words w
            JOIN word_notes wn ON w.id = wn.word_id
            LEFT JOIN word_search_history h ON w.id = h.word_id
            GROUP BY w.id, wn.id
            ORDER BY wn.updated_at DESC
        """)
        return [dict(row) for row in cursor.fetchall()]
