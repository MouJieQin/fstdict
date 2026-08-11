from rapidocr import EngineType, LangDet, LangRec, ModelType, OCRVersion, RapidOCR
import sys
import subprocess
from libs.common import Utils
from libs.log_config import logger


class OcrEngine:
    def __init__(self):
        self._rec_lang_type = "ch"
        self._is_ocring = False
        self._set_lang_type_imple(self._rec_lang_type)

    def set_language_type(self, lang_type: str):
        if self._rec_lang_type == lang_type:
            return
        self._rec_lang_type = lang_type
        self._set_lang_type_imple(lang_type)

    def _set_lang_type_imple(self, lang_type: str):
        if lang_type == "korean":
            self._engine = RapidOCR(
                params={
                    "Det.engine_type": EngineType.ONNXRUNTIME,
                    "Det.lang_type": LangDet.CH,
                    "Det.model_type": ModelType.SMALL,
                    "Det.ocr_version": OCRVersion.PPOCRV6,

                    "Rec.engine_type": EngineType.ONNXRUNTIME,
                    "Rec.lang_type": lang_type,
                    "Rec.model_type": ModelType.MOBILE,
                    "Rec.ocr_version": OCRVersion.PPOCRV5,
                }
            )
        else:
            self._engine = RapidOCR(
                params={
                    "Det.engine_type": EngineType.ONNXRUNTIME,
                    "Det.lang_type": LangDet.CH,
                    "Det.model_type": ModelType.SMALL,
                    "Det.ocr_version": OCRVersion.PPOCRV6,

                    "Rec.engine_type": EngineType.ONNXRUNTIME,
                    "Rec.lang_type": lang_type,
                    "Rec.model_type": ModelType.SMALL,
                    "Rec.ocr_version": OCRVersion.PPOCRV6,

                    # "Cls.engine_type": EngineType.ONNXRUNTIME,
                    # "Cls.lang_type": LangDet.CH,
                    # "Cls.model_type": ModelType.MOBILE,
                    # "Cls.ocr_version": OCRVersion.PPOCRV4,
                }
            )

    def _ocr_img(self, img: str) -> str:
        txt: str = ""
        result = self._engine(img)
        for line in result.to_json():  # type: ignore
            if txt:
                txt += " " + (line["txt"])
            else:
                txt += (line["txt"])
        return txt

    def _ocr_on_macos(self) -> str:
        screenshot_path = Utils.IMA_PATH_FOR_OCR
        ret = subprocess.run(["screencapture", "-i", "-o", "-t", "png", screenshot_path], check=True)
        if ret.returncode != 0:
            return ""
        return self._ocr_img(screenshot_path)

    def _get_ocr_session_id(self):
        return Utils.CONFIG["ocr"]["session"]["id"]

    def _get_ocr_lang_type(self):
        session_id = self._get_ocr_session_id()
        config = Utils.db.get_session_config(session_id)
        if not config:
            return self._rec_lang_type
        lang = config.get("ocr_lang_type", self._rec_lang_type)
        return Utils.CONFIG["ocr"]["lang_types"].get(lang, "ch")

    def is_ocring(self):
        return self._is_ocring

    def ocr(self) -> str:
        if self._is_ocring:
            return ""
        self._is_ocring = True
        self.set_language_type(self._get_ocr_lang_type())
        try:
            if sys.platform == "darwin":
                return self._ocr_on_macos()
            elif sys.platform.startswith("win"):
                return ""
            elif sys.platform.startswith("linux"):
                return ""
        except Exception as e:
            logger.exception(f"OCR:{e}")
            return ""
        finally:
            self._is_ocring = False
