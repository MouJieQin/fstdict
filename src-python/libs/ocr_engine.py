from rapidocr import EngineType, LangDet, LangRec, ModelType, OCRVersion, RapidOCR
import sys
import subprocess
from libs.config import UtilsBase
from libs.log_config import logger


class OcrEngine:
    def __init__(self, lang_type: str):
        self._rec_lang_type = ""
        self.set_langage_type(lang_type)

    def set_langage_type(self, lang_type: str):
        if self._rec_lang_type == lang_type:
            return
        self._rec_lang_type = lang_type
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

    def ocr_img(self, img: str) -> str:
        txt: str = ""
        result = self._engine(img)
        for line in result.to_json():  # type: ignore
            if txt:
                txt += " " + (line["txt"])
            else:
                txt += (line["txt"])
        return txt

    def _ocr_on_macos(self) -> str:
        screenshot_path = UtilsBase.IMA_PATH_FOR_OCR
        ret = subprocess.run(["screencapture", "-i", "-o", "-t", "png", screenshot_path], check=True)
        if ret.returncode != 0:
            return ""
        return self.ocr_img(screenshot_path)

    def ocr(self) -> str:
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
