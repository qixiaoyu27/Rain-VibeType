import sys
import types
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from worker.rain_worker import (
    FunAsrNanoAdapter,
    ParaformerZhAdapter,
    SenseVoiceAdapter,
    create_adapter,
    error_code,
    extract_text,
    transcription_should_commit,
)


class FakeModel:
    def __init__(self, **kwargs):
        self.kwargs = kwargs

    def generate(self, **_kwargs):
        return [{"text": " 本地识别 "}]


class FakeArray:
    def astype(self, *_args):
        return self

    def __truediv__(self, _value):
        return self


class WorkerContractTest(unittest.TestCase):
    def test_extract_text_accepts_funasr_list_shape(self) -> None:
        self.assertEqual(extract_text([{"text": " 本地识别 "}]), "本地识别")

    def test_all_adapters_share_load_transcribe_unload_contract(self) -> None:
        fake_funasr = types.SimpleNamespace(AutoModel=FakeModel)
        fake_numpy = types.SimpleNamespace(
            frombuffer=lambda *_args, **_kwargs: FakeArray(),
            float32=float,
        )
        with TemporaryDirectory() as temporary_directory, patch.dict(
            sys.modules, {"funasr": fake_funasr, "numpy": fake_numpy}
        ):
            for adapter_type, expected_type in (
                ("sensevoice", SenseVoiceAdapter),
                ("fun_asr_nano", FunAsrNanoAdapter),
                ("paraformer_zh", ParaformerZhAdapter),
            ):
                adapter = create_adapter(adapter_type)
                self.assertIsInstance(adapter, expected_type)
                adapter.load(temporary_directory, "cpu", {})
                self.assertTrue(adapter.health_check())
                self.assertEqual(adapter.transcribe(b"\0\0", 16000), "本地识别")
                self.assertEqual(adapter.transcribe(b"", 16000), "")
                adapter.unload()
                self.assertFalse(adapter.health_check())
                self.assertIsNone(adapter.model)
                self.assertEqual(adapter.model_path, "")

    def test_unknown_adapter_is_rejected(self) -> None:
        with self.assertRaises(ValueError):
            create_adapter("unknown")

    def test_cancelled_request_never_commits_and_errors_are_normalized(self) -> None:
        self.assertFalse(transcription_should_commit("request-1", {"request-1"}))
        self.assertTrue(transcription_should_commit("request-2", {"request-1"}))
        self.assertEqual(error_code(FileNotFoundError("missing")), "MODEL_NOT_INSTALLED")
        self.assertEqual(error_code(RuntimeError("CUDA out of memory")), "DEVICE_OUT_OF_MEMORY")


if __name__ == "__main__":
    unittest.main()
