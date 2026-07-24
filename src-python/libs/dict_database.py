import sqlite3
import json
from typing import Optional, List, Dict, Any


class DictDatabase:
    def __init__(self, db_path: str):
        self.conn = sqlite3.connect(db_path)
        self.conn.row_factory = sqlite3.Row  # 使用字典形式返回结果
        self._setup_foreign_keys()
        self._create_tables()

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
        self._create_krdict_table()

    def _create_krdict_table(self) -> None:
        """创建 krdict 表"""
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute(
                """
                CREATE TABLE IF NOT EXISTS krdict (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    word TEXT NOT NULL,
                    definition JSON TEXT
                )
            """
            )
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_word ON krdict(word)")

    def _delete_table(self, table_name: str) -> None:
        """删除指定表"""
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute(f"DROP TABLE IF EXISTS {table_name}")
            self.conn.commit()

    def close(self) -> None:
        self.conn.close()

    def batch_insert_krdict(self, dict_json_data: List[Dict[str, Any]]) -> None:
        """批量插入单词"""
        with self.conn:
            cursor = self.conn.cursor()
            cursor.executemany(
                f"INSERT OR IGNORE INTO krdict (word, definition) VALUES (?, ?)",
                [
                    (item["word"], json.dumps(item["definition"], ensure_ascii=False))
                    for item in dict_json_data
                ],
            )
            self.conn.commit()

    def recreate_krdict_table(self, dict_json_data: List[Dict[str, Any]]) -> None:
        """重新创建 krdict 表并批量插入单词"""
        self._delete_table("krdict")
        self._create_krdict_table()
        self.batch_insert_krdict(dict_json_data)

    def query(self, word: str) -> Optional[Dict[str, Any]]:
        """查询单词定义"""
        with self.conn:
            cursor = self.conn.cursor()
            cursor.execute("SELECT definition FROM krdict WHERE word = ?", (word,))
            result = cursor.fetchone()
            if result:
                return json.loads(result["definition"])
            return None
