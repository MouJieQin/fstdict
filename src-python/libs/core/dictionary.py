"""
Dictionary search engine wrapper around the fstd library.
"""
import time
import os
from pathlib import Path
from typing import Dict, Optional, List, Callable
import fstd
import fstdtools

from libs.log_config import logger
from libs.common.utils import Utils


class DictionarySearcher:
    """Manages dictionary loading and provides search operations."""

    def __init__(self):
        self._dict_names: List[str] = []
        self._engine = Utils.fstd_engine
        self._load_all_dictionaries()

    def _load_all_dictionaries(self) -> None:
        """Load all available dictionaries into the search engine."""
        logger.info("Loading all dictionaries")
        for dict_name, dict_info in Utils.DICT_INFO.items():
            logger.info(f"Loading dictionary: {dict_name}")
            fstdx_path = dict_info["path"]
            self._load_dictionary(dict_name, fstdx_path)
        logger.info(f"All dictionaries ({len(Utils.DICT_INFO)}) loaded")
        self.reload_priority_suffixes()

    def _load_dictionary(self, name: str, path: str) -> None:
        """Load a single dictionary into the engine."""
        self._engine.insert_if_not_exists(name, path)
        self._dict_names.append(name)

    def remove_dictionary(self, name: str) -> None:
        """Remove a dictionary from the engine."""
        self._engine.erase(name)
        if name in self._dict_names:
            self._dict_names.remove(name)

    def reload_dictionary(self, name: str) -> None:
        """Reload a dictionary from disk."""
        Utils.Config.removeDictInfo(name)
        self._engine.erase(name)

        dict_dir = Utils.getDictDir(name)
        Utils.Config.checkDictInfo(Path(dict_dir))

        fstdx_path = os.path.join(dict_dir, f"{name}.fstdx")
        self._engine.insert_if_not_exists(name, fstdx_path)

        if name not in self._dict_names:
            self._dict_names.append(name)

    def reload_priority_suffixes(self) -> None:
        """Reload priority suffix configuration."""
        prior_suffix = Utils.CONFIG["app"]["prior_suffix"]
        self._engine.remove_all_prior_suffix()
        for _, value in prior_suffix.items():
            self._engine.insert_prior_suffix(value)

    def lookup(
        self,
        keyword: str,
        dict_names: Optional[List[str]] = None,
        ignorecase: Optional[bool] = None,
    ) -> Dict[str, Dict[str, List[str]]]:
        """
        Perform an exact match lookup across dictionaries.
        Automatically follows @@@LINK redirects.
        """
        results = self._exact_lookup(keyword, dict_names)
        if results:
            return results

        use_dicts = dict_names or self._dict_names

        # Fallback: case-insensitive regex match
        regex_result = self._engine.regex_search(f'(?i){keyword}$', use_dicts)
        if regex_result[1] or not regex_result[0]:
            return results

        return self._exact_lookup(regex_result[0][0], dict_names)

    def _exact_lookup(
        self, keyword: str, dict_names: Optional[List[str]]
    ) -> Dict[str, Dict[str, List[str]]]:
        """Internal exact match lookup with redirect resolution."""
        results = {}
        target_dicts = dict_names or self._dict_names

        for dict_name in target_dicts:
            res = self._engine.exact_match_search(keyword, dict_name)
            if res:
                resolved = []
                self._resolve_redirects(resolved, res, dict_name, [keyword])
                results[dict_name] = resolved

        return results

    def _resolve_redirects(
        self,
        result: List[str],
        current: List[str],
        dict_name: str,
        visited: List[str]
    ) -> None:
        """Recursively resolve @@@LINK= redirect entries."""
        for item in current:
            if "@@@LINK=" not in item:
                result.append(item)
            else:
                redirect_word = item.split("@@@LINK=")[1].strip()
                if redirect_word not in visited:
                    visited.append(redirect_word)
                    redirect_result = self._engine.exact_match_search(redirect_word, dict_name)
                    if redirect_result:
                        self._resolve_redirects(result, redirect_result, dict_name, visited)

    def keyword_suggestions(
        self,
        keyword: str,
        search_method: str,
        dict_names: List[str],
        limit: int = 20,
    ) -> List[str]:
        """Get keyword suggestions based on search method."""
        use_dicts = dict_names

        if search_method == "prefix_search":
            start_time = time.time()
            result = self._engine.predictive_search(keyword, use_dicts)
            elapsed = time.time() - start_time
            logger.debug(f"Prefix search completed in {elapsed:.4f}s")
            return result

        elif search_method == "regex_search":
            regex_result = self._engine.regex_search(keyword, use_dicts)
            if regex_result[1]:
                return [f"FSTD_ERROR{regex_result[1]}"]
            return regex_result[0]

        elif search_method == "prefix_distance_search":
            return self._engine.prefix_distance_search(keyword, use_dicts, 3)

        elif search_method == "suggest_search":
            return self._engine.suggest(keyword, use_dicts)

        else:
            logger.error(f"Invalid search method: {search_method}")
            return []

    async def add_dictionary(
        self, dict_path_str: str, send_progress: Callable
    ) -> None:
        """Add a new dictionary from a file or directory path."""
        dict_path = Path(dict_path_str)

        if not dict_path.exists():
            logger.error(f"Dictionary path does not exist: {dict_path_str}")
            await send_progress({"msg": f"Dictionary path does not exist: `{dict_path_str}`", "type": "error"})
            return

        if dict_path.is_file():
            parent = dict_path.parent
            await self._add_from_directory_depth2(parent, send_progress)
        elif dict_path.is_dir():
            await self._add_from_directory_depth2(dict_path, send_progress)
        else:
            logger.error(f"Path is neither a file nor a directory: {dict_path_str}")
            await send_progress({"msg": "Invalid dictionary path", "type": "error"})
            return

        Utils.Config.renew_dict_set_options()

    async def _add_from_directory_depth2(self, directory: Path, send_progress: Callable) -> None:
        """Scan directory and subdirectories for dictionary files."""
        for item in directory.iterdir():
            if item.is_dir():
                await self._add_from_directory(str(item.absolute()), send_progress)
        await self._add_from_directory(str(directory.absolute()), send_progress)

    async def _copy_file(self, file: str, reload_dict_names: list[str], send_progress: Callable) -> None:
        dict_name = Path(file).stem.split('.')[0]
        if not dict_name:
            return
        dict_dir = Utils.getDictDir(dict_name)
        if os.path.exists(dict_dir):
            target_path = os.path.join(dict_dir, Path(file).name)
            if os.path.exists(target_path):
                await send_progress({"msg": f"File `{target_path}` already exists, skip `{file}`", "type": "warning"})
                return
            await send_progress({"type": "info", "msg": f"Copying `{file}` to `{dict_dir}` ..."})
            Utils.copyFile(file, dict_dir)
            reload_dict_names.append(dict_name)
            return
        await send_progress({"msg": f"Dictionary `{dict_name}` does not exist, skip `{file}`", "type": "warning"})

    async def _add_from_directory(self, dict_path_str: str, send_progress: Callable) -> None:
        dict_path = Path(dict_path_str)
        css = []
        js = []
        fstdx = []
        fstdd = []
        mdx = []
        mdd = []
        cover = []
        # Scan all files inside directory
        for file in dict_path.iterdir():
            if file.is_file():
                if file.suffix == ".fstdx":
                    fstdx.append(str(file.absolute()))
                elif file.suffix == ".fstdd":
                    fstdd.append(str(file.absolute()))
                elif file.suffix == ".mdx":
                    mdx.append(str(file.absolute()))
                elif file.suffix == ".mdd":
                    mdd.append(str(file.absolute()))
                elif file.suffix == ".css":
                    css.append(str(file.absolute()))
                elif file.suffix == ".js":
                    js.append(str(file.absolute()))
                elif file.suffix in (".jpg", ".jpeg", ".png", ".gif"):
                    cover.append(str(file.absolute()))

        new_dict_names = []
        for fstdx_ in fstdx:
            dict_name = Path(fstdx_).stem
            dict_dir = Utils.getDictDir(dict_name)
            fstdx_path = os.path.join(dict_dir, dict_name + ".fstdx")
            if os.path.exists(fstdx_path):
                reader = fstd.FstdxReader(fstdx_path)
                if reader:
                    logger.warning(f"Dictionary {dict_name} already exists, skip {fstdx_}")
                    await send_progress({"type": "warning", "msg": f"Dictionary `{dict_name}` already exists, skip `{fstdx_}`"})
                    continue
            Utils.createDirIfnotExists(dict_dir)
            await send_progress({"type": "info", "msg": f"Copying `{fstdx_}` to `{dict_dir}` ..."})
            Utils.copyFile(fstdx_, dict_dir)
            new_dict_names.append(dict_name)

        for mdx_ in mdx:
            dict_name = Path(mdx_).stem
            dict_dir = Utils.getDictDir(dict_name)
            fstdx_path = os.path.join(dict_dir, dict_name + ".fstdx")
            if os.path.exists(fstdx_path):
                reader = fstd.FstdxReader(fstdx_path)
                if reader:
                    logger.warning(f"Dictionary {dict_name} already exists, skip {mdx_}")
                    await send_progress({"msg": f"Dictionary `{dict_name}` already exists, skip `{mdx_}`", "type": "warning"})
                    continue
            Utils.createDirIfnotExists(dict_dir)
            await send_progress({"type": "info", "msg": f"Converting `{mdx_}` to fstdx, it may take a while, please wait ..."})
            ret = fstdtools.convert(mdx_, fstdx_path, compress_level=5, compress_dict_size=130, block_size=32)
            if ret != 0:
                logger.error(f"Failed to convert dictionary {mdx_}")
                await send_progress({"msg": f"Failed to convert dictionary {mdx_}", "type": "error"})
                continue
            new_dict_names.append(dict_name)

        if len(new_dict_names) == 1:
            dict_name = new_dict_names[0]
            dict_dir = Utils.getDictDir(dict_name)
            fstdx_path = os.path.join(dict_dir, dict_name + ".fstdx")
            for item in mdd:
                output_path = os.path.join(dict_dir, Path(item).stem + ".fstdd")
                await send_progress({"type": "info", "msg": f"Converting `{item}` to `fstdd`, it may take a while, please wait ..."})
                ret = fstdtools.convert(item, output_path, compress_level=5, compress_dict_size=130, block_size=32)
                if ret != 0:
                    logger.error(f"Failed to convert dictionary {item}")
                    await send_progress({"msg": f"Failed to convert dictionary `{item}`", "type": "error"})
            for item in fstdd:
                await send_progress({"type": "info", "msg": f"Copying `{item}` to `{dict_dir}` ..."})
                Utils.copyFile(item, dict_dir)
            for item in cover:
                await send_progress({"type": "info", "msg": f"Copying `{item}` to `{dict_dir}` ..."})
                Utils.copyFile(item, dict_dir)
            for item in css:
                await send_progress({"type": "info", "msg": f"Copying `{item}` to `{dict_dir}` ..."})
                Utils.copyFile(item, dict_dir)
            for item in js:
                await send_progress({"type": "info", "msg": f"Copying `{item}` to `{dict_dir}` ..."})
                Utils.copyFile(item, dict_dir)
            Utils.Config.checkDictInfo(Path(dict_dir))
            self._load_dictionary(dict_name, fstdx_path)
            await send_progress({"msg": f"Dictionary `{dict_name}` added successfully", "type": "success"})
            return

        reload_dict_names = []
        for item in mdd:
            dict_name = Path(item).stem.split('.')[0]
            if not dict_name:
                continue
            dict_dir = Utils.getDictDir(dict_name)
            if not os.path.exists(dict_dir):
                await send_progress({"msg": f"Dictionary `{dict_name}` does not exist, skip `{item}`", "type": "warning"})
                continue

            output_path = os.path.join(dict_dir, Path(item).stem + ".fstdd")
            await send_progress({"type": "info", "msg": f"Converting `{item}` to `fstdd`, it may take a while, please wait ..."})
            ret = fstdtools.convert(item, output_path, compress_level=5, compress_dict_size=130, block_size=32)
            if ret != 0:
                logger.error(f"Failed to convert dictionary {item}")
                await send_progress({"msg": f"Failed to convert dictionary `{item}`", "type": "error"})
            reload_dict_names.append(dict_name)

        for item in fstdd:
            await self._copy_file(item, reload_dict_names, send_progress)
        for item in cover:
            await self._copy_file(item, reload_dict_names, send_progress)
        for item in css:
            await self._copy_file(item, reload_dict_names, send_progress)
        for item in js:
            await self._copy_file(item, reload_dict_names, send_progress)

        for dict_name in new_dict_names:
            dict_dir = Utils.getDictDir(dict_name)
            fstdx_path = os.path.join(dict_dir, dict_name + ".fstdx")
            Utils.Config.checkDictInfo(Path(dict_dir))
            self._load_dictionary(dict_name, fstdx_path)

        for dict_name in reload_dict_names:
            self.reload_dictionary(dict_name)


# Global singleton instance
dictionary_searcher = DictionarySearcher()
