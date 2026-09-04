import sys
import os
import subprocess
import shutil
from pathlib import Path


class UtilBase:
    """Global utility"""

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
    def removeFileIfExists(path: str) -> None:
        if os.path.exists(path):
            os.remove(path)

    @staticmethod
    def find_files_by_postfix(root_dir: str, dict_name: str, postfix: str) -> list[str]:
        """Find all files with a given extension in a directory."""
        files = []
        for item in Path(root_dir).iterdir():
            if item.is_file() and item.name.lower().endswith(postfix):
                files.append(f"{dict_name}/{item.name}")
        return files

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
