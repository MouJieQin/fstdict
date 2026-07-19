import sys
import io
import os
import logging
import platformdirs
from pathlib import Path
from logging.handlers import TimedRotatingFileHandler
from colorama import init, Fore, Style

init(autoreset=True)

# Detect PyInstaller frozen packaged binary
IS_FROZEN = getattr(sys, "frozen", False)


def get_log_path() -> Path:
    APP_NAME = "FstDict"
    APP_AUTHOR = "qinmoujie"
    log_path = Path(platformdirs.user_log_dir(APP_NAME, APP_AUTHOR))
    log_path.mkdir(exist_ok=True, parents=True)
    return log_path / "server.log"


# Fix Windows --noconsole None stdout crash
if IS_FROZEN:
    # Mock dummy text stream to avoid isatty() / buffer errors for uvicorn
    dummy_buffer = io.BytesIO()
    sys.stdout = io.TextIOWrapper(dummy_buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(dummy_buffer, encoding="utf-8", errors="replace")
else:
    # Only rewrite stdout encoding in development environment
    if sys.stdout is not None and hasattr(sys.stdout, "buffer"):
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    if sys.stderr is not None and hasattr(sys.stderr, "buffer"):
        sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")


class CustomFormatter(logging.Formatter):
    LEVEL_COLOR = {
        logging.DEBUG: Fore.LIGHTBLACK_EX,
        logging.INFO: Fore.GREEN,
        logging.WARNING: Fore.YELLOW,
        logging.ERROR: Fore.RED,
        logging.CRITICAL: Fore.LIGHTRED_EX,
    }
    RESET = Style.RESET_ALL

    def format(self, record):
        time_str = self.formatTime(record, datefmt="%Y-%m-%d %H:%M:%S")
        color = self.LEVEL_COLOR.get(record.levelno, "")
        level_str = f"{color}[{record.levelname.lower()}]{self.RESET}"
        thread_str = f"[thread {record.thread}]"
        file_line_str = f"[{record.filename}:{record.lineno}]"
        msg = record.getMessage()

        full_line = f"[{time_str}] {level_str} {thread_str} {file_line_str} {msg}"
        return full_line


def setup_logger():
    logger = logging.getLogger()
    logger.handlers.clear()

    log_level_name = os.getenv("LOG_LEVEL", "INFO")
    log_level = getattr(logging, log_level_name, logging.INFO)
    logger.setLevel(log_level)

    # File handler: Timed daily rotation, store in system Logs folder
    file_formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] [thread %(thread)s] [%(filename)s:%(lineno)d] %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )
    file_handler = TimedRotatingFileHandler(
        filename=get_log_path(), when="D", interval=1, backupCount=3, encoding="utf-8"
    )
    file_handler.setFormatter(file_formatter)
    logger.addHandler(file_handler)

    # Only attach console StreamHandler if NOT frozen
    # Frozen Windows --noconsole uses dummy stream, skip colored console output
    if not IS_FROZEN:
        console_handler = logging.StreamHandler(stream=sys.stdout)
        console_handler.setFormatter(CustomFormatter())
        logger.addHandler(console_handler)

    return logger


logger = setup_logger()
