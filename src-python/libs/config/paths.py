"""
Centralized path definitions for the application.
All paths are resolved once at import time.
"""
import sys
import platformdirs
from pathlib import Path

APP_NAME = "FstDict"
APP_AUTHOR = "qinmoujie"

# Base directory: handles both frozen (PyInstaller) and development modes
BASE_DIR = Path(sys._MEIPASS) if getattr(sys, "frozen", False) else Path(__file__).resolve().parent.parent.parent  # type: ignore

# System directories
APP_SUPPORT_PATH = Path(platformdirs.user_data_dir(APP_NAME, APP_AUTHOR))
APP_LOG_PATH = Path(platformdirs.user_log_dir(APP_NAME, APP_AUTHOR))
APP_CACHE_PATH = Path(platformdirs.user_cache_dir(APP_NAME, APP_AUTHOR))

# Application storage
STORAGE_PATH = APP_SUPPORT_PATH / "Storage"
CONFIG_DIR = STORAGE_PATH / "config"
DATA_DIR = STORAGE_PATH / "data"
DICTIONARIES_DIR = STORAGE_PATH / "dictionaries"

# Config files
CONFIG_FILE = CONFIG_DIR / "config.json"
CGEVENT_CONFIG_FILE = CONFIG_DIR / "cgevent_config.json"
DICT_CONFIG_FILE = CONFIG_DIR / "dict_config.json"
ANKI_CONFIG_FILE = CONFIG_DIR / "anki_config.json"

# Default config templates (shipped with the app)
DEFAULT_CONFIG_FILE = BASE_DIR / "config.json"
DEFAULT_CGEVENT_CONFIG_FILE = BASE_DIR / "cgevent_config.json"
DEFAULT_DICT_CONFIG_FILE = BASE_DIR / "dict_config.json"

# Binary paths
FFMPEG_BINARY = BASE_DIR / "ffmpeg" / ("fstdict-ffmpeg.exe" if sys.platform.startswith("win") else "fstdict-ffmpeg")

# OCR paths
RAPIDOCR_MODELS_DIR = DATA_DIR / "rapidocr" / "models"
OCR_SCREENSHOT_PATH = APP_CACHE_PATH / "screenshot_for_ocr.png"

# Database paths
FSTDICT_DB_PATH = DATA_DIR / "fstdict.db"
DICT_DB_PATH = DATA_DIR / "dict.db"

# Supported audio file extensions for transcoding
AUDIO_EXTENSIONS = {".spx", ".ogg", ".wav", ".mp3", ".amr", ".aac", ".flac", ".m4a", ".opus", ".wma"}
