import os
import asyncio
import psutil
from libs.log_config import logger
from libs.common.utils import Utils


class ExitHandler:
    @staticmethod
    def is_pid_alive(pid: int) -> bool:
        return psutil.pid_exists(pid)

    @staticmethod
    def check_tauri_main_alive() -> bool:
        return ExitHandler.is_pid_alive(Utils.tauri_main_pid) if Utils.tauri_main_pid != 0 else False

    @staticmethod
    async def _notify_cgevent_server_exit():
        if Utils.cgevent_ws_client:
            await Utils.cgevent_ws_client.send_exit_request()

    @staticmethod
    async def _wait_clean_and_exit():
        Utils.iwin_ws_client.set_do_not_retry()
        Utils.cgevent_ws_client.set_do_not_retry()
        await ExitHandler._notify_cgevent_server_exit()
        logger.info("All connections marked for shutdown. Exiting.")
        os._exit(0)

    @staticmethod
    def clean_and_exit():
        try:
            loop = asyncio.get_running_loop()
            loop.create_task(ExitHandler._wait_clean_and_exit())
        except RuntimeError:
            logger.error("No async loop, sync fallback clean")
