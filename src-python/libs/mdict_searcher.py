import os
from typing import Dict, Optional
from libs.log_config import logger
from libs.mdict_query.mdict_query import IndexBuilder
from libs.config import UtilsBase

# 导入 C++ 引擎
import word_engine  # 👈 这是你编译后的模块


class MdictSearcher:
    def __init__(self):
        self._indexBuilders: Dict[str, IndexBuilder] = {}
        self._all_dict_names: list[str] = []

        # ====================== 核心变化 ======================
        # 所有单词存在 C++，Python 0 存储！
        self._word_engine = word_engine.WordStorage()

        self._build_mdxs_index()

    def _build_mdxs_index(self):
        """构建所有词典索引"""
        logger.info("开始构建所有词典索引")
        for dict_name, dict_info in UtilsBase.DICT_INFO.items():
            logger.info(f"构建 {dict_name} 索引...")

            # 构建原索引
            self._indexBuilders[dict_name] = IndexBuilder(dict_info["path"])
            key_path = UtilsBase.DICT_INFO[dict_name]["root"] + "/keys.txt"
            if not os.path.exists(key_path):
                with open(key_path, mode="w", encoding="utf-8") as f:
                    f.write("\n".join(self._indexBuilders[dict_name].get_mdx_keys()))
            # words = self._indexBuilders[dict_name].get_mdx_keys()
            # ====================== 关键 ======================
            # 单词直接传给 C++，Python 不保存！
            logger.info(f"开始导入 {dict_name} 到 C++ 引擎...")
            num_words = self._word_engine.add_dict_from_file(dict_name, key_path)
            logger.info(f"{dict_name} 导入 C++ 完成：{num_words} 个单词")
            self._all_dict_names.append(dict_name)
        logger.info(
            f"所有词典索引构建完成，共 {self._word_engine.total_words()} 个单词"
        )

    def mdx_lookup(
        self,
        keyword: str,
        dict_names: Optional[list[str]],
        ignorecase: Optional[bool] = None,
    ) -> Dict[str, Dict[str, list[str]]]:
        """查询所有词典"""
        results = {}
        if dict_names is None:
            dict_names = self._all_dict_names
        for dict_name in dict_names:
            indexBuilder = self._indexBuilders[dict_name]
            res = indexBuilder.mdx_lookup(keyword, ignorecase)
            if res:
                result = []
                self._hand_link_word(result, indexBuilder, res, [keyword], ignorecase)
                results[dict_name] = result
        return results

    def _hand_link_word(
        self,
        result: list[str],
        indexBuilder: IndexBuilder,
        cur_result: list[str],
        words_show: list[str],
        ignorecase: Optional[bool] = None,
    ):
        """处理重定向单词"""
        for i in range(len(cur_result)):
            item = cur_result[i]
            if "@@@LINK=" not in item:
                result.append(item)
            else:
                redirect_word = item.split("@@@LINK=")[1].strip()
                if redirect_word not in words_show:
                    words_show.append(redirect_word)
                    res_redirect = indexBuilder.mdx_lookup(redirect_word, ignorecase)
                    if res_redirect:
                        self._hand_link_word(
                            result, indexBuilder, res_redirect, words_show, ignorecase
                        )

    def keyword_options_search(
        self,
        keyword: str,
        search_method: str,
        dict_names: Optional[list[str]] = None,
        limit=20,
    ):
        """
        🔥🔥🔥 所有搜索全部在 C++ 完成
        0 数据拷贝、0 Python 遍历
        """
        # 1. 切换用户选择的词典（瞬间完成，不复制数据）
        use_dicts = dict_names or self._all_dict_names
        # self._word_engine.set_active_dicts(use_dicts)

        # 2. 直接调用 C++ 搜索
        if search_method == "prefix_search":
            return self._word_engine.prefix_search(keyword, use_dicts, limit)

        elif search_method == "contains_search":
            return self._word_engine.contains_search(keyword, use_dicts, limit)

        elif search_method == "fuzzy_search":
            return self._word_engine.fuzzy_search(keyword, use_dicts, limit)

        elif search_method == "fuzzy_contains_search":
            return self._word_engine.fuzzy_contains_search(keyword, use_dicts, limit)

        else:
            logger.error("无效搜索方式")
            return []

    # @staticmethod
    # def prefix_search(words_sorted: list[str], keyword: str, limit=20):
    #     """前缀匹配：以 xxx 开头"""
    #     idx = bisect.bisect_left(words_sorted, keyword)
    #     result = []

    #     for word in words_sorted[idx:]:
    #         if word.startswith(keyword):
    #             result.append(word)
    #             if len(result) >= limit:
    #                 break
    #         else:
    #             break  # 排序后不匹配就直接退出，超快

    #     return result

    # @staticmethod
    # def contains_search(words: list[str], keyword: str, limit=20):
    #     """包含匹配：单词里含有 xxx"""
    #     result = []
    #     keyword = keyword.lower()  # 不区分大小写

    #     for word in words:
    #         if keyword in word.lower():
    #             result.append(word)
    #             if len(result) >= limit:
    #                 break

    #     return result

    # @staticmethod
    # def fuzzy_search(words: list[str], keyword: str, limit=20):
    #     """
    #     模糊搜索：找最相似的前 N 个
    #     越小越像：0=完全一样
    #     """
    #     # 用堆取 TopN，比全排序快 10 倍以上
    #     heap = []
    #     keyword = keyword.lower()

    #     for word in words:
    #         word_lower = word.lower()
    #         dist = distance(keyword, word_lower)

    #         # 推入堆（距离，单词）
    #         heapq.heappush(heap, (dist, word))

    #     # 取出前 limit 个
    #     result = [word for _, word in heapq.nsmallest(limit, heap)]
    #     return result

    # @staticmethod
    # def fuzzy_contains_search(words: list[str], keyword: str, limit=20):
    #     """
    #     模糊包含搜索：找最相似的前 N 个
    #     """
    #     # 用堆取 TopN，比全排序快 10 倍以上
    #     heap = []
    #     keyword = keyword.lower()

    #     for word in words:
    #         word_lower = word.lower()
    #         if keyword not in word_lower:
    #             continue
    #         dist = distance(keyword, word_lower)

    #         # 推入堆（距离，单词）
    #         heapq.heappush(heap, (dist, word))

    #     # 取出前 limit 个
    #     result = [word for _, word in heapq.nsmallest(limit, heap)]
    #     return result
