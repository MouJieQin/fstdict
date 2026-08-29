"""
Application configuration management: loading, saving, and migration.
"""
import json
import os
import sys
import shutil
from pathlib import Path
from urllib.parse import quote
from typing import Dict, List
from fastapi import WebSocket


import fstd

import libs.config.paths as app_paths
# from libs.config.paths import *
from libs.log_config import logger


class UtilsBase:
    """Base class containing all configuration constants and state."""

    # Detect PyInstaller packaged binary
    IS_FROZEN = getattr(sys, "frozen", False)

    # App metadata
    APP_NAME = app_paths.APP_NAME
    APP_AUTHOR = app_paths.APP_AUTHOR

    # Path constants (re-exported from paths module)
    BASE_DIR = app_paths.BASE_DIR
    APP_SUPPORT_PATH = str(app_paths.APP_SUPPORT_PATH)
    APP_LOG_PATH = str(app_paths.APP_LOG_PATH)
    APP_CACHE_PATH = str(app_paths.APP_CACHE_PATH)
    FSTDICT_SUPPORT_PATH = str(app_paths.APP_SUPPORT_PATH)
    FSTDICT_STORAGE_PATH = str(app_paths.STORAGE_PATH)
    USER_CONFIG_DIR = str(app_paths.CONFIG_DIR)
    CONFIG_FILE = str(app_paths.CONFIG_FILE)
    DICT_CONFIG_FILE = str(app_paths.DICT_CONFIG_FILE)
    ANKI_CONFIG_FILE = str(app_paths.ANKI_CONFIG_FILE)

    DEFAULT_CONFIG_FILE = str(app_paths.DEFAULT_CONFIG_FILE)
    DEFAULT_DICT_CONFIG_FILE = str(app_paths.DEFAULT_DICT_CONFIG_FILE)

    FFMPEG_PATH = str(app_paths.FFMPEG_BINARY)
    DICTIONARYS_PATH = str(app_paths.DICTIONARIES_DIR)
    DATA_PATH = str(app_paths.DATA_DIR)
    RAPID_OCR_MODELS_PATH = str(app_paths.RAPIDOCR_MODELS_DIR)
    FSTDICT_DATABASE_PATH = str(app_paths.FSTDICT_DB_PATH)
    DICT_DATABASE_PATH = str(app_paths.DICT_DB_PATH)
    IMA_PATH_FOR_OCR = str(app_paths.OCR_SCREENSHOT_PATH)

    AUDIO_SUFFIX = app_paths.AUDIO_EXTENSIONS

    # FST search engine instance
    fstd_engine = fstd.FstdxSearcher()
    fstd.set_log_level(3)

    # Configuration state
    DEFAULT_CONFIG: Dict = {}
    CONFIG: Dict = {}
    DEFAULT_DICT_CONFIG: Dict = {}
    DICT_CONFIG: Dict = {}

    REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION: List[str] = []
    DICT_INFO: Dict = {}

    # WebSocket connection state maps
    electron_websockets: Dict[int, WebSocket] = {}
    session_websockets: Dict[int, Dict[int, WebSocket]] = {}

    fstdict_helper_websocket: WebSocket | None = None
    fstdict_main_websocket: WebSocket | None = None
    cgevent_register_map: Dict = {}

    # --- File system utilities ---

    @staticmethod
    def createDirIfnotExists(path: str) -> None:
        if not os.path.exists(path):
            os.makedirs(path)

    @staticmethod
    def copyFile(src: str, dst: str) -> None:
        shutil.copy2(src, dst)

    @staticmethod
    def removeDirIfExists(path: str) -> None:
        if os.path.exists(path):
            shutil.rmtree(path)

    @staticmethod
    def getDictDir(dict_name: str) -> str:
        return os.path.join(UtilsBase.DICTIONARYS_PATH, dict_name)

    @staticmethod
    def getDictPath(dict_name: str) -> str:
        return os.path.join(UtilsBase.getDictDir(dict_name), f"{dict_name}.fstdx")

    @staticmethod
    def removeFileIfExists(path: str) -> None:
        if os.path.exists(path):
            os.remove(path)

    @staticmethod
    def find_files_by_postfix(root_dir: str, dict_name: str, postfix: str) -> List[str]:
        """Find all files with a given extension in a directory."""
        files = []
        for item in Path(root_dir).iterdir():
            if item.is_file() and item.name.lower().endswith(postfix):
                files.append(f"{dict_name}/{item.name}")
        return files

    class Config:
        """Configuration management inner class."""

        @staticmethod
        def syncConfigFile(config: Dict, config_file: str) -> None:
            """Write configuration dictionary to a JSON file."""
            with open(config_file, mode="w", encoding="utf-8") as f:
                f.write(json.dumps(config, ensure_ascii=False, indent=4))

        @staticmethod
        def syncConfig() -> None:
            UtilsBase.Config.syncConfigFile(UtilsBase.CONFIG, UtilsBase.CONFIG_FILE)

        @staticmethod
        def syncDictConfig() -> None:
            UtilsBase.Config.syncConfigFile(
                UtilsBase.DICT_CONFIG, UtilsBase.DICT_CONFIG_FILE
            )

        @staticmethod
        def init_config(config: Dict) -> None:
            UtilsBase.CONFIG = config
            UtilsBase.Config.syncConfig()

        @staticmethod
        def init_dict_config(config: Dict) -> None:
            UtilsBase.DICT_CONFIG = config
            UtilsBase.Config.syncDictConfig()

        @staticmethod
        def create_dict_set_option(option_name: str) -> bool:
            dict_set_options: Dict = UtilsBase.DICT_CONFIG["dict_set_options"]
            if option_name not in dict_set_options:
                dict_set_options[option_name] = json.loads(
                    json.dumps(dict_set_options["default"], ensure_ascii=False)
                )
                UtilsBase.Config.syncDictConfig()
                return True
            return False

        @staticmethod
        def remove_dict_set_option(option_name: str) -> bool:
            dict_set_options: Dict = UtilsBase.DICT_CONFIG["dict_set_options"]
            if option_name != "default" and option_name in dict_set_options:
                del dict_set_options[option_name]
                UtilsBase.Config.syncDictConfig()
                return True
            return False

        @staticmethod
        def rename_dict_set_option(old_name: str, new_name: str) -> bool:
            dict_set_options: Dict = UtilsBase.DICT_CONFIG["dict_set_options"]
            if old_name != "default" and old_name in dict_set_options:
                option = json.loads(
                    json.dumps(dict_set_options[old_name], ensure_ascii=False)
                )
                del dict_set_options[old_name]
                dict_set_options[new_name] = option
                UtilsBase.Config.syncDictConfig()
                return True
            return False

        @staticmethod
        def checkDictInfo(directory: Path) -> None:
            """Scan a dictionary directory and populate DICT_INFO metadata."""
            dict_name = directory.name
            fstdx_path = directory.absolute() / f"{dict_name}.fstdx"

            if not fstdx_path.is_file():
                return

            UtilsBase.DICT_INFO[dict_name] = {
                "name": dict_name,
                "root": str(directory.absolute()),
                "path": str(fstdx_path.absolute()),
                "css": UtilsBase.find_files_by_postfix(
                    str(directory.absolute()), dict_name, ".css"
                ),
                "js": UtilsBase.find_files_by_postfix(
                    str(directory.absolute()), dict_name, ".js"
                ),
                "data": "",
                "cover": "",
                "cover_url": "",
            }

            # Check for data directory
            data_path = directory.absolute() / "data"
            if data_path.is_dir():
                UtilsBase.DICT_INFO[dict_name]["data"] = str(data_path.absolute())

            # Look for cover image
            for img_file in directory.iterdir():
                if img_file.is_file() and img_file.suffix.lower() in [
                    ".jpg", ".jpeg", ".png", ".gif"
                ]:
                    cover_rel_path = f"{dict_name}/{img_file.name}"
                    UtilsBase.DICT_INFO[dict_name]["cover"] = cover_rel_path
                    UtilsBase.DICT_INFO[dict_name]["cover_url"] = (
                        f"http://127.0.0.1:5959/api/dictionaries/{quote(cover_rel_path)}"
                    )
                    break

        @staticmethod
        def removeDictInfo(dict_name: str) -> None:
            """Remove dictionary metadata from DICT_INFO."""
            UtilsBase.DICT_INFO.pop(dict_name, None)

        @staticmethod
        def _renew_dict_set_option(old_option: List) -> List:
            """Update a dictionary set option with newly discovered dictionaries."""
            old_names = [item["name"] for item in old_option if item["name"] in UtilsBase.DICT_INFO]
            new_options = []

            # Add new dictionaries not in the old set
            for dict_name in UtilsBase.DICT_INFO:
                if dict_name not in old_names:
                    new_options.append({"name": dict_name, "is_enabled": False})

            # Keep existing dictionary settings
            for item in old_option:
                if item["name"] in UtilsBase.DICT_INFO:
                    new_options.append(item)

            return new_options

        @staticmethod
        def renew_dict_set_options() -> None:
            """Refresh all dictionary set options with current dictionary list."""
            old_options: Dict = UtilsBase.DICT_CONFIG["dict_set_options"]
            new_options = {}
            for key, option in old_options.items():
                new_options[key] = UtilsBase.Config._renew_dict_set_option(option)
            UtilsBase.DICT_CONFIG["dict_set_options"] = new_options


def initialize_config() -> None:
    """
    Initialize the application configuration.
    Creates directories, loads config files, applies defaults, and migrates schemas.
    """
    logger.info(f"FstDict support path: {UtilsBase.FSTDICT_SUPPORT_PATH}")

    # Ensure required directories exist
    UtilsBase.createDirIfnotExists(UtilsBase.USER_CONFIG_DIR)
    UtilsBase.createDirIfnotExists(UtilsBase.DATA_PATH)
    UtilsBase.createDirIfnotExists(UtilsBase.DICTIONARYS_PATH)
    UtilsBase.createDirIfnotExists(UtilsBase.RAPID_OCR_MODELS_PATH)

    # Load default config templates
    with open(UtilsBase.DEFAULT_CONFIG_FILE, mode="r", encoding="utf-8") as f:
        UtilsBase.DEFAULT_CONFIG = json.load(f)
    with open(UtilsBase.DEFAULT_DICT_CONFIG_FILE, mode="r", encoding="utf-8") as f:
        UtilsBase.DEFAULT_DICT_CONFIG = json.load(f)

    # Load user configs if they exist
    for config_file, config_attr in [
        (UtilsBase.CONFIG_FILE, "CONFIG"),
        (UtilsBase.DICT_CONFIG_FILE, "DICT_CONFIG"),
    ]:
        if os.path.isfile(config_file):
            with open(config_file, mode="r", encoding="utf-8") as f:
                setattr(UtilsBase, config_attr, json.load(f))
        else:
            setattr(UtilsBase, config_attr, {})

    # Scan dictionaries directory
    dict_path = Path(UtilsBase.DICTIONARYS_PATH)
    for file in dict_path.iterdir():
        if file.is_dir():
            UtilsBase.Config.checkDictInfo(file)

    # Helper: recursively set default values for missing keys
    def set_defaults(config: Dict, defaults: Dict) -> bool:
        """Set default values for missing keys. Returns True if config was unchanged."""
        changed = False

        def _apply(cfg: Dict, dflt: Dict):
            nonlocal changed
            for key, value in dflt.items():
                if key not in cfg:
                    changed = True
                    cfg[key] = value
                elif isinstance(value, dict):
                    _apply(cfg[key], value)
        _apply(config, defaults)
        return not changed

    # Helper: remove deprecated keys
    def remove_keys(config: Dict, key_paths: List[List[str]]) -> bool:
        """Remove deprecated configuration keys. Returns True if anything was removed."""
        removed = False

        def _remove(cfg: Dict, keys: List[str], index: int):
            nonlocal removed
            key = keys[index]
            if index == len(keys) - 1:
                if key in cfg:
                    removed = True
                    del cfg[key]
            else:
                if key in cfg:
                    _remove(cfg[key], keys, index + 1)

        for key_path in key_paths:
            _remove(config, key_path, 0)
        return removed

    # Apply defaults and clean up deprecated keys for each config
    def migrate_config(config: Dict, defaults: Dict, file_path: str, remove_list: List[List[str]]) -> None:
        needs_sync = False
        if not set_defaults(config, defaults):
            needs_sync = True
        if remove_keys(config, remove_list):
            needs_sync = True
        if needs_sync:
            UtilsBase.Config.syncConfigFile(config, file_path)

    migrate_config(
        UtilsBase.CONFIG,
        UtilsBase.DEFAULT_CONFIG,
        UtilsBase.CONFIG_FILE,
        [["schema_version"], ["dict_set_options"]]
    )
    migrate_config(
        UtilsBase.DICT_CONFIG,
        UtilsBase.DEFAULT_DICT_CONFIG,
        UtilsBase.DICT_CONFIG_FILE,
        []
    )

    # Apply selection monitoring setting to auto-register list
    if UtilsBase.CONFIG["app"]["helper_selection"]["enabled"]:
        UtilsBase.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION.append("kHandlerTextSelection")

    # Final initialization steps
    UtilsBase.Config.renew_dict_set_options()
    UtilsBase.Config.init_config(UtilsBase.CONFIG)
    UtilsBase.Config.init_dict_config(UtilsBase.DICT_CONFIG)


# Run initialization on module import
initialize_config()
