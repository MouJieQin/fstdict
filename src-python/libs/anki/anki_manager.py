# ========== Anki 管理器 ==========
import hashlib
import json
import os
import asyncio
from typing import Callable

from libs.config import UtilsBase
from libs.anki.anki_api import AnkiApi
from libs.log_config import logger

class AnkiManager:


    html_back_content_prefix = """
    <!DOCTYPE html>
    <html>
        <head>
            <meta charset="utf-8">
            <style>
                body { margin: 0; padding: 0; }
                iframe { width: 100%; height: 85vh; border: none;  }
            </style>
        </head>
        <body>
    """

    def __init__(self):
        self.anki_api = AnkiApi()
        self.prefix_div='<div id="'
        self.suffix_div='</div>'
        self.__reload_anki_config()

    def _sync_anki_config(self):
        with open(UtilsBase.ANKI_CONFIG_FILE, mode="w", encoding="utf-8") as f:
            f.write(json.dumps(self.anki_config, ensure_ascii=False, indent=4))

    def __reload_anki_config(self):
        if os.path.exists(UtilsBase.ANKI_CONFIG_FILE):
            with open(UtilsBase.ANKI_CONFIG_FILE, "r") as f:
                self.anki_config = json.load(f)
        else:
            self.anki_config = {
                "format":{
                    "default":{
                        "front":'<p style="font-size: 32px; font-weight: bold;">{keyword}</p>',
                    }
                }
            }
            self._sync_anki_config()

    @staticmethod
    def get_str_unique_id(text: str) -> str:
        return hashlib.md5(text.strip().encode("utf-8")).hexdigest()
    
    def get_unique_id_from_front(self, front: str) -> str:
        start_index=front.find(self.prefix_div)
        end_index=front.find('">', start_index)
        if start_index==-1 or end_index==-1:
            return ""
        return front[start_index+len(self.prefix_div):end_index]
        
    def get_deck_cards_indexed_by_unique_id(self, deck_name: str) -> dict:
        cards = self.anki_api.get_deck_cards_info(deck_name)
        result = {}
        for card in cards:
            unique_id = self.get_unique_id_from_front(card["front"])
            result[unique_id] = card
        return result
    
    def set_cancel_flag(self, cancel: bool):
        self.cancel_flag = cancel

    def is_cancel_flag(self) -> bool:
        return self.cancel_flag

    async def update_words_to_anki(self, session_id: str, deck_name: str, words: list, send_progress: Callable):
        """
        更新 Anki 中的单词
        :param deck_name: Anki 牌名
        :param words: 要更新的单词列表
        """
        if self.is_cancel_flag():
            msg = {"type": "canceled"}
            await send_progress(msg)
            return
        
        self.__reload_anki_config()
        deck_format_config = self.anki_config["format"].get(deck_name, {})
        deck_front_format_config = deck_format_config.get("front", "")
        front_format_str =  deck_front_format_config or self.anki_config["format"]["default"]["front"]
        msg = {"type": "trying_acquiring_cards_from_anki"}
        await send_progress(msg)


        # ✅ 异步队列：线程往里扔进度，async 往外发消息
        import queue
        q = queue.Queue()
        def sync_task():
            logger.debug("sync_task 开始")
            deck_cards = {}
            try:
                deck_cards = self.get_deck_cards_indexed_by_unique_id(deck_name)
                logger.debug(f"获取到 {len(deck_cards)} 张 Anki 卡片")
            except Exception as e:
                q.put(("error","获取 Anki 卡片失败！请先运行Anki并确认安装AnkiConnect插件！"))
                logger.error(e)
                return
            count = total_count = updated_count = update_error_count = created_count = create_error_count = 0
            total_count = len(words)
            try:
                for word in words:
                    if self.is_cancel_flag():
                        q.put(("cancel",))
                        break
                    logger.debug(f"处理单词：{word['word']}")
                    unique_id = self.get_str_unique_id(word['word'])
                    front_first_line=self.prefix_div + unique_id +'">'
                    front_last_line =self.suffix_div
                    front_content= front_format_str.format(keyword=word['word'])
                    front=f"{front_first_line}\n{front_content}\n{front_last_line}"

                    back = self.html_back_content_prefix
                    back += f"<iframe src=\"http://localhost:9595/#/dict/{session_id}?keyword={word['word']}&env=anki\"></iframe>\n"
                    back += "</body>\n</html>"

                    note_id = deck_cards[unique_id]["noteId"] if unique_id in deck_cards else None
                    success, res_msg = self.anki_api.update_note_to_deck(deck_name, note_id, front, back)

                    count += 1
                    if success:
                        if note_id:
                            updated_count += 1
                        else:
                            created_count += 1
                    else:
                        if note_id:
                            update_error_count += 1
                        else:
                            create_error_count += 1

                    if count % 10 == 0:
                        q.put(("progress", count, total_count, updated_count, created_count, update_error_count, create_error_count))
                q.put(("done", count, total_count, updated_count, created_count, update_error_count, create_error_count))
            except Exception as e:
                q.put(("error", "更新 Anki 卡片失败!"))
                logger.error(e)
                return

        logger.debug(f"开始task处理")
        # task_update = asyncio.create_task(asyncio.to_thread(sync_task))
        import threading
        task_update = threading.Thread(
            target=sync_task,
            args=(),
            daemon=True,
        )
        task_update.start()
        logger.debug(f"task处理开始")

        # ✅ 异步循环：实时发进度（不阻塞）
        async def process_messages():
            while True:
                try:
                    await asyncio.sleep(0.05)
                    msg = q.get(block=False)
                    logger.debug(msg)
                except:
                    if not task_update.is_alive():
                        break
                    continue

                if msg[0] == "cancel":
                    await send_progress({"type": "canceled"})
                    return
                elif msg[0] == "progress":
                    await send_progress({
                        "type": "updating_cards",
                        "data": {
                            "count": msg[1],
                            "total_count": msg[2],
                            "updated_count": msg[3],
                            "created_count": msg[4],
                            "update_error_count": msg[5],
                            "create_error_count": msg[6],
                        }
                    })
                elif msg[0] == "done":
                    await send_progress({
                        "type": "done",
                        "data": {
                            "count": msg[1],
                            "total_count": msg[2],
                            "updated_count": msg[3],
                            "created_count": msg[4],
                            "update_error_count": msg[5],
                            "create_error_count": msg[6],
                        }
                    })
                    break
                elif msg[0] == "error":
                    await send_progress({"type": "error","data":{"error_message":msg[1]}})
                    break
        task_messages = asyncio.create_task(process_messages())