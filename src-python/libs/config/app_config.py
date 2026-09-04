"""
Application configuration management: loading, saving, and migration.
"""
import json
import os
import sys
import fstd
from pathlib import Path
from urllib.parse import quote
from typing import Dict, List
from fastapi import WebSocket

import libs.config.paths as app_paths
from libs.log_config import logger
from libs.common.utilbase import UtilBase
from libs.ws_clients.cgevent_client import CgEventWsClient
from libs.core.database import FstDictDatabase


class Utils(UtilBase):
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

    # Configuration state
    DEFAULT_CONFIG: Dict = {}
    CONFIG: Dict = {}
    DEFAULT_DICT_CONFIG: Dict = {}
    DICT_CONFIG: Dict = {}

    REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION: List[str] = []
    DICT_INFO: Dict = {}

    # FST search engine instance
    fstd_engine = fstd.FstdxSearcher()
    fstd.set_log_level(3)

    # Shortcut configuration state
    shortcut_map: Dict = {}

    # WebSocket connection state maps
    electron_websockets: Dict[int, WebSocket] = {}
    session_websockets: Dict[int, Dict[int, WebSocket]] = {}

    fstdict_helper_websocket: WebSocket | None = None
    fstdict_main_websocket: WebSocket | None = None
    cgevent_register_map: Dict = {}

    # Database instance
    db = FstDictDatabase(FSTDICT_DATABASE_PATH)

    # WebSocket client instances (initialized at app startup)
    cgevent_ws_client: CgEventWsClient

    # --- File system utilities ---

    @staticmethod
    def getDictDir(dict_name: str) -> str:
        return os.path.join(Utils.DICTIONARYS_PATH, dict_name)

    @staticmethod
    def getDictPath(dict_name: str) -> str:
        return os.path.join(Utils.getDictDir(dict_name), f"{dict_name}.fstdx")

    @staticmethod
    def delete_dictionary(dict_name: str) -> None:
        """Delete a dictionary directory and update configuration."""
        dict_dir = Utils.getDictDir(dict_name)
        Utils.removeDirIfExists(dict_dir)
        Utils.Config.removeDictInfo(dict_name)
        Utils.Config.renew_dict_set_options()

    @staticmethod
    def reveal_dict_in_file_manager(dict_name: str) -> bool:
        """Open file manager and highlight the dictionary file."""
        dict_path = Utils.getDictPath(dict_name)
        return Utils.reveal_in_file_manager(dict_path)

    class Config:
        """Configuration management inner class."""

        @staticmethod
        def syncConfigFile(config: Dict, config_file: str) -> None:
            """Write configuration dictionary to a JSON file."""
            with open(config_file, mode="w", encoding="utf-8") as f:
                f.write(json.dumps(config, ensure_ascii=False, indent=4))

        @staticmethod
        def syncConfig() -> None:
            Utils.Config.syncConfigFile(Utils.CONFIG, Utils.CONFIG_FILE)

        @staticmethod
        def syncDictConfig() -> None:
            Utils.Config.syncConfigFile(
                Utils.DICT_CONFIG, Utils.DICT_CONFIG_FILE
            )

        @staticmethod
        def init_config(config: Dict) -> None:
            Utils.CONFIG = config
            Utils.Config.syncConfig()

        @staticmethod
        def init_dict_config(config: Dict) -> None:
            Utils.DICT_CONFIG = config
            Utils.Config.syncDictConfig()

        @staticmethod
        def make_shorcut(shortcut_keys: List[str]) -> str:
            """Convert a list of shortcut keys into a standardized string format."""
            return "+".join(shortcut_keys)

        @staticmethod
        def init_shortcut_map() -> None:
            """Initialize the shortcut map from the configuration."""
            shortcuts = Utils.CONFIG.get("shortcuts", {})
            for sc_name, sc_keys in shortcuts.items():
                shortcut = Utils.Config.make_shorcut(sc_keys)
                Utils.shortcut_map[shortcut] = sc_name

        @staticmethod
        async def update_shortcut(shortcut_name: str, shortcut_keys: List[str]) -> None:
            """Update a specific shortcut in the configuration and map."""
            old_keys = Utils.CONFIG["shortcuts"].get(shortcut_name, [])
            old_shortcut = Utils.Config.make_shorcut(old_keys)
            if old_shortcut in Utils.shortcut_map:
                del Utils.shortcut_map[old_shortcut]
            Utils.CONFIG["shortcuts"][shortcut_name] = shortcut_keys
            Utils.Config.syncConfig()
            shortcut = Utils.Config.make_shorcut(shortcut_keys)
            Utils.shortcut_map[shortcut] = shortcut_name
            if Utils.fstdict_main_websocket:
                await Utils.fstdict_main_websocket.send_text(json.dumps({
                    "type": "unregister_shortcut",
                    "data": {
                        "shortcut": old_shortcut
                    }
                }))
                await Utils.fstdict_main_websocket.send_text(json.dumps({
                    "type": "register_shortcut",
                    "data": {
                        "shortcut": shortcut
                    }
                }))

        @staticmethod
        def create_dict_set_option(option_name: str) -> bool:
            dict_set_options: Dict = Utils.DICT_CONFIG["dict_set_options"]
            if option_name not in dict_set_options:
                dict_set_options[option_name] = json.loads(
                    json.dumps(dict_set_options["default"], ensure_ascii=False)
                )
                Utils.Config.syncDictConfig()
                return True
            return False

        @staticmethod
        def remove_dict_set_option(option_name: str) -> bool:
            dict_set_options: Dict = Utils.DICT_CONFIG["dict_set_options"]
            if option_name != "default" and option_name in dict_set_options:
                del dict_set_options[option_name]
                Utils.Config.syncDictConfig()
                return True
            return False

        @staticmethod
        def rename_dict_set_option(old_name: str, new_name: str) -> bool:
            dict_set_options: Dict = Utils.DICT_CONFIG["dict_set_options"]
            if old_name != "default" and old_name in dict_set_options:
                option = json.loads(
                    json.dumps(dict_set_options[old_name], ensure_ascii=False)
                )
                del dict_set_options[old_name]
                dict_set_options[new_name] = option
                Utils.Config.syncDictConfig()
                return True
            return False

        @staticmethod
        def checkDictInfo(directory: Path) -> None:
            """Scan a dictionary directory and populate DICT_INFO metadata."""
            dict_name = directory.name
            fstdx_path = directory.absolute() / f"{dict_name}.fstdx"

            if not fstdx_path.is_file():
                return

            Utils.DICT_INFO[dict_name] = {
                "name": dict_name,
                "root": str(directory.absolute()),
                "path": str(fstdx_path.absolute()),
                "css": Utils.find_files_by_postfix(
                    str(directory.absolute()), dict_name, ".css"
                ),
                "js": Utils.find_files_by_postfix(
                    str(directory.absolute()), dict_name, ".js"
                ),
                "data": "",
                "cover": "",
                "cover_url": "",
            }

            # Check for data directory
            data_path = directory.absolute() / "data"
            if data_path.is_dir():
                Utils.DICT_INFO[dict_name]["data"] = str(data_path.absolute())

            # Look for cover image
            for img_file in directory.iterdir():
                if img_file.is_file() and img_file.suffix.lower() in [
                    ".jpg", ".jpeg", ".png", ".gif"
                ]:
                    cover_rel_path = f"{dict_name}/{img_file.name}"
                    Utils.DICT_INFO[dict_name]["cover"] = cover_rel_path
                    Utils.DICT_INFO[dict_name]["cover_url"] = (
                        f"http://127.0.0.1:5959/api/dictionaries/{quote(cover_rel_path)}"
                    )
                    break

        @staticmethod
        def removeDictInfo(dict_name: str) -> None:
            """Remove dictionary metadata from DICT_INFO."""
            Utils.DICT_INFO.pop(dict_name, None)

        @staticmethod
        def _renew_dict_set_option(old_option: List) -> List:
            """Update a dictionary set option with newly discovered dictionaries."""
            old_names = [item["name"] for item in old_option if item["name"] in Utils.DICT_INFO]
            new_options = []

            # Add new dictionaries not in the old set
            for dict_name in Utils.DICT_INFO:
                if dict_name not in old_names:
                    new_options.append({"name": dict_name, "is_enabled": False})

            # Keep existing dictionary settings
            for item in old_option:
                if item["name"] in Utils.DICT_INFO:
                    new_options.append(item)

            return new_options

        @staticmethod
        def renew_dict_set_options() -> None:
            """Refresh all dictionary set options with current dictionary list."""
            old_options: Dict = Utils.DICT_CONFIG["dict_set_options"]
            new_options = {}
            for key, option in old_options.items():
                new_options[key] = Utils.Config._renew_dict_set_option(option)
            Utils.DICT_CONFIG["dict_set_options"] = new_options


def initialize_config() -> None:
    """
    Initialize the application configuration.
    Creates directories, loads config files, applies defaults, and migrates schemas.
    """
    logger.info(f"FstDict support path: {Utils.FSTDICT_SUPPORT_PATH}")

    # Ensure required directories exist
    Utils.createDirIfnotExists(Utils.USER_CONFIG_DIR)
    Utils.createDirIfnotExists(Utils.DATA_PATH)
    Utils.createDirIfnotExists(Utils.DICTIONARYS_PATH)
    Utils.createDirIfnotExists(Utils.RAPID_OCR_MODELS_PATH)

    # Load default config templates
    with open(Utils.DEFAULT_CONFIG_FILE, mode="r", encoding="utf-8") as f:
        Utils.DEFAULT_CONFIG = json.load(f)
    with open(Utils.DEFAULT_DICT_CONFIG_FILE, mode="r", encoding="utf-8") as f:
        Utils.DEFAULT_DICT_CONFIG = json.load(f)

    # Load user configs if they exist
    for config_file, config_attr in [
        (Utils.CONFIG_FILE, "CONFIG"),
        (Utils.DICT_CONFIG_FILE, "DICT_CONFIG"),
    ]:
        if os.path.isfile(config_file):
            with open(config_file, mode="r", encoding="utf-8") as f:
                setattr(Utils, config_attr, json.load(f))
        else:
            setattr(Utils, config_attr, {})

    # Scan dictionaries directory
    dict_path = Path(Utils.DICTIONARYS_PATH)
    for file in dict_path.iterdir():
        if file.is_dir():
            Utils.Config.checkDictInfo(file)

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
            Utils.Config.syncConfigFile(config, file_path)

    migrate_config(
        Utils.CONFIG,
        Utils.DEFAULT_CONFIG,
        Utils.CONFIG_FILE,
        [
            ["schema_version"], ["dict_set_options"],
            ["ocr", "session"],
            ["app", "session"], ["app", "helper_selection", "session"]
        ]
    )
    migrate_config(
        Utils.DICT_CONFIG,
        Utils.DEFAULT_DICT_CONFIG,
        Utils.DICT_CONFIG_FILE,
        []
    )

    # Apply selection monitoring setting to auto-register list
    if Utils.CONFIG["app"]["helper_selection"]["enabled"]:
        Utils.REGISTER_CGEVENT_RIGHT_AFTER_CONNECTION.append("kHandlerTextSelection")

    # Final initialization steps
    Utils.Config.renew_dict_set_options()
    Utils.Config.init_config(Utils.CONFIG)
    Utils.Config.init_dict_config(Utils.DICT_CONFIG)
    Utils.Config.init_shortcut_map()


# Run initialization on module import
initialize_config()
