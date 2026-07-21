import sqlite3
import json
from typing import Optional, List, Dict, Any, Tuple
from datetime import datetime
from libs.log_config import logger


class FstDictDatabase:
    def __init__(self, db_path: str):
        self.conn = sqlite3.connect(db_path)
        self.conn.row_factory = sqlite3.Row  # 使用字典形式返回结果
        self._setup_foreign_keys()
        self._create_tables()
        self._create_indexes()

    def _setup_foreign_keys(self) -> None:
        """确保外键约束启用"""
        with self.conn:
            self.conn.execute("PRAGMA foreign_keys = ON")
            # 验证约束是否启用
            cursor = self.conn.cursor()
            cursor.execute("PRAGMA foreign_keys")
            if cursor.fetchone()[0] != 1:
                raise RuntimeError("无法启用外键约束")

    def _create_tables(self) -> None:
        """创建所有表：会话、单词、收藏夹、收藏关系、查询历史、单词笔记"""
        with self.conn:
            cursor = self.conn.cursor()
            # 会话表
            cursor.execute(
                """
                CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY,
                config TEXT
            )
        """
            )

        # 单词表
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS words (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL UNIQUE,
                query_count INTEGER DEFAULT 0,
                created_at TIMESTAMP DEFAULT (datetime('now','localtime'))
            )
        """
        )
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_word ON words(word);")

        # 收藏夹表
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS folders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at TIMESTAMP DEFAULT (datetime('now','localtime'))
            )
        """
        )

        # 单词收藏关系表
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS word_favorites (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word_id INTEGER NOT NULL,
                folder_id INTEGER NOT NULL,
                created_at TIMESTAMP DEFAULT (datetime('now','localtime')),
                FOREIGN KEY (word_id) REFERENCES words(id) ON DELETE CASCADE,
                FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE CASCADE,
                UNIQUE(word_id, folder_id)
            )
        """
        )

        # 查询历史表
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS word_search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word_id INTEGER NOT NULL,
                searched_at TIMESTAMP DEFAULT (datetime('now','localtime')),
                FOREIGN KEY (word_id) REFERENCES words(id) ON DELETE CASCADE
            )
        """
        )

        # ===================== 新增：单词笔记表 =====================
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS word_notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word_id INTEGER NOT NULL,
                content TEXT NOT NULL,            -- 笔记内容
                updated_at TIMESTAMP DEFAULT (datetime('now','localtime')),
                FOREIGN KEY (word_id) REFERENCES words(id) ON DELETE CASCADE,
                UNIQUE(word_id)                   -- 一个单词只允许一条笔记
            )
        """
        )

    def _create_indexes(self):
        """创建索引，查询速度提升10~100倍"""
        with self.conn:
            c = self.conn.cursor()
            c.execute("CREATE INDEX IF NOT EXISTS idx_word ON words(word);")
            c.execute("CREATE INDEX IF NOT EXISTS idx_folder_name ON folders(name);")
            c.execute(
                "CREATE INDEX IF NOT EXISTS idx_word_favorites_word_id ON word_favorites(word_id);"
            )
            c.execute(
                "CREATE INDEX IF NOT EXISTS idx_word_favorites_folder_id ON word_favorites(folder_id);"
            )
            c.execute(
                "CREATE INDEX IF NOT EXISTS idx_word_search_history_word_id ON word_search_history(word_id);"
            )
            c.execute(
                "CREATE INDEX IF NOT EXISTS idx_word_notes_word_id ON word_notes(word_id);"
            )

    def close(self) -> None:
        self.conn.close()

    def create_session(self, session_id: int, config_config: Dict[str, Any]):
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute(
                "INSERT INTO sessions (id, config) VALUES (?, ?)",
                (session_id, json.dumps(config_config, ensure_ascii=False)),
            )

    def delete_session(self, session_id: int):
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute("DELETE FROM sessions WHERE id = ?", (session_id,))

    def get_session_config(self, session_id: int) -> Optional[Dict[str, Any]]:
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute("SELECT config FROM sessions WHERE id = ?", (session_id,))
            row = cursor.fetchone()
            if row:
                return json.loads(row["config"])
            return None

    def is_session_exist(self, session_id: int) -> bool:
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute("SELECT id FROM sessions WHERE id = ?", (session_id,))
            return bool(cursor.fetchone())

    def update_session_config(self, session_id: int, config_config: Dict[str, Any]):
        if not self.is_session_exist(session_id):
            self.create_session(session_id, config_config)
            return
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute(
                "UPDATE sessions SET config = ? WHERE id = ?",
                (json.dumps(config_config, ensure_ascii=False), session_id),
            )

    def get_all_session_id(self) -> List[int]:
        cursor = self.conn.execute("SELECT id FROM sessions ORDER BY id")
        return [row["id"] for row in cursor.fetchall()]

    def get_all_sessions(self) -> List[Dict]:
        cursor = self.conn.execute("SELECT * FROM sessions ORDER BY id")
        return [dict({"id": row["id"], "config": json.loads(row["config"])}) for row in cursor.fetchall()]

    # ======================== 单词核心操作 ========================
    def get_or_create_word(self, word: str) -> int:
        """
        核心方法：查询单词，不存在则插入
        返回：单词ID
        """
        word = word.strip().lower()  # 统一小写，避免重复
        with self.conn:
            cursor = self.conn.cursor()
            # 先查询
            cursor.execute("SELECT id FROM words WHERE word = ?", (word,))
            row = cursor.fetchone()
            if row:
                word_id = row["id"]
                # 查询次数 +1
                # cursor.execute(
                #     "UPDATE words SET query_count = query_count + 1 WHERE id = ?",
                #     (word_id,),
                # )
                return word_id
            # 不存在则插入
            cursor.execute("INSERT INTO words (word) VALUES (?)", (word,))
            if cursor.lastrowid:
                return int(cursor.lastrowid)
            else:
                raise ValueError("Failed to insert word into database")

    def add_search_history(self, word: str) -> None:
        """记录一次查询历史（自动创建单词）"""
        word_id = self.get_or_create_word(word)
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute(
                "INSERT INTO word_search_history (word_id) VALUES (?)", (word_id,)
            )
            # 查询次数 +1
            cursor.execute(
                "UPDATE words SET query_count = query_count + 1 WHERE id = ?",
                (word_id,),
            )

    def is_word_exists(self, word: str) -> bool:
        """检查单词是否在数据库中"""
        word = word.strip().lower()
        cursor = self.conn.execute("SELECT 1 FROM words WHERE word = ?", (word,))
        return cursor.fetchone() is not None

    # ======================== 收藏夹操作 ========================
    def get_default_folder_id(self, session_id: int) -> Optional[int]:
        """获取会话的默认收藏夹ID"""
        config = self.get_session_config(session_id)
        if config is None:
            return None
        default_folder_id = config.get("default_folder", {}).get("id")
        if default_folder_id is None:
            return None
        return default_folder_id

    def is_folder_exist(self, folder_id: int) -> bool:
        """检查收藏夹是否存在"""
        cursor = self.conn.execute("SELECT 1 FROM folders WHERE id = ?", (folder_id,))
        return cursor.fetchone() is not None

    def create_folder(self, name: str, description: str = "") -> int:
        """创建收藏夹"""
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute(
                "INSERT INTO folders (name, description) VALUES (?, ?)",
                (name, description),
            )
            if cursor.lastrowid:
                return int(cursor.lastrowid)
            else:
                raise ValueError("Failed to insert folder into database")

    def get_all_folders(self) -> List[Dict]:
        """获取所有收藏夹"""
        cursor = self.conn.execute("SELECT * FROM folders ORDER BY created_at DESC")
        return [dict(row) for row in cursor.fetchall()]

    def get_folder_words_count(self, folder_id: int) -> int:
        """获取某个收藏夹下的单词数量"""
        cursor = self.conn.execute(
            """
            SELECT COUNT(*) FROM word_favorites
            WHERE folder_id = ?
        """,
            (folder_id,),
        )
        return cursor.fetchone()[0]

    def get_all_folder_info(self) -> List[Dict]:
        """获取所有收藏夹信息"""
        folders = self.get_all_folders()
        for folder in folders:
            folder["words_count"] = self.get_folder_words_count(folder["id"])
        return folders

    # ======================== 收藏/取消收藏单词 ========================
    def favorite_word(self, word: str, folder_id: int) -> bool:
        """收藏单词到指定文件夹"""
        try:
            word_id = self.get_or_create_word(word)
            with self.conn:
                self.conn.execute(
                    "INSERT INTO word_favorites (word_id, folder_id) VALUES (?, ?)",
                    (word_id, folder_id),
                )
            return True
        except sqlite3.IntegrityError:
            # 已收藏
            return False

    def unfavorite_word(self, word: str, folder_id: int) -> bool:
        """取消收藏单词"""
        word = word.strip().lower()
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute(
                """
                DELETE FROM word_favorites
                WHERE word_id = (SELECT id FROM words WHERE word = ?) AND folder_id = ?
            """,
                (word, folder_id),
            )
            return cursor.rowcount > 0

    def is_word_favorited(self, word: str, folder_id: int) -> bool:
        """检查单词是否已收藏"""
        word = word.strip().lower()
        cursor = self.conn.execute(
            """
            SELECT 1 FROM word_favorites
            WHERE word_id = (SELECT id FROM words WHERE word = ?) AND folder_id = ?
        """,
            (word, folder_id),
        )
        return cursor.fetchone() is not None

    # ======================== 查询收藏列表、历史记录 ========================

    def get_folder_id_by_name(self, folder_name: str) -> Optional[int]:
        """根据收藏夹名称获取收藏夹ID"""
        cursor = self.conn.execute(
            "SELECT id FROM folders WHERE name = ?", (folder_name,)
        )
        folder = cursor.fetchone()
        return None if not folder else folder[0]

    def get_folder_words_by_name(self, folder_name: str) -> List[Dict]:
        """根据收藏夹名称获取收藏夹下的所有单词"""
        folder_id = self.get_folder_id_by_name(folder_name)
        if folder_id is None:
            return []
        return self.get_folder_words(folder_id)

    def get_folder_words(self, folder_id: int) -> List[Dict]:
        """获取某个收藏夹下的所有单词"""
        cursor = self.conn.execute(
            """
            SELECT w.word, w.created_at, w.query_count, wf.created_at as favorited_at
            FROM word_favorites wf
            JOIN words w ON wf.word_id = w.id
            WHERE wf.folder_id = ?
            ORDER BY wf.created_at DESC
        """,
            (folder_id,),
        )
        return [dict(row) for row in cursor.fetchall()]

    def get_search_history(self, limit: int = 100) -> List[Dict]:
        """获取最近查询的单词记录"""
        cursor = self.conn.execute(
            """
            SELECT DISTINCT w.word, w.query_count, MAX(h.searched_at) as last_searched
            FROM word_search_history h
            JOIN words w ON h.word_id = w.id
            GROUP BY w.id
            ORDER BY last_searched DESC
            LIMIT ?
        """,
            (limit,),
        )
        return [dict(row) for row in cursor.fetchall()]

    def get_hot_words(self, limit: int = 50) -> List[Dict]:
        """获取查询次数最多的热词"""
        cursor = self.conn.execute(
            """
            SELECT word, query_count, created_at
            FROM words
            WHERE query_count > 0
            ORDER BY query_count DESC
            LIMIT ?
        """,
            (limit,),
        )
        return [dict(row) for row in cursor.fetchall()]

    def clear_search_history(self) -> None:
        """清空查询历史"""
        with self.conn:
            self.conn.execute("DELETE FROM word_search_history")

    # ======================== 收藏夹管理（新增：重命名、删除） ========================
    def rename_folder(self, folder_id: int, new_name: str) -> bool:
        """
        收藏夹重命名
        返回：是否成功
        """
        try:
            with self.conn:
                cursor = self.conn.cursor()
                cursor.execute(
                    "UPDATE folders SET name = ? WHERE id = ?",
                    (new_name.strip(), folder_id),
                )
            return cursor.rowcount > 0
        except sqlite3.IntegrityError:
            # 名称重复
            return False

    def update_folder_description(self, folder_id: int, description: str) -> bool:
        """修改收藏夹描述"""
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute(
                "UPDATE folders SET description = ? WHERE id = ?",
                (description, folder_id),
            )
        return cursor.rowcount > 0

    def delete_folder(self, folder_id: int) -> bool:
        """
        删除收藏夹（会自动删除里面所有收藏关系，外键级联）
        返回：是否成功
        """
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute("DELETE FROM folders WHERE id = ?", (folder_id,))
            return cursor.rowcount > 0

    def get_folder_detail(self, folder_id: int) -> Optional[Dict]:
        """获取单个收藏夹详情"""
        cursor = self.conn.execute(
            """
            SELECT * FROM folders WHERE id = ?
        """,
            (folder_id,),
        )
        row = cursor.fetchone()
        return dict(row) if row else None

    # ======================== 单词笔记功能（新增） ========================
    def save_word_note(self, word: str, note_content: str) -> bool:
        """
        保存/更新单词笔记（一个单词只能有一条笔记，重复会自动覆盖）
        """
        word_id = self.get_or_create_word(word)
        try:
            with self.conn:
                # INSERT OR REPLACE：存在就更新，不存在就插入
                self.conn.execute(
                    """
                    INSERT OR REPLACE INTO word_notes 
                    (word_id, content, updated_at) 
                    VALUES (?, ?, datetime('now','localtime'))
                """,
                    (word_id, note_content.strip()),
                )
            return True
        except Exception as e:
            logger.error(f"保存笔记失败: {e}")
            return False

    def get_word_note(self, word: str) -> Optional[str]:
        """获取单词的笔记，没有则返回 None"""
        word = word.strip().lower()
        cursor = self.conn.execute(
            """
            SELECT n.content 
            FROM word_notes n
            JOIN words w ON n.word_id = w.id
            WHERE w.word = ?
        """,
            (word,),
        )
        row = cursor.fetchone()
        return row["content"] if row else None

    def delete_word_note(self, word: str) -> bool:
        """删除单词笔记"""
        word = word.strip().lower()
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute(
                """
                DELETE FROM word_notes
                WHERE word_id = (SELECT id FROM words WHERE word = ?)
            """,
                (word,),
            )
            return cursor.rowcount > 0

    def has_word_note(self, word: str) -> bool:
        """检查单词是否有笔记"""
        return self.get_word_note(word) is not None

    def get_all_notes(self, limit: int = 200) -> List[Dict]:
        """获取所有带笔记的单词"""
        cursor = self.conn.execute(
            """
            SELECT w.word, n.content, n.updated_at
            FROM word_notes n
            JOIN words w ON n.word_id = w.id
            ORDER BY n.updated_at DESC
            LIMIT ?
        """,
            (limit,),
        )
        return [dict(row) for row in cursor.fetchall()]
