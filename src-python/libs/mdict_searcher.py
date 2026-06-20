import os
from typing import Dict, Optional
from libs.log_config import logger
from libs.mdict_query.mdict_query import IndexBuilder
from libs.config import UtilsBase

# 导入 C++ 引擎
import fstd_engine  # 👈 这是你编译后的模块


class MdictSearcher:
    def __init__(self):
        self._all_dict_names: list[str] = []

        # self._fstd_engine = fstd_engine.FstdEngine()
        self._fstd_engine = UtilsBase.fstd_engine
        self._build_mdxs_index()

    def _build_mdxs_index(self):
        """构建所有词典索引"""
        logger.info("开始构建所有词典索引")
        is_fstd_engine_loaded_from_meta = False
        if os.path.exists(UtilsBase.FSTD_SEARCHER_META_PATH):
            logger.info("找到现有索引，直接加载")
            try:
                UtilsBase.fstd_engine = fstd_engine.FstdEngine(
                    UtilsBase.FSTD_SEARCHER_META_PATH
                )
                self._fstd_engine = UtilsBase.fstd_engine

                is_fstd_engine_loaded_from_meta = True
            except Exception as e:
                logger.error(f"加载现有索引失败，重新构建：{e}")
                is_fstd_engine_loaded_from_meta = False

            logger.info(
                f"is_fstd_engine_loaded_from_meta:{is_fstd_engine_loaded_from_meta}"
            )

        for dict_name, dict_info in UtilsBase.DICT_INFO.items():
            logger.info(f"构建 {dict_name} 索引...")

            fstdx_path = UtilsBase.DICT_INFO[dict_name]["path"]
            self._fstd_engine.add_dict_from_file(dict_name, fstdx_path)
            logger.info(f"开始导入 {dict_name} 到 C++ 引擎...")
            self._all_dict_names.append(dict_name)
        if not is_fstd_engine_loaded_from_meta:
            self._fstd_engine.build_indexes(UtilsBase.FSTDX_INDEX_PATH)
            self._fstd_engine.save_to_disk(UtilsBase.FSTD_SEARCHER_META_PATH)
        logger.info("所有词典索引构建完成")

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
            res = self._fstd_engine.look_up(keyword, dict_name)
            if res:
                result = []
                self._hand_link_word(result, res, dict_name, [keyword], ignorecase)
                results[dict_name] = result
        return results

    def _hand_link_word(
        self,
        result: list[str],
        cur_result: list[str],
        dict_name: str,
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
                    res_redirect = self._fstd_engine.look_up(redirect_word, dict_name)
                    if res_redirect:
                        self._hand_link_word(
                            result, res_redirect, dict_name, words_show, ignorecase
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
            return self._fstd_engine.prefix_distance_search(keyword, use_dicts, 6)
            # result = []
            # prefix = keyword
            # while prefix:
            #     result = self._fstd_engine.predictive_search(prefix, use_dicts, limit)
            #     if result:
            #         return result
            #     prefix = prefix[:-1]
            # return result

        elif search_method == "contains_search":
            regex_result = self._fstd_engine.regex_search(keyword, use_dicts)
            return regex_result[0]

        elif search_method == "fuzzy_search":
            return self._fstd_engine.edit_distance_search(keyword, use_dicts, 2)

        elif search_method == "fuzzy_contains_search":
            return self._fstd_engine.suggest(keyword, use_dicts)

        else:
            logger.error("无效搜索方式")
            return []
