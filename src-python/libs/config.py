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
    AUDIO_SUFFIX = {".spx", ".ogg", ".wav", ".mp3", ".amr", ".aac", ".flac", ".m4a", ".opus", ".wma"}
    BASE_DIR = Path(sys._MEIPASS) if getattr(sys, 'frozen', False) else Path(__file__).resolve().parent.parent
    APP_SUPPORT_PATH = platformdirs.user_data_dir(APP_NAME, APP_AUTHOR)
    APP_LOG_PATH = platformdirs.user_log_dir(APP_NAME, APP_AUTHOR)
    APP_CACHE_PATH = platformdirs.user_cache_dir(APP_NAME, APP_AUTHOR)
    FSTDICT_SUPPORT_PATH = f"{APP_SUPPORT_PATH}"
    FSTDICT_STORAGE_PATH = f"{FSTDICT_SUPPORT_PATH}/Storage"
    USER_CONFIG_DIR = FSTDICT_STORAGE_PATH + "/config"
    CONFIG_FILE = USER_CONFIG_DIR + "/config.json"
    CGEVENT_CONFIG_FILE = USER_CONFIG_DIR + "/cgevent_config.json"
    DICT_CONFIG_FILE = USER_CONFIG_DIR + "/dict_config.json"
    ANKI_CONFIG_FILE = USER_CONFIG_DIR + "/anki_config.json"
    DEFAULT_CONFIG_FILE = str(BASE_DIR / "config.json")
    DEFAULT_CGEVENT_CONFIG_FILE = str(BASE_DIR / "cgevent_config.json")
    DEFAULT_DICT_CONFIG_FILE = str(BASE_DIR / "dict_config.json")
    FFMPEG_PATH = str(BASE_DIR / "ffmpeg" / ("fstdict-ffmpeg.exe" if sys.platform.startswith("win") else "fstdict-ffmpeg"))
    DICTIONARYS_PATH = FSTDICT_STORAGE_PATH + "/dictionaries"
    DATA_PATH = FSTDICT_STORAGE_PATH + "/data"
    FSTDICT_DATABASE_PATH = DATA_PATH + "/fstdict.db"
    DICT_DATABASE_PATH = DATA_PATH + "/dict.db"
    IMA_PATH_FOR_OCR = APP_CACHE_PATH + "/screenshot_for_ocr.png"

    fstd_engine = fstd.FstdxSearcher()

    DEFAULT_CONFIG = {}
    CONFIG = {}
    DEFAULT_CGEVENT_CONFIG = {}
    CGEVENT_CONFIG = {}
    DEFAULT_DICT_CONFIG = {}
    DICT_CONFIG = {}
    REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION = ["globalKeyboardShortCut"]
    DICT_INFO = {}

    # WebSocket 连接管理
    electron_websockets: Dict[int, WebSocket] = {}
    spa_websockets: Dict[int, WebSocket] = {}
    session_websockets: Dict[int, Dict[int, WebSocket]] = {}
    windows_websockets: Dict[int, WebSocket] = {}
    fstdict_helper_websocket: WebSocket | None = None
    fstdict_main_websocket: WebSocket | None = None
    cgevent_register_map: Dict = {}

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
        def syncConfigFile(config: dict, config_file: str):
            with open(config_file, mode="w", encoding="utf-8") as f:
                f.write(json.dumps(config, ensure_ascii=False, indent=4))

        @staticmethod
        def syncConfig():
            """同步配置文件"""
            UtilsBase.Config.syncConfigFile(UtilsBase.CONFIG, UtilsBase.CONFIG_FILE)

        @staticmethod
        def syncCgeventConfig():
            UtilsBase.Config.syncConfigFile(UtilsBase.CGEVENT_CONFIG, UtilsBase.CGEVENT_CONFIG_FILE)

        @staticmethod
        def syncDictConfig():
            UtilsBase.Config.syncConfigFile(UtilsBase.DICT_CONFIG, UtilsBase.DICT_CONFIG_FILE)

        @staticmethod
        def init_config(config: dict):
            """初始化配置目录和文件"""
            UtilsBase.CONFIG = config
            UtilsBase.Config.syncConfig()

        @staticmethod
        def init_cgevent_config(cgevent_config: dict):
            """初始化Cgevent配置目录和文件"""
            UtilsBase.CGEVENT_CONFIG = cgevent_config
            UtilsBase.Config.syncDictConfig()

        @staticmethod
        def init_dict_config(dict_config: dict):
            UtilsBase.DICT_CONFIG = dict_config
            UtilsBase.Config.syncDictConfig()

        @staticmethod
        def create_dict_set_option(option_name: str) -> bool:
            dict_set_options: dict = UtilsBase.DICT_CONFIG["dict_set_options"]
            if option_name not in dict_set_options:
                dict_set_options[option_name] = json.loads(json.dumps(dict_set_options["default"], ensure_ascii=False))
                UtilsBase.Config.syncDictConfig()
                return True
            return False

        @staticmethod
        def remove_dict_set_option(option_name: str) -> bool:
            dict_set_options: dict = UtilsBase.DICT_CONFIG["dict_set_options"]
            if option_name != 'default' and option_name in dict_set_options:
                del dict_set_options[option_name]
                UtilsBase.Config.syncDictConfig()
                return True
            return False

        @staticmethod
        def rename_dict_set_option(old_option_name: str, new_option_name: str) -> bool:
            dict_set_options: dict = UtilsBase.DICT_CONFIG["dict_set_options"]
            if old_option_name != 'default' and old_option_name in dict_set_options:
                new_option = json.loads(json.dumps(dict_set_options[old_option_name], ensure_ascii=False))
                del dict_set_options[old_option_name]
                dict_set_options[new_option_name] = new_option
                UtilsBase.Config.syncDictConfig()
                return True
            return False

        @staticmethod
        def _renew_dict_set_option(old_dict_set_option: list) -> list:
            old_dict_names = []
            dict_set_options = []
            for item in old_dict_set_option:
                name = item["name"]
                if name in UtilsBase.DICT_INFO:
                    old_dict_names.append(name)
            new_dict_names = []
            for dict_name in UtilsBase.DICT_INFO:
                if dict_name not in old_dict_names:
                    new_dict_names.append(dict_name)
            for name in new_dict_names:
                dict_set_options.append({"name": name,
                                         "is_enabled": False})
            for item in old_dict_set_option:
                name = item["name"]
                if name in UtilsBase.DICT_INFO:
                    dict_set_options.append(item)
            return dict_set_options

        @staticmethod
        def renew_dict_set_options():
            old_dict_set_options: dict = UtilsBase.DICT_CONFIG["dict_set_options"]
            new_dict_set_options = {}
            for key, option in old_dict_set_options.items():
                new_dict_set_options[key] = UtilsBase.Config._renew_dict_set_option(option)
            UtilsBase.DICT_CONFIG["dict_set_options"] = new_dict_set_options

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
                        UtilsBase.DICT_INFO[dict_name]["cover_url"] = f"http://localhost:5959/api/download?path={quote(UtilsBase.DICT_INFO[dict_name]['cover'])}"
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

    with open(UtilsBase.DEFAULT_CGEVENT_CONFIG_FILE, mode="r", encoding="utf-8") as f:
        UtilsBase.DEFAULT_CGEVENT_CONFIG = json.load(f)

    with open(UtilsBase.DEFAULT_DICT_CONFIG_FILE, mode="r", encoding="utf-8") as f:
        UtilsBase.DEFAULT_DICT_CONFIG = json.load(f)

    if os.path.isfile(UtilsBase.CONFIG_FILE):
        with open(UtilsBase.CONFIG_FILE, mode="r", encoding="utf-8") as f:
            UtilsBase.CONFIG = json.load(f)
    else:
        UtilsBase.CONFIG = {}

    if os.path.isfile(UtilsBase.CGEVENT_CONFIG_FILE):
        with open(UtilsBase.CGEVENT_CONFIG_FILE, mode="r", encoding="utf-8") as f:
            UtilsBase.CGEVENT_CONFIG = json.load(f)
    else:
        UtilsBase.CGEVENT_CONFIG = {}

    if os.path.isfile(UtilsBase.DICT_CONFIG_FILE):
        with open(UtilsBase.DICT_CONFIG_FILE, mode="r", encoding="utf-8") as f:
            UtilsBase.DICT_CONFIG = json.load(f)
    else:
        UtilsBase.DICT_CONFIG = {}

    dict_path = Path(UtilsBase.DICTIONARYS_PATH)
    for file in dict_path.iterdir():
        if file.is_dir():
            UtilsBase.Config.checkDictInfo(file)

    def setDefaultValIfNone(config: dict, defaultConfig: dict):
        diff_flag = False

        def _setDefaultValIfNone(config: dict, defaultConfig: dict):
            nonlocal diff_flag
            for key, default_val in defaultConfig.items():
                if key not in config:
                    diff_flag = True
                    config[key] = default_val
                else:
                    if isinstance(default_val, dict):
                        _setDefaultValIfNone(config[key], default_val)
        _setDefaultValIfNone(config, defaultConfig)
        return not diff_flag

    def removeValIfExist(config: dict, keys: list[list]):
        removed_flag = False

        def _removeValIfExist(config: dict, key_str: list, index: int):
            nonlocal removed_flag
            key = key_str[index]
            if index == len(key_str) - 1:
                if key in config:
                    removed_flag = True
                    del config[key]
            else:
                if key in config:
                    _removeValIfExist(config[key], key_str, index + 1)

        for key_str in keys:
            _removeValIfExist(config, key_str, 0)
        return removed_flag

    def resetConfigIfNeed(config: dict, default_config: dict, config_file: str, keys_to_remove: list[list]):
        sync_flag = False
        if setDefaultValIfNone(config, default_config):
            sync_flag = True
        if removeValIfExist(config, keys_to_remove):
            sync_flag = True
        if sync_flag:
            UtilsBase.Config.syncConfigFile(config, config_file)

    keys_to_remove = [["schema_version"], ["dict_set_options"]]
    resetConfigIfNeed(UtilsBase.CONFIG, UtilsBase.DEFAULT_CONFIG, UtilsBase.CONFIG_FILE, keys_to_remove)

    keys_to_remove = []
    resetConfigIfNeed(UtilsBase.CGEVENT_CONFIG, UtilsBase.DEFAULT_CGEVENT_CONFIG, UtilsBase.CGEVENT_CONFIG_FILE, keys_to_remove)

    keys_to_remove = []
    resetConfigIfNeed(UtilsBase.DICT_CONFIG, UtilsBase.DEFAULT_DICT_CONFIG, UtilsBase.DICT_CONFIG_FILE, keys_to_remove)

    if UtilsBase.CONFIG["app"]["helper_selection"]["enabled"]:
        UtilsBase.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION.append("handlerEventTextSelection")

    UtilsBase.Config.renew_dict_set_options()
    UtilsBase.Config.init_config(UtilsBase.CONFIG)
    UtilsBase.Config.init_cgevent_config(UtilsBase.CGEVENT_CONFIG)
    UtilsBase.Config.init_dict_config(UtilsBase.DICT_CONFIG)


init_config()
