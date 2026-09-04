"""
Router for dictionary file serving with on-the-fly audio transcoding.

Handles static resource delivery for dictionary files, including automatic
extraction from FST archives on first access and real-time MP3 transcoding
for audio assets.
"""
import sys
import asyncio
import subprocess
import urllib.parse
from pathlib import Path

from fastapi import APIRouter, HTTPException
from fastapi.responses import FileResponse, StreamingResponse

from libs.config.paths import DICTIONARIES_DIR, FFMPEG_BINARY, AUDIO_EXTENSIONS
from libs.config.app_config import Utils
from libs.log_config import logger

router = APIRouter()

# Maximum concurrent FFmpeg processes to prevent system resource exhaustion
MAX_CONCURRENT_FFMPEG = 4
_ffmpeg_semaphore = asyncio.Semaphore(MAX_CONCURRENT_FFMPEG)

# Buffer size (bytes) for reading FFmpeg stdout during streaming
FFMPEG_STREAM_CHUNK_SIZE = 65536

# Subdirectory inside each dictionary folder for cached extracted FST files
FST_CACHE_SUBDIR = "data"


@router.get("/api/dictionaries/{file_path:path}")
async def download_file(file_path: str):
    """
    Serve a dictionary resource by relative path.

    Execution flow:
    1. Decode and normalize the requested URL path
    2. Validate path to prevent directory traversal attacks
    3. Return file directly if it already exists on disk
    4. Extract file from FST archive if not yet cached locally
    5. Transcode audio files to MP3 on the fly before streaming
    """
    # Decode URL-encoded characters and normalize duplicate slashes
    decoded_path = urllib.parse.unquote(file_path)
    decoded_path = decoded_path.replace("//", "/")
    logger.info(f"Requested download path: {decoded_path}")

    # Resolve absolute path for direct root-level file lookup
    root_level_path = DICTIONARIES_DIR / decoded_path
    root_level_path = _validate_path_safety(root_level_path)

    # Serve immediately if file exists in the dictionary root directory
    if root_level_path.is_file():
        return _build_file_response(root_level_path)

    # Parse dictionary name and internal file key for FST extraction
    try:
        dict_name, file_key = decoded_path.split("/", maxsplit=1)
    except ValueError as e:
        logger.error(f"Cannot parse dictionary name from path: {e}")
        raise HTTPException(status_code=400, detail="Resource does not exist")

    # Resolve target path inside the dictionary's FST cache directory
    cache_dir = DICTIONARIES_DIR / dict_name / FST_CACHE_SUBDIR
    cached_file_path = cache_dir / file_key
    cached_file_path = _validate_path_safety(cached_file_path)

    # Extract from archive if the file is not yet cached on disk
    if not cached_file_path.is_file():
        _extract_from_fst_archive(dict_name, file_key, cache_dir)

    # Final existence check after extraction attempt
    if not cached_file_path.is_file():
        raise HTTPException(status_code=400, detail="Resource does not exist")

    return _build_file_response(cached_file_path)


def _validate_path_safety(target_path: Path) -> Path:
    """
    Verify that the resolved path stays within the allowed dictionaries directory.

    Prevents directory traversal attacks via '../' sequences in user input.
    Returns the resolved absolute path if valid; raises HTTP 400 otherwise.
    """
    try:
        resolved_path = target_path.resolve()
        allowed_base = DICTIONARIES_DIR.resolve()
        if not str(resolved_path).startswith(str(allowed_base)):
            raise HTTPException(status_code=400, detail="Invalid path")
        return resolved_path
    except Exception as e:
        logger.warning(f"Path security validation failed: {e}")
        raise HTTPException(status_code=400, detail="Invalid path")


def _extract_from_fst_archive(dict_name: str, file_key: str, target_dir: Path) -> None:
    """
    Extract a single file from the FST dictionary archive into the cache directory.

    First attempts extraction with the raw file key. If the file is still not
    found, retries with a leading '/' prefix to handle alternate archive path formats.
    """
    try:
        Utils.fstd_engine.extract(dict_name, file_key, str(target_dir))

        # Retry with leading slash prefix if file still missing after first attempt
        if not (target_dir / file_key).is_file():
            Utils.fstd_engine.extract_if_exists(dict_name, "/" + file_key, str(target_dir))
    except Exception as e:
        logger.error(f"FST resource extraction failed: {e}")
        raise HTTPException(status_code=400, detail="Resource does not exist")


def _build_file_response(file_path: Path):
    """
    Generate the appropriate HTTP response for the requested file.

    Non-audio files are returned directly as a static FileResponse.
    Audio files are transcoded to MP3 on the fly and streamed back.
    """
    file_extension = file_path.suffix.lower()

    # Return non-audio assets directly from disk
    if file_extension not in AUDIO_EXTENSIONS:
        return FileResponse(path=file_path, filename=file_path.name)

    async def stream_transcoded_mp3():
        # Hold semaphore for the full duration of transcoding and streaming
        async with _ffmpeg_semaphore:
            proc = await _start_ffmpeg_process(file_path)

        try:
            while True:
                chunk = await asyncio.to_thread(proc.stdout.read, FFMPEG_STREAM_CHUNK_SIZE)  # type: ignore
                if not chunk:
                    break
                yield chunk

            return_code = proc.wait()
            if return_code != 0:
                error_msg = proc.stderr.read(2048).decode("utf-8", errors="ignore")  # type: ignore
                logger.error(f"FFmpeg transcoding failed (code={return_code}): {error_msg}")
        finally:
            # Clean up process even if the client disconnects mid-stream
            if proc.poll() is None:
                proc.terminate()
                proc.wait()

    return StreamingResponse(
        stream_transcoded_mp3(),
        media_type="audio/mpeg",
        headers={
            "Content-Disposition": f'attachment; filename="{file_path.stem}.mp3"'
        }
    )


async def _start_ffmpeg_process(input_path: Path) -> subprocess.Popen:
    """
    Spawn an FFmpeg subprocess to transcode audio to MP3 format.

    Output is piped to stdout for streaming. On Windows platforms,
    no separate console window is created for the subprocess.
    """
    command = [
        str(FFMPEG_BINARY),
        "-y",
        "-i", str(input_path),
        "-c:a", "libmp3lame",
        "-b:a", "96k",
        "-f", "mp3",
        "pipe:1"
    ]

    popen_options = {
        "stdout": subprocess.PIPE,
        "stderr": subprocess.PIPE,
        "stdin": subprocess.DEVNULL
    }

    # Suppress spawned console window on Windows
    if sys.platform == "win32":
        popen_options["creationflags"] = subprocess.CREATE_NO_WINDOW

    try:
        process = subprocess.Popen(command, **popen_options)  # type: ignore
        return process
    except Exception as e:
        logger.error(f"Failed to launch FFmpeg process: {e}")
        raise HTTPException(status_code=500, detail="Audio transcoding failed to start")
