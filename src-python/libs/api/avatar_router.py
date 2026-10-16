
from fastapi import UploadFile, Form, File, HTTPException, APIRouter
from libs.config.app_config import Utils
from fastapi.responses import JSONResponse, FileResponse
import time
from pathlib import Path
from libs.common.session_manager import SessionManager


router = APIRouter()


class Avatar:
    @staticmethod
    def get_session_avatar_dir(session_id: int):
        return Utils.AVATARS_PATH + f"/{session_id}"

    @staticmethod
    def get_session_avatar_path(session_id: int, filename: str):
        return Avatar.get_session_avatar_dir(session_id) + f"/{filename}"

    @staticmethod
    def remove_session_avatar_file(session_id: int, filename: str):
        Utils.removeFileIfExists(
            Avatar.get_session_avatar_path(session_id, filename)
        )


@router.get("/api/avatar/{file_path:path}")
async def download_file(file_path: str):
    try:
        slices = file_path.split("/")
        session_id = int(slices[0])
        filename = slices[1]
        return FileResponse(path=Avatar.get_session_avatar_path(session_id, filename))
    except FileNotFoundError:
        raise HTTPException(status_code=404, detail="File not found")


@router.post("/api/upload/session_avatar")
async def upload_avatar(
    file: UploadFile = File(...),
    session_id: int = Form(...),
):
    try:
        # 检查文件类型
        if file.content_type is None or not file.content_type.startswith(
            ("image/jpeg", "image/png")
        ):
            raise HTTPException(status_code=400, detail="Only JPG/PNG format are supported")

        # 检查文件大小
        contents = await file.read()
        if len(contents) > 10 * 1024 * 1024:
            raise HTTPException(status_code=400, detail="File size cannot exceed 10MB")

        # 生成唯一文件名
        if file.filename is None:
            raise HTTPException(status_code=400, detail="Filename is empty")

        filename = str(int(time.time() * 1000)) + Path(file.filename).suffix.lower()
        Utils.createDirIfnotExists(Avatar.get_session_avatar_dir(session_id))
        file_path = Avatar.get_session_avatar_path(session_id, filename)

        # 删除旧文件
        config = Utils.db.get_session_config(session_id)
        if config and "avatar" in config:
            old_avatar_filename = config["avatar"]
            if old_avatar_filename:
                Avatar.remove_session_avatar_file(session_id, old_avatar_filename)

        # 保存文件
        with open(file_path, "wb") as f:
            f.write(contents)

        # 返回成功响应
        await SessionManager.update_session_avatar(session_id, filename)
        return {"success": True, "filename": filename}

    except HTTPException as e:
        return JSONResponse(
            status_code=e.status_code, content={"success": False, "detail": e.detail}
        )
    except Exception as e:
        return JSONResponse(
            status_code=500, content={"success": False, "detail": f"Internal server error: {str(e)}"}
        )
