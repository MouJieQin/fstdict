"""
OCR engine wrapper around RapidOCR with cross-platform screenshot support.
All OCR and screenshot operations are blocking; call via asyncio.to_thread().
"""
from rapidocr import EngineType, LangDet, ModelType, OCRVersion, RapidOCR

from libs.config.app_config import Utils
from libs.log_config import logger


class OcrEngine:
    """Optical Character Recognition engine with screenshot capture."""

    def __init__(self):
        self._current_lang = "ch"
        self._is_ocr_active = False
        self._engine: RapidOCR = None  # type: ignore
        self._initialize_engine(self._current_lang)

    def set_language(self, lang_type: str) -> None:
        """Change the recognition language. Reinitializes engine if changed."""
        if self._current_lang == lang_type:
            return
        self._current_lang = lang_type
        self._initialize_engine(lang_type)

    def _initialize_engine(self, lang_type: str) -> None:
        """Initialize the RapidOCR engine with specified language configuration."""
        common_params = {
            "Global.model_root_dir": Utils.RAPID_OCR_MODELS_PATH,
            "Det.engine_type": EngineType.ONNXRUNTIME,
            "Det.lang_type": LangDet.CH,
            "Det.model_type": ModelType.SMALL,
            "Det.ocr_version": OCRVersion.PPOCRV6,
            "Global.use_cls": False,
        }

        if lang_type == "korean":
            params = {
                **common_params,
                "Rec.engine_type": EngineType.ONNXRUNTIME,
                "Rec.lang_type": lang_type,
                "Rec.model_type": ModelType.MOBILE,
                "Rec.ocr_version": OCRVersion.PPOCRV5,
            }
        else:
            params = {
                **common_params,
                "Rec.engine_type": EngineType.ONNXRUNTIME,
                "Rec.lang_type": lang_type,
                "Rec.model_type": ModelType.SMALL,
                "Rec.ocr_version": OCRVersion.PPOCRV6,
            }

        self._engine = RapidOCR(params=params)

    def _get_configured_language(self) -> str:
        """Get the configured OCR language from session settings."""
        session_id = Utils.CONFIG["app"]["windows"]["helper_main"]["session_id"]
        config = Utils.db.get_session_config(session_id)
        if not config:
            return self._current_lang
        lang_key = config.get("ocr_lang_type", self._current_lang)
        return Utils.CONFIG["ocr"]["lang_types"].get(lang_key, "ch")

    def _recognize_image(self, image_path: str) -> str:
        """Run OCR on an image file and return concatenated text."""
        result = self._engine(image_path)
        text_parts = []
        for line in result.to_json():  # type: ignore
            text_parts.append(line["txt"])
        return " ".join(text_parts)

    def ocr(self) -> str:
        """
        Main OCR entry point: captures screenshot and recognizes text.
        Platform-agnostic; returns empty string on unsupported platforms or error.
        """
        try:
            self.set_language(self._get_configured_language())
            return self._recognize_image(Utils.IMA_PATH_FOR_OCR)
        except Exception as e:
            logger.exception(f"OCR operation failed: {e}")
            return ""


# Global singleton instance
ocr_engine = OcrEngine()
