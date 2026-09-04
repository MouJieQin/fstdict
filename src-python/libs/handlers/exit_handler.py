import os
import sys
import asyncio
import json
from libs.log_config import logger
from libs.config.app_config import Utils
from fastapi import WebSocket


class ExitHandler:
    @staticmethod
    def hard_restart():
        os.execv(sys.executable, [sys.executable] + sys.argv)

    @staticmethod
    async def _notify_cgevent_server_exit():
        if Utils.cgevent_ws_client:
            await Utils.cgevent_ws_client.send_exit_request()

    @staticmethod
    async def _send_exit_request(ws: WebSocket):
        msg = {
            "type": "exit_request",
            "data": {}
        }
        await ws.send_text(json.dumps(msg))

    @staticmethod
    async def _notify_tauri_main_exit():
        if Utils.fstdict_main_websocket:
            await ExitHandler._send_exit_request(Utils.fstdict_main_websocket)

    @staticmethod
    async def _notify_tauri_helper_exit():
        if Utils.fstdict_helper_websocket:
            await ExitHandler._send_exit_request(Utils.fstdict_helper_websocket)

    @staticmethod
    async def _wait_clean_and_exit(force_exit=False):
        Utils.cgevent_ws_client.set_do_not_retry()
        await ExitHandler._notify_tauri_helper_exit()
        await ExitHandler._notify_cgevent_server_exit()
        await ExitHandler._notify_tauri_main_exit()
        logger.info("All connections marked for shutdown. Exiting.")
        if force_exit or Utils.IS_FROZEN:
            os._exit(0)
        else:
            ExitHandler.hard_restart()

    @staticmethod
    def clean_and_exit(force_exit=False):
        try:
            loop = asyncio.get_running_loop()
            loop.create_task(ExitHandler._wait_clean_and_exit(force_exit))
        except RuntimeError:
            logger.error("No async loop, sync fallback clean")
