import json
import urllib.request


class AnkiApi:

    @staticmethod
    def invoke_anki(action, **params):
        """调用 AnkiConnect API 的通用函数"""
        request_data = json.dumps(
            {"action": action, "version": 6, "params": params}
        ).encode("utf-8")

        request = urllib.request.Request(
            "http://localhost:8765",
            data=request_data,
            headers={"Content-Type": "application/json"},
        )

        with urllib.request.urlopen(request) as response:
            result = json.load(response)
            if result.get("error"):
                raise Exception(f"Anki 错误：{result['error']}")
            return result.get("result")


    @staticmethod
    def get_deck_cards_info(deck_name):
        """
        获取指定卡组中所有卡片的 front、back、cardId、noteId
        返回：列表，每个元素是字典，包含四个字段
        """
        # 1. 获取卡组所有卡片 ID
        card_ids = AnkiApi.invoke_anki("findCards", query=f'deck:"{deck_name}"')
        if not card_ids:
            print(f"未找到【{deck_name}】中的卡片")
            return []

        # 2. 获取卡片的详细信息（包含关联的 noteId）
        cards_info = AnkiApi.invoke_anki("cardsInfo", cards=card_ids)

        # 3. 批量获取笔记内容（front + back）
        note_ids = [card["note"] for card in cards_info]
        notes_info = AnkiApi.invoke_anki("notesInfo", notes=note_ids)

        # 4. 组装结果：front + back + cardId + noteId
        result = []
        for card, note in zip(cards_info, notes_info):
            front = note["fields"].get("Front", {}).get("value", "无正面内容")
            back = note["fields"].get("Back", {}).get("value", "无反面内容")  # 新增获取反面

            result.append({
                "cardId": card["cardId"],
                "noteId": note["noteId"],
                "front": front,
                "back": back    # 新增返回反面
            })

        return result


    @staticmethod
    def update_note_to_deck(deck_name: str, noteId=None, front: str = "", back: str = ""):
        """
        【新增函数】智能更新/插入卡片（笔记）
        - 有 noteId → 更新该笔记的 front/back（不改变学习记录）
        - 无 noteId → 新建卡片到卡组
        - 卡组不存在 → 自动创建卡组

        参数：
            deck_name: 目标卡组名
            noteId: 笔记ID（为空则新建）
            front: 卡片正面
            back: 卡片反面
        返回：
            (成功标识, 消息/ID)
        """
        # 1. 卡组不存在则自动创建
        existing_decks = AnkiApi.invoke_anki("deckNames")
        if deck_name not in existing_decks:
            AnkiApi.invoke_anki("createDeck", deck=deck_name)
            print(f"✅ 自动创建卡组：{deck_name}")

        # 2. 更新已有笔记（有 noteId）
        if noteId:
            # 先获取原笔记，对比是否需要更新
            try:
                note_info = AnkiApi.invoke_anki("notesInfo", notes=[noteId])[0]
                old_front = note_info["fields"].get("Front", {}).get("value", "")
                old_back = note_info["fields"].get("Back", {}).get("value", "")

                if old_front == front and old_back == back:
                    return True, f"✅ 笔记 {noteId} 内容无变化，无需更新"

                # 执行更新（只改内容，不影响学习进度）
                AnkiApi.invoke_anki("updateNoteFields", note={
                    "id": noteId,
                    "fields": {
                        "Front": front,
                        "Back": back
                    }
                })
                return True, f"✅ 成功更新笔记 {noteId}"
            except Exception as e:
                return False, f"❌ 更新失败：{str(e)}"

        # 3. 没有 noteId → 新建笔记
        else:
            try:
                # Anki 默认模型：Basic（正面Front、反面Back）
                note = {
                    "deckName": deck_name,
                    "modelName": "Basic",
                    "fields": {
                        "Front": front,
                        "Back": back
                    },
                    "options": {
                        "allowDuplicate": False,
                        "duplicateScope": "deck",
                        "duplicateScopeDeckName": deck_name
                    },
                    "tags": []  # 可加标签
                }
                new_note_id = AnkiApi.invoke_anki("addNote", note=note)
                return True, f"✅ 成功新建卡片，笔记ID：{new_note_id}"
            except Exception as e:
                return False, f"❌ 新建失败：{str(e)}"


# ------------------- 测试示例 -------------------

# -------------------------- 配置项（修改这里）--------------------------
DECK_NAME = "Yonsei Korean Word 3"  # 替换成你要查询的卡组名，必须完全一致
# ---------------------------------------------------------------------


if __name__ == "__main__":
    # 测试1：获取卡组所有卡片（含正面+反面）
    print("===== 获取卡组卡片 =====")
    cards = AnkiApi.get_deck_cards_info(DECK_NAME)
    print(f"共 {len(cards)} 张卡片")
    for c in cards[:]:  # 只打印前3张
        print(f"正面：{c['front']}")
        print(f"反面：{c['back']}")
        print(f"笔记ID：{c['noteId']}")
        print("-"*30)


    #为之前的卡片添加front id
    # import hashlib
    
    # for c in cards[:]:
    #     noteId = c['noteId']
    #     front = c['front']
    #     keyword = front.replace("<p>", "").replace("</p>", "").strip()
    #     front_id= hashlib.md5(keyword.encode("utf-8")).hexdigest()
    #     front = f"<div id=\"{front_id}\">{front}</div>"
    #     back = c['back']
    #     print(f"正面：{front}", f"反面：{back}", f"笔记ID：{noteId}")
    #     AnkiApi.upsert_note_to_deck(DECK_NAME, noteId, front, back)



#     # 测试2：更新已有笔记（不会改变学习记录）
#     print("\n===== 更新笔记 =====")
#     success, msg = AnkiApi.upsert_note_to_deck(
#         deck_name=DECK_NAME,
#         noteId=1776935753891,  # 替换成真实笔记ID
#         front="新的正面",
#         back="新的反面"
#     )
#     print(msg)

#     # 测试3：新建笔记
#     print("\n===== 新建卡片 =====")
#     success, msg = AnkiApi.upsert_note_to_deck(
#         deck_name=DECK_NAME,
#         noteId=None,  # 为空=新建
#         front="测试新建正面2",
#         back="测试新建反面2"
#     )
#     print(msg)