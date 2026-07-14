import time
import os
from typing import Dict, Optional
from libs.log_config import logger
from libs.config import UtilsBase
from pathlib import Path
import fstd
import fstdtools


class FstDictSearcher:
    def __init__(self):
        self._all_dict_names: list[str] = []

        self._fstd_engine = UtilsBase.fstd_engine
        self._load_fstdx()

    def _load_fstdx(self):
        """构建所有词典索引"""
        logger.info("开始构建所有词典索引")
        for dict_name, dict_info in UtilsBase.DICT_INFO.items():
            logger.info(f"加载 {dict_name} ...")
            fstdx_path = UtilsBase.DICT_INFO[dict_name]["path"]
            logger.info(f"开始导入 {dict_name} 到 fstd 引擎...")
            self._load_dict(dict_name, fstdx_path)
        logger.info("所有词典加载完成")
        if "prior_suffix" in UtilsBase.CONFIG:
            self._prior_suffix = UtilsBase.CONFIG["prior_suffix"]
            logger.info(f"prior_suffix: {self._prior_suffix}")
            for _, value in self._prior_suffix.items():
                self._fstd_engine.insert_prior_suffix(value)

    def _load_dict(self, dict_name: str, dict_path: str):
        """加载词典"""
        self._fstd_engine.insert_if_not_exists(dict_name, dict_path)
        self._all_dict_names.append(dict_name)

    def remove_dictionary(self, dict_name: str):
        """删除词典"""
        # self._fstd_engine.remove(dict_name)
        self._all_dict_names.remove(dict_name)

    def _add_dictionary_from_dir(self, dict_path_str: str):
        dict_path = Path(dict_path_str)
        css = []
        js = []
        fstdx = ""
        fstdd = []
        mdx = ""
        mdd = []
        cover = ""
        msgs = []
        # 获取所有文件
        for file in dict_path.iterdir():
            if file.is_file():
                if file.suffix == ".fstdx":
                    if not fstdx:
                        fstdx = str(file.absolute())
                    else:
                        logger.error(f"词典路径 {dict_path_str} 下有多个 fstdx 文件")
                        return [{"msg": f"词典路径 {dict_path_str} 下有多个 fstdx 文件", "type": "error"}]
                elif file.suffix == ".fstdd":
                    fstdd.append(str(file.absolute()))
                elif file.suffix == ".mdx":
                    if not mdx:
                        mdx = str(file.absolute())
                    else:
                        logger.error(f"词典路径 {dict_path_str} 下有多个 mdx 文件")
                        return [{"msg": f"词典路径 {dict_path_str} 下有多个 mdx 文件", "type": "error"}]
                elif file.suffix == ".mdd":
                    mdd.append(str(file.absolute()))
                elif file.suffix == ".css":
                    css.append(str(file.absolute()))
                elif file.suffix == ".js":
                    js.append(str(file.absolute()))
                elif file.suffix == ".jpg" or file.suffix == ".jpeg" or file.suffix == ".png" or file.suffix == ".gif":
                    cover = str(file.absolute())
        if fstdx and mdx:
            logger.error(f"词典路径 {dict_path_str} 下同时存在 fstdx 文件和 mdx 文件")
            return [{"msg": f"词典路径 {dict_path_str} 下同时存在 fstdx 文件和 mdx 文件", "type": "error"}]
        if fstdx:
            dict_name = Path(fstdx).stem
            dict_dir = UtilsBase.getDictDir(dict_name)
            fstdx_path = os.path.join(dict_dir, dict_name + ".fstdx")
            if os.path.exists(fstdx_path):
                reader = fstd.FstdxReader(fstdx_path)
                if reader:
                    logger.warning(f"词典 {dict_name} 已存在，跳过")
                    return [{"msg": f"词典 {dict_name} 已存在，跳过", "type": "warning"}]
            UtilsBase.createDirIfnotExists(dict_dir)
            UtilsBase.copyFile(fstdx, dict_dir)
            UtilsBase.copyFile(cover, dict_dir)
            for item in css:
                UtilsBase.copyFile(item, dict_dir)
            for item in js:
                UtilsBase.copyFile(item, dict_dir)
            for item in fstdd:
                UtilsBase.copyFile(item, dict_dir)
            for item in mdd:
                output_path = os.path.join(dict_dir, Path(item).stem + ".fstdd")
                ret = fstdtools.convert(item, output_path, compress_level=5, compress_dict_size=130, block_size=32)
                if ret != 0:
                    logger.error(f"转换词典 {item} 失败")
                    msgs.append({"msg": f"转换词典 {item} 失败", "type": "error"})
            UtilsBase.Config.checkDictInfo(Path(dict_dir))
            self._load_dict(dict_name, fstdx_path)
            msgs.append({"msg": f"词典 {dict_name} 添加成功", "type": "success"})
            return msgs
        if mdx:
            dict_name = Path(mdx).stem
            dict_dir = UtilsBase.getDictDir(dict_name)
            fstdx_path = os.path.join(dict_dir, dict_name + ".fstdx")
            if os.path.exists(fstdx_path):
                reader = fstd.FstdxReader(fstdx_path)
                if reader:
                    logger.warning(f"词典 {dict_name} 已存在，跳过")
                    return [{"msg": f"词典 {dict_name} 已存在，跳过", "type": "warning"}]
            UtilsBase.createDirIfnotExists(dict_dir)
            ret = fstdtools.convert(mdx, fstdx_path, compress_level=5, compress_dict_size=130, block_size=32)
            if ret != 0:
                logger.error(f"转换词典 {mdx} 失败")
                return [{"msg": f"转换词典 {mdx} 失败", "type": "error"}]
            for item in mdd:
                output_path = os.path.join(dict_dir, Path(item).stem + ".fstdd")
                ret = fstdtools.convert(item, output_path, compress_level=5, compress_dict_size=130, block_size=32)
                if ret != 0:
                    logger.error(f"转换词典 {item} 失败")
                    msgs.append({"msg": f"转换词典 {item} 失败", "type": "error"})
            for item in fstdd:
                UtilsBase.copyFile(item, dict_dir)
            UtilsBase.copyFile(cover, dict_dir)
            for item in css:
                UtilsBase.copyFile(item, dict_dir)
            for item in js:
                UtilsBase.copyFile(item, dict_dir)
            UtilsBase.Config.checkDictInfo(Path(dict_dir))
            self._load_dict(dict_name, fstdx_path)
            msgs.append({"msg": f"词典 {dict_name} 添加成功", "type": "success"})
            return msgs
        return msgs

    def _add_dictionary_from_dir_2_depth(self, dir: Path):
        """从目录添加词典"""
        msgs = []
        for item in dir.iterdir():
            if item.is_dir():
                msgs_ = self._add_dictionary_from_dir(str(item.absolute()))
        msgs_ = self._add_dictionary_from_dir(str(dir.absolute()))
        msgs.extend(msgs_)
        return msgs

    def add_dictionary(self, dict_path_str: str):
        dict = Path(dict_path_str)
        if not dict.exists():
            logger.error(f"词典路径 {dict_path_str} 不存在")
            return [{"msg": f"词典路径 {dict_path_str} 不存在", "type": "error"}]
        if dict.is_file():
            parent = dict.parent
            return self._add_dictionary_from_dir_2_depth(parent)
        elif dict.is_dir():
            return self._add_dictionary_from_dir_2_depth(dict)
        else:
            logger.error(f"词典路径 {dict_path_str} 不是文件或目录")
            return [{"msg": f"词典路径 {dict_path_str} 不是文件或目录", "type": "error"}]

    def lookup(
        self,
        keyword: str,
        dict_names: Optional[list[str]],
        ignorecase: Optional[bool] = None,
    ) -> Dict[str, Dict[str, list[str]]]:
        """查询所有词典"""
        results = self._lookup_imple(keyword, dict_names)
        if results:
            return results
        use_dicts = dict_names or self._all_dict_names
        regex_result = self._fstd_engine.regex_search(f'(?i){keyword}$', use_dicts)
        if regex_result[1] or not regex_result[0]:
            return results
        return self._lookup_imple(regex_result[0][0], dict_names)

    def _lookup_imple(
        self,
        keyword: str,
        dict_names: Optional[list[str]],
    ) -> Dict[str, Dict[str, list[str]]]:
        results = {}
        if dict_names is None:
            dict_names = self._all_dict_names
        for dict_name in dict_names:
            res = self._fstd_engine.exact_match_search(keyword, dict_name)
            if res:
                result = []
                self._hand_link_word(result, res, dict_name, [keyword])
                results[dict_name] = result
        return results

    def _hand_link_word(
        self,
        result: list[str],
        cur_result: list[str],
        dict_name: str,
        words_show: list[str]
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
                    res_redirect = self._fstd_engine.exact_match_search(redirect_word, dict_name)
                    if res_redirect:
                        self._hand_link_word(result, res_redirect, dict_name, words_show)

    def keyword_options_search(
        self,
        keyword: str,
        search_method: str,
        dict_names: Optional[list[str]] = None,
        limit=20,
    ):
        use_dicts = dict_names or self._all_dict_names
        if search_method == "prefix_search":
            # calculate the time cost of predictive_search
            start_time = time.time()
            result = []
            prefix = keyword
            # while prefix:
            #     result = self._fstd_engine.predictive_search(prefix, use_dicts)
            #     if result:
            #         return result
            #     prefix = prefix[:-1]
            result = self._fstd_engine.predictive_search(prefix, use_dicts)
            end_time = time.time()
            logger.info(f"predictive_search time cost: {end_time - start_time}")
            return result

        elif search_method == "regex_search":
            regex_result = self._fstd_engine.regex_search(keyword, use_dicts)
            if regex_result[1]:
                return [f"FSTD_ERROR{regex_result[1]}"]
            return regex_result[0]

        elif search_method == "prefix_distance_search":
            return self._fstd_engine.prefix_distance_search(keyword, use_dicts, 3)

            # return self._fstd_engine.edit_distance_search(keyword, use_dicts, 2)

        elif search_method == "suggest_search":
            return self._fstd_engine.suggest(keyword, use_dicts)

        else:
            logger.error("无效搜索方式")
            return []
