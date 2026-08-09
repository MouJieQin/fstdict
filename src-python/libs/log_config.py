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
    """保持您原有的彩色控制台输出排版格式"""
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


class DesktopSmartRotatingFileHandler(BaseRotatingHandler):
    """
    专为桌面端/Tauri 侧载程序设计的智能日志处理器。
    1. 跨进程重启检测：启动时检查物理修改时间，跨天即自动归档。
    2. 单日大小安全熔断：单日内写满最大限制容量后自动切分额外编号。
    """

    def __init__(self, filename, maxBytes=10 * 1024 * 1024, backupCount=3, encoding="utf-8"):
        self.baseFilename = os.path.abspath(filename)
        self.maxBytes = maxBytes
        self.backupCount = backupCount
        self.encoding = encoding

        # 启动时优先执行一次基于时间的跨天核验
        if os.path.exists(self.baseFilename):
            mtime = os.path.getmtime(self.baseFilename)
            file_date = datetime.fromtimestamp(mtime).strftime("%Y-%m-%d")
            current_date = datetime.now().strftime("%Y-%m-%d")

            if file_date != current_date:
                self.rotate_on_time_trigger(file_date)

        super().__init__(self.baseFilename, mode="a", encoding=self.encoding)

    def rotate_on_time_trigger(self, file_date):
        """跨天直接更名并清理超期文件"""
        rotated_filename = f"{self.baseFilename}.{file_date}"
        if os.path.exists(rotated_filename):
            rotated_filename += f"_{int(os.path.getmtime(self.baseFilename))}"
        try:
            os.rename(self.baseFilename, rotated_filename)
        except Exception:
            pass
        self.prune_old_backups()

    def doRollover(self):
        """单日内大小超出阈值触发的归档切分机制 (兼容标准 Popen 多开调用防护)"""
        if self.stream:
            self.stream.close()
            self.stream = None  # type: ignore

        current_time_str = datetime.now().strftime("%Y-%m-%d_%H%M%S")
        rotated_filename = f"{self.baseFilename}.{current_time_str}"

        try:
            os.rename(self.baseFilename, rotated_filename)
        except Exception:
            pass

        self.prune_old_backups()

        if not self.delay:
            self.stream = self._open()

    def shouldRollover(self, record):
        """流输出前判断文件是否已写满"""
        if self.maxBytes > 0:
            if self.stream is None:
                self.stream = self._open()
            try:
                self.stream.seek(0, 2)
                if self.stream.tell() + len(record.getMessage()) >= self.maxBytes:
                    return True
            except Exception:
                pass
        return False

    def prune_old_backups(self):
        """只保留指定数量的历史归档文件"""
        dir_name = os.path.dirname(self.baseFilename)
        base_name = os.path.basename(self.baseFilename)

        matching_files = sorted(
            Path(dir_name).glob(f"{base_name}.*"),
            key=os.path.getmtime
        )

        if len(matching_files) > self.backupCount:
            files_to_delete = matching_files[:-self.backupCount]
            for file_path in files_to_delete:
                try:
                    file_path.unlink()
                except Exception:
                    pass

    def emit(self, record):
        """安全的日志写出方法"""
        try:
            if self.shouldRollover(record):
                self.doRollover()
            msg = self.format(record)
            stream = self.stream
            stream.write(msg + self.terminator)
            self.flush()
        except Exception:
            self.handleError(record)


def setup_logger():
    logger = logging.getLogger()
    logger.handlers.clear()

    log_level_name = os.getenv("LOG_LEVEL", "INFO")
    log_level = getattr(logging, log_level_name, logging.INFO)
    logger.setLevel(log_level)

    # 保持原有的文件流基础格式排版
    file_formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] [thread %(thread)s] [%(filename)s:%(lineno)d] %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )

    # 替换为带有单日大小限制熔断 (maxBytes=10MB) 的智能桌面端处理器
    file_handler = DesktopSmartRotatingFileHandler(
        filename=get_log_path(),
        maxBytes=5 * 1024 * 1024,  # 单个文件满 10MB 自动分流
        backupCount=1,
        encoding="utf-8"
    )
    file_handler.setFormatter(file_formatter)
    logger.addHandler(file_handler)

    # 非打包冻结环境，且允许 stdout 输出时附加彩色输出
    if not IS_FROZEN:
        console_handler = logging.StreamHandler(stream=sys.stdout)
        console_handler.setFormatter(CustomFormatter())
        logger.addHandler(console_handler)

    return logger


logger = setup_logger()
