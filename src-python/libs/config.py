import json
import os
from pathlib import Path
import appdirs
import shutil
from fastapi import WebSocket
from typing import Dict
from libs.log_config import logger


class UtilsBase:
    # 路径配置
    SERVER_SRC_ABS_PATH = os.path.abspath(os.getcwd())
    APP_SUPPORT_PATH = appdirs.user_data_dir()[0:-1]
    MXDICT_SUPPORT_PATH = f"{APP_SUPPORT_PATH}/com.qinmoujie.mxdict"
    MXDICT_STORAGE_PATH = f"{MXDICT_SUPPORT_PATH}/MxDict-Storage"
    USER_CONFIG_DIR = MXDICT_STORAGE_PATH + "/config"
    CONFIG_FILE = USER_CONFIG_DIR + "/config.json"
    ANKI_CONFIG_FILE = USER_CONFIG_DIR + "/anki_config.json"
    DEFAULT_CONFIG_FILE = SERVER_SRC_ABS_PATH + "/config.json"
    DICTIONARYS_PATH = MXDICT_STORAGE_PATH + "/dictionaries"
    DATA_PATH = MXDICT_STORAGE_PATH + "/data"
    MXDICT_DATABASE_PATH = DATA_PATH + "/mxdict.db"
    DICT_DATABASE_PATH = DATA_PATH + "/dict.db"

    DEFAULT_CONFIG = {}
    CONFIG = {}
    MXDICT_CONFIG = {}
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
    def removeDirIfExists(path: str):
        if os.path.exists(path):
            shutil.rmtree(path)

    @staticmethod
    def removeFileIfExists(path: str):
        if os.path.exists(path):
            os.remove(path)

    @staticmethod
    def find_files_by_postfix(root_dir: str, postfix: str) -> list[str]:
        # 递归遍历目录
        files = []
        for dir_path, _, filenames in os.walk(root_dir):
            # 遍历当前目录所有文件
            for filename in filenames:
                # 判断是否是 css 文件
                if filename.lower().endswith(postfix):
                    # 返回完整路径
                    files.append(os.path.join(dir_path, filename))
        # 没找到返回 ""
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

            # UtilsBase.AI_CONFIG = UtilsBase.CONFIG["ai_assistant"]


def init_config():
    """初始化配置目录和文件"""
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

    def checkDickInfo():
        dict_path = Path(UtilsBase.DICTIONARYS_PATH)
        # 获取所有文件
        for file in dict_path.iterdir():
            if file.is_dir():
                mdx_path = file.resolve() / f"{file.name}.fstdx"
                mdict_info_json = file.resolve() / "mxdict_info.json"
                if mdx_path.is_file():
                    if mdict_info_json.is_file():
                        with open(mdict_info_json, mode="r", encoding="utf-8") as f:
                            mdict_info = json.load(f)
                            UtilsBase.DICT_INFO[file.name] = mdict_info
                    else:
                        UtilsBase.DICT_INFO[file.name] = {}
                        UtilsBase.DICT_INFO[file.name]["name"] = file.name
                        UtilsBase.DICT_INFO[file.name]["root"] = str(file.resolve())
                        UtilsBase.DICT_INFO[file.name]["path"] = str(mdx_path.resolve())
                        UtilsBase.DICT_INFO[file.name]["css"] = (
                            UtilsBase.find_files_by_postfix(str(file.resolve()), ".css")
                        )
                        UtilsBase.DICT_INFO[file.name]["js"] = (
                            UtilsBase.find_files_by_postfix(str(file.resolve()), ".js")
                        )
                        data_path = file.resolve() / "data"
                        if data_path.is_dir():
                            UtilsBase.DICT_INFO[file.name]["data"] = str(
                                data_path.resolve()
                            )
                        else:
                            UtilsBase.DICT_INFO[file.name]["data"] = ""
                        UtilsBase.DICT_INFO[file.name]["cover"] = ""
                        # walk through the current folder to find cover image with suffix .jpg/.jpeg/.png/.gif
                        for img_file in file.iterdir():
                            if img_file.is_file() and img_file.suffix.lower() in [
                                ".jpg",
                                ".jpeg",
                                ".png",
                                ".gif",
                            ]:
                                UtilsBase.DICT_INFO[file.name]["cover"] = str(
                                    img_file.resolve()
                                )
                                break
                        # save dict info into mxdict_info.json
                        with open(mdict_info_json, mode="w", encoding="utf-8") as f:
                            json.dump(
                                UtilsBase.DICT_INFO[file.name],
                                f,
                                ensure_ascii=False,
                                indent=4,
                            )

    checkDickInfo()

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

    UtilsBase.Config.init_config(UtilsBase.CONFIG)


init_config()
