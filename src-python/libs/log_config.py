"""
Logging configuration for FstDict server.
Features:
- Colored console output in development
- Smart file rotation (daily + size-based)
- PyInstaller frozen environment compatibility
- Cross-platform log directory via platformdirs
"""
import sys
import io
import os
import logging
import platformdirs
from pathlib import Path
from datetime import datetime
from logging.handlers import BaseRotatingHandler
from colorama import init, Fore, Style

init(autoreset=True)

# Detect PyInstaller packaged binary
IS_FROZEN = getattr(sys, "frozen", False)


def _get_log_file_path() -> Path:
    """Get the full path to the log file, creating directories as needed."""
    APP_NAME = "FstDict"
    APP_AUTHOR = "qinmoujie"
    log_dir = Path(platformdirs.user_log_dir(APP_NAME, APP_AUTHOR))
    log_dir.mkdir(exist_ok=True, parents=True)
    return log_dir / "server.log"


# Fix for Windows --noconsole mode where stdout/stderr may be None
if IS_FROZEN:
    _dummy_buffer = io.BytesIO()
    sys.stdout = io.TextIOWrapper(_dummy_buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(_dummy_buffer, encoding="utf-8", errors="replace")
else:
    # Ensure proper UTF-8 encoding for console output in dev mode
    if sys.stdout is not None and hasattr(sys.stdout, "buffer"):
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    if sys.stderr is not None and hasattr(sys.stderr, "buffer"):
        sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")


class _ColoredConsoleFormatter(logging.Formatter):
    """Formatter for colored console output."""

    LEVEL_COLORS = {
        logging.DEBUG: Fore.LIGHTBLACK_EX,
        logging.INFO: Fore.GREEN,
        logging.WARNING: Fore.YELLOW,
        logging.ERROR: Fore.RED,
        logging.CRITICAL: Fore.LIGHTRED_EX,
    }
    RESET = Style.RESET_ALL

    def format(self, record: logging.LogRecord) -> str:
        time_str = self.formatTime(record, datefmt="%Y-%m-%d %H:%M:%S")
        color = self.LEVEL_COLORS.get(record.levelno, "")
        level_str = f"{color}[{record.levelname.lower()}]{self.RESET}"
        thread_str = f"[thread {record.thread}]"
        location_str = f"[{record.filename}:{record.lineno}]"
        msg = record.getMessage()
        return f"[{time_str}] {level_str} {thread_str} {location_str} {msg}"


class _DesktopRotatingFileHandler(BaseRotatingHandler):
    """
    Smart log file handler designed for desktop applications.
    - Rotates on date change at application startup
    - Rotates when file size exceeds maxBytes
    - Keeps only backupCount number of archived logs
    """

    def __init__(self, filename, maxBytes=10 * 1024 * 1024, backupCount=3, encoding="utf-8"):
        self.baseFilename = os.path.abspath(filename)
        self.maxBytes = maxBytes
        self.backupCount = backupCount
        self.encoding = encoding

        # Check for cross-day rotation on startup
        if os.path.exists(self.baseFilename):
            mtime = os.path.getmtime(self.baseFilename)
            file_date = datetime.fromtimestamp(mtime).strftime("%Y-%m-%d")
            current_date = datetime.now().strftime("%Y-%m-%d")
            if file_date != current_date:
                self._rotate_by_date(file_date)

        super().__init__(self.baseFilename, mode="a", encoding=self.encoding)

    def _rotate_by_date(self, file_date: str) -> None:
        """Rename log file with date suffix when day changes."""
        rotated_name = f"{self.baseFilename}.{file_date}"
        if os.path.exists(rotated_name):
            rotated_name += f"_{int(os.path.getmtime(self.baseFilename))}"
        try:
            os.rename(self.baseFilename, rotated_name)
        except Exception:
            pass
        self._prune_old_logs()

    def doRollover(self) -> None:
        """Triggered when file size exceeds maxBytes."""
        if self.stream:
            self.stream.close()
            self.stream = None  # type: ignore

        timestamp = datetime.now().strftime("%Y-%m-%d_%H%M%S")
        rotated_name = f"{self.baseFilename}.{timestamp}"

        try:
            os.rename(self.baseFilename, rotated_name)
        except Exception:
            pass

        self._prune_old_logs()

        if not self.delay:
            self.stream = self._open()

    def shouldRollover(self, record: logging.LogRecord) -> bool:
        """Check if file size will exceed limit with this record."""
        if self.maxBytes <= 0:
            return False
        if self.stream is None:
            self.stream = self._open()
        try:
            self.stream.seek(0, 2)
            if self.stream.tell() + len(record.getMessage()) >= self.maxBytes:
                return True
        except Exception:
            pass
        return False

    def _prune_old_logs(self) -> None:
        """Delete oldest archived logs beyond backupCount limit."""
        dir_name = os.path.dirname(self.baseFilename)
        base_name = os.path.basename(self.baseFilename)

        log_files = sorted(
            Path(dir_name).glob(f"{base_name}.*"),
            key=os.path.getmtime
        )

        if len(log_files) > self.backupCount:
            for file_path in log_files[:-self.backupCount]:
                try:
                    file_path.unlink()
                except Exception:
                    pass

    def emit(self, record: logging.LogRecord) -> None:
        """Safely emit a log record."""
        try:
            if self.shouldRollover(record):
                self.doRollover()
            msg = self.format(record)
            self.stream.write(msg + self.terminator)
            self.flush()
        except Exception:
            self.handleError(record)


def setup_logger() -> logging.Logger:
    """Configure and return the root logger."""
    logger = logging.getLogger()
    logger.handlers.clear()

    log_level_name = os.getenv("LOG_LEVEL", "INFO")
    log_level = getattr(logging, log_level_name, logging.INFO)
    logger.setLevel(log_level)

    # File handler formatter (plain text, no colors)
    file_formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] [thread %(thread)s] [%(filename)s:%(lineno)d] %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )

    # Smart file handler with size + date rotation
    file_handler = _DesktopRotatingFileHandler(
        filename=_get_log_file_path(),
        maxBytes=5 * 1024 * 1024,
        backupCount=1,
        encoding="utf-8"
    )
    file_handler.setFormatter(file_formatter)
    logger.addHandler(file_handler)

    # Console output only in development mode
    if not IS_FROZEN:
        console_handler = logging.StreamHandler(stream=sys.stdout)
        console_handler.setFormatter(_ColoredConsoleFormatter())
        logger.addHandler(console_handler)

    return logger


# Initialize logger on module import
logger = setup_logger()
