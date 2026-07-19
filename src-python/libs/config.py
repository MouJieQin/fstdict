import json
import os
import sys
from pathlib import Path
import platformdirs
import shutil
from fastapi import WebSocket
from typing import Dict
from libs.log_config import logger
from urllib.parse import quote

import fstd


class UtilsBase:
    # 路径配置
    APP_NAME = "FstDict"
    APP_AUTHOR = "qinmoujie"
    BASE_DIR = Path(sys._MEIPASS) if getattr(sys, 'frozen', False) else Path(__file__).resolve().parent.parent
    APP_SUPPORT_PATH = platformdirs.user_data_dir(APP_NAME, APP_AUTHOR)
    APP_LOG_PATH = platformdirs.user_log_dir(APP_NAME, APP_AUTHOR)
    APP_CACHE_PATH = platformdirs.user_cache_dir(APP_NAME, APP_AUTHOR)
    FSTDICT_SUPPORT_PATH = f"{APP_SUPPORT_PATH}"
    FSTDICT_STORAGE_PATH = f"{FSTDICT_SUPPORT_PATH}/Storage"
    USER_CONFIG_DIR = FSTDICT_STORAGE_PATH + "/config"
    CONFIG_FILE = USER_CONFIG_DIR + "/config.json"
    ANKI_CONFIG_FILE = USER_CONFIG_DIR + "/anki_config.json"
    DEFAULT_CONFIG_FILE = str(BASE_DIR / "config.json")
    DICTIONARYS_PATH = FSTDICT_STORAGE_PATH + "/dictionaries"
    FSTD_SEARCHER_META_PATH = DICTIONARYS_PATH + "/fstd_searcher_meta.json"
    FSTDX_INDEX_PATH = DICTIONARYS_PATH + "/fstd_indexes.fstdxidx"
    DATA_PATH = FSTDICT_STORAGE_PATH + "/data"
    FSTDICT_DATABASE_PATH = DATA_PATH + "/fstdict.db"
    DICT_DATABASE_PATH = DATA_PATH + "/dict.db"

    fstd_engine = fstd.FstdxSearcher()

    DEFAULT_CONFIG = {}
    CONFIG = {}
    FSTDICT_CONFIG = {}
    DICT_INFO = {}

    # WebSocket 连接管理
    electron_websockets: Dict[int, WebSocket] = {}
    spa_websockets: Dict[int, WebSocket] = {}
    session_websockets: Dict[int, Dict[int, WebSocket]] = {}
    windows_websockets: Dict[int, WebSocket] = {}

    @staticmethod
    def createDirIfnotExists(path: str):
        if not os.path.exists(path):
            os.makedirs(path)

    @staticmethod
    def copyFile(src: str, dst: str):
        shutil.copy2(src, dst)

    @staticmethod
    def removeDirIfExists(path: str):
        if os.path.exists(path):
            shutil.rmtree(path)

    @staticmethod
    def getDictDir(dict_name: str) -> str:
        return UtilsBase.DICTIONARYS_PATH + "/" + dict_name

    @staticmethod
    def getDictPath(dict_name: str) -> str:
        return UtilsBase.getDictDir(dict_name) + "/" + dict_name + ".fstdx"

    @staticmethod
    def removeFileIfExists(path: str):
        if os.path.exists(path):
            os.remove(path)

    @staticmethod
    def find_files_by_postfix(root_dir: str, dictName: str, postfix: str) -> list[str]:
        files = []
        p = Path(root_dir)
        for item in p.iterdir():
            if item.is_file() and item.name.lower().endswith(postfix):
                files.append("/".join([dictName, item.name]))
        return files

    class Config:
        @staticmethod
        def syncConfig():
            """同步配置文件"""
            with open(UtilsBase.CONFIG_FILE, mode="w", encoding="utf-8") as f:
                f.write(json.dumps(UtilsBase.CONFIG, ensure_ascii=False, indent=4))

        @staticmethod
        def init_config(config: dict):
            """初始化配置目录和文件"""
            UtilsBase.CONFIG = config
            UtilsBase.Config.syncConfig()

        @staticmethod
        def create_dict_set_option(option_name: str) -> bool:
            dict_set_options: dict = UtilsBase.CONFIG["dict_set_options"]
            if option_name not in dict_set_options:
                dict_set_options[option_name] = json.loads(json.dumps(dict_set_options["default"], ensure_ascii=False))
                UtilsBase.Config.syncConfig()
                return True
            return False

        @staticmethod
        def _renew_dict_set_option(old_dict_set_option: list) -> list:
            old_dict_names = []
            default_dict_set_options = []
            for item in old_dict_set_option:
                name = item["name"]
                if name in UtilsBase.DICT_INFO:
                    old_dict_names.append(name)
            new_dict_names = []
            for dict_name in UtilsBase.DICT_INFO:
                if dict_name not in old_dict_names:
                    new_dict_names.append(dict_name)
            for name in new_dict_names:
                default_dict_set_options.append({"name": name,
                                                 "cover_url": f"http://localhost:5959/api/download?path={quote(UtilsBase.DICT_INFO[name]['cover'])}",
                                                 "is_enabled": True})
            for item in old_dict_set_option:
                name = item["name"]
                if name in UtilsBase.DICT_INFO:
                    default_dict_set_options.append(item)
            return default_dict_set_options

        @staticmethod
        def renew_dict_set_options():
            old_dict_set_options: dict = UtilsBase.CONFIG["dict_set_options"]
            new_dict_set_options = {}
            for key, option in old_dict_set_options.items():
                new_dict_set_options[key] = UtilsBase.Config._renew_dict_set_option(option)
            UtilsBase.CONFIG["dict_set_options"] = new_dict_set_options

        @staticmethod
        def checkDictInfo(file: Path):
            dict_name = file.name
            fstdx_path = file.absolute() / f"{dict_name}.fstdx"
            if fstdx_path.is_file():
                UtilsBase.DICT_INFO[dict_name] = {}
                UtilsBase.DICT_INFO[dict_name]["name"] = dict_name
                UtilsBase.DICT_INFO[dict_name]["root"] = str(file.absolute())
                UtilsBase.DICT_INFO[dict_name]["path"] = str(fstdx_path.absolute())
                UtilsBase.DICT_INFO[dict_name]["css"] = (
                    UtilsBase.find_files_by_postfix(str(file.absolute()), dict_name, ".css")
                )
                UtilsBase.DICT_INFO[dict_name]["js"] = (
                    UtilsBase.find_files_by_postfix(str(file.absolute()), dict_name, ".js")
                )
                data_path = file.absolute() / "data"
                if data_path.is_dir():
                    UtilsBase.DICT_INFO[dict_name]["data"] = str(
                        data_path.absolute()
                    )
                else:
                    UtilsBase.DICT_INFO[dict_name]["data"] = ""
                UtilsBase.DICT_INFO[dict_name]["cover"] = ""
                # walk through the current folder to find cover image with suffix .jpg/.jpeg/.png/.gif
                for img_file in file.iterdir():
                    if img_file.is_file() and img_file.suffix.lower() in [
                        ".jpg",
                        ".jpeg",
                        ".png",
                        ".gif",
                    ]:
                        UtilsBase.DICT_INFO[dict_name]["cover"] = "/".join([dict_name, img_file.name])
                        break

        @staticmethod
        def removeDictInfo(dict_name: str):
            """删除字典信息"""
            UtilsBase.DICT_INFO.pop(dict_name, None)


def init_config():
    """初始化配置目录和文件"""
    logger.info(f"fstdict support path: {UtilsBase.FSTDICT_SUPPORT_PATH}")
    UtilsBase.createDirIfnotExists(UtilsBase.USER_CONFIG_DIR)
    UtilsBase.createDirIfnotExists(UtilsBase.DATA_PATH)
    UtilsBase.createDirIfnotExists(UtilsBase.DICTIONARYS_PATH)

    with open(UtilsBase.DEFAULT_CONFIG_FILE, mode="r", encoding="utf-8") as f:
        UtilsBase.DEFAULT_CONFIG = json.load(f)

    if os.path.isfile(UtilsBase.CONFIG_FILE):
        with open(UtilsBase.CONFIG_FILE, mode="r", encoding="utf-8") as f:
            UtilsBase.CONFIG = json.load(f)
    else:
        UtilsBase.CONFIG = {}

    dict_path = Path(UtilsBase.DICTIONARYS_PATH)
    for file in dict_path.iterdir():
        if file.is_dir():
            UtilsBase.Config.checkDictInfo(file)

    diff_flag = False
    # 检查配置项是否缺失，并使用默认值填充

    def setDefaultValIfNone(config: dict, defaultConfig: dict):
        nonlocal diff_flag
        for key, default_val in defaultConfig.items():
            if key not in config:
                diff_flag = True
                config[key] = default_val
            else:
                if isinstance(default_val, dict):
                    setDefaultValIfNone(config[key], default_val)

    setDefaultValIfNone(UtilsBase.CONFIG, UtilsBase.DEFAULT_CONFIG)

    if diff_flag:
        logger.info("配置文件缺失部分项，已使用默认值填充")
        logger.info(f"配置文件: {UtilsBase.CONFIG_FILE}")
        logger.info(f"默认配置: {UtilsBase.DEFAULT_CONFIG_FILE}")
        UtilsBase.Config.syncConfig()

    UtilsBase.Config.renew_dict_set_options()
    UtilsBase.Config.init_config(UtilsBase.CONFIG)


init_config()
