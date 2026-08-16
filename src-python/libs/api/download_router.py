"""
Router for file download and on-the-fly audio transcoding.
"""
import sys
import asyncio
import subprocess
import urllib.parse
from pathlib import Path

from fastapi import APIRouter, HTTPException
from fastapi.responses import FileResponse, StreamingResponse

from libs.config.paths import DICTIONARIES_DIR, FFMPEG_BINARY, AUDIO_EXTENSIONS
from libs.common.utils import Utils
from libs.log_config import logger

router = APIRouter()

# Limit concurrent ffmpeg processes to avoid resource exhaustion
_ffmpeg_semaphore = asyncio.Semaphore(4)


@router.get("/api/download")
async def download_file(path: str):
    """
    Download a dictionary resource.
    Automatically extracts files from the FST archive on first access.
    Transcodes audio files to MP3 on the fly.
    """
    logger.info(f"Original download path: {path}")
    decoded_path = urllib.parse.unquote(path)
    decoded_path = decoded_path.replace("//", "/")
    logger.info(f"Resolved download path: {decoded_path}")

    file_path = DICTIONARIES_DIR / decoded_path

    # Extract resource from dictionary archive if it does not exist on disk
    if not file_path.is_file():
        try:
            dict_name, _, file_key = decoded_path.split("/", maxsplit=2)
            data_dir = DICTIONARIES_DIR / dict_name / "data"
            Utils.fstd_engine.extract(dict_name, file_key, str(data_dir))

            # Retry with leading slash prefix if still not found
            if not file_path.is_file():
                Utils.fstd_engine.extract_if_exists(dict_name, "/" + file_key, str(data_dir))
        except Exception as e:
            logger.error(f"Failed to extract resource: {e}")
            raise HTTPException(status_code=400, detail="Resource does not exist")

    if not file_path.is_file():
        raise HTTPException(status_code=400, detail="Resource does not exist")

    # Return non-audio files directly
    file_ext = file_path.suffix.lower()
    if file_ext not in AUDIO_EXTENSIONS:
        return FileResponse(path=file_path, filename=file_path.name)

    # Audio file: transcode to MP3 and stream
    async with _ffmpeg_semaphore:
        proc = await _start_ffmpeg_process(file_path)

    async def stream_mp3():
        try:
            while True:
                chunk = await asyncio.to_thread(proc.stdout.read, 65536)  # type: ignore
                if not chunk:
                    break
                yield chunk

            retcode = proc.wait()
            if retcode != 0:
                err = proc.stderr.read(2048).decode("utf-8", errors="ignore")  # type: ignore
                logger.error(f"FFmpeg transcoding failed (code={retcode}): {err}")
        finally:
            if proc.poll() is None:
                proc.terminate()
                proc.wait()

    return StreamingResponse(
        stream_mp3(),
        media_type="audio/mpeg",
        headers={
            "Content-Disposition": f'attachment; filename="{file_path.stem}.mp3"'
        }
    )


async def _start_ffmpeg_process(input_path: Path) -> subprocess.Popen:
    """Start an FFmpeg process to transcode audio to MP3."""
    cmd = [
        str(FFMPEG_BINARY),
        "-y",
        "-i", str(input_path),
        "-c:a", "libmp3lame",
        "-b:a", "96k",
        "-f", "mp3",
        "pipe:1"
    ]

    popen_kwargs = {
        "stdout": subprocess.PIPE,
        "stderr": subprocess.PIPE,
        "stdin": subprocess.DEVNULL
    }

    # Suppress console window on Windows
    if sys.platform == "win32":
        popen_kwargs["creationflags"] = subprocess.CREATE_NO_WINDOW

    try:
        proc = subprocess.Popen(cmd, **popen_kwargs)  # type: ignore
        return proc
    except Exception as e:
        logger.error(f"Failed to start FFmpeg: {e}")
        raise HTTPException(status_code=500, detail="Audio transcoding failed to start")
