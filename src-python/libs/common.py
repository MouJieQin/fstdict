import sys
import subprocess
import os
from libs.config import UtilsBase
from libs.websocket_client import WsClient
from libs.fstdict_database import FstDictDatabase
from libs.dict_database import DictDatabase


class Utils(UtilsBase):
    """通用工具类"""

    db = FstDictDatabase(UtilsBase.FSTDICT_DATABASE_PATH)
    dict_db = DictDatabase(UtilsBase.DICT_DATABASE_PATH)
    iwin_ws_client: WsClient

    @staticmethod
    def delete_dictionary(dict_name: str) -> None:
        """
        删除指定字典文件夹
        dict_name (str): 字典名称
        return (bool): 是否执行成功
        """
        dict_dir = Utils.getDictDir(dict_name)
        Utils.removeDirIfExists(dict_dir)
        Utils.Config.removeDictInfo(dict_name)
        Utils.Config.renew_dict_set_options()

    @staticmethod
    def reveal_dict_in_file_manager(dict_name: str) -> bool:
        """
        跨平台打开文件管理器并定位选中指定字典文件夹
        dict_name (str): 字典名称
        return (bool): 是否执行成功
        """
        dict_path = Utils.getDictPath(dict_name)
        return Utils.reveal_in_file_manager(dict_path)

    @staticmethod
    def reveal_in_file_manager(file_path: str) -> bool:
        """
        跨平台打开文件管理器并定位选中指定文件
        :param file_path: 目标文件/文件夹绝对路径
        :return: 是否执行成功
        """
        file_path = os.path.abspath(file_path)
        if not os.path.exists(file_path):
            return False

        try:
            if sys.platform == "darwin":
                # macOS: open -R 自动在 Finder 中选中文件
                subprocess.run(["open", "-R", file_path], check=True)

            elif sys.platform.startswith("win"):
                # Windows: explorer /select, 注意路径用正斜杠或转义
                # 必须用 shell=True，explorer 对参数格式要求特殊
                subprocess.run(f'explorer /select,"{file_path}"', shell=True)

            elif sys.platform.startswith("linux"):
                # Linux：优先尝试常见文件管理器
                # dbus 方式最通用（Nautilus/Nemo/Thunar 等大多支持）
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
                    # 兜底：直接打开所在文件夹
                    folder = os.path.dirname(file_path)
                    subprocess.run(["xdg-open", folder], check=True)

            else:
                return False

            return True
        except Exception:
            return False
