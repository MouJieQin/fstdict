"""
Global utility class and shared state.
Extends the config base class with database and runtime state.
"""
import sys
import os
import subprocess

from libs.config.app_config import UtilsBase
from libs.ws_clients.iwin_client import IWinWsClient
from libs.ws_clients.cgevent_client import CgEventWsClient
from libs.core.database import FstDictDatabase


class Utils(UtilsBase):
    """Global utility and shared application state."""

    # Database instance
    db = FstDictDatabase(UtilsBase.FSTDICT_DATABASE_PATH)

    # WebSocket client instances (initialized at app startup)
    iwin_ws_client: IWinWsClient
    cgevent_ws_client: CgEventWsClient

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

    @staticmethod
    def reveal_in_file_manager(file_path: str) -> bool:
        """
        Cross-platform function to open file manager and select a file.
        Returns True on success, False on failure.
        """
        file_path = os.path.abspath(file_path)
        if not os.path.exists(file_path):
            return False

        try:
            if sys.platform == "darwin":
                subprocess.run(["open", "-R", file_path], check=True)

            elif sys.platform.startswith("win"):
                subprocess.run(f'explorer /select,"{file_path}"', shell=True)

            elif sys.platform.startswith("linux"):
                try:
                    subprocess.run([
                        "dbus-send",
                        "--session",
                        "--dest=org.freedesktop.FileManager1",
                        "--type=method_call",
                        "/org/freedesktop/FileManager1",
                        "org.freedesktop.FileManager1.ShowItems",
                        f"array:string:file://{file_path}",
                        "string:"
                    ], check=True)
                except (FileNotFoundError, subprocess.CalledProcessError):
                    folder = os.path.dirname(file_path)
                    subprocess.run(["xdg-open", folder], check=True)
            else:
                return False

            return True
        except Exception:
            return False
