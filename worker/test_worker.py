import sys
import types
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from worker.rain_worker import (
    MAX_AUDIO_BYTES,
    RUNTIME_DEPENDENCIES,
    FunAsrNanoAdapter,
    ParaformerZhAdapter,
    SenseVoiceAdapter,
    adapter_needs_load,
    create_adapter,
    error_code,
    extract_text,
    validate_audio_bytes,
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

    def test_fun_asr_nano_uses_the_tensor_input_contract(self) -> None:
        fake_torch = types.SimpleNamespace(from_numpy=lambda audio: ("tensor", audio))
        with patch.dict(sys.modules, {"torch": fake_torch}):
            options = FunAsrNanoAdapter().generate_options("audio", 16000)

        self.assertEqual(options["input"], [("tensor", "audio")])
        self.assertEqual(options["language"], "中文")
        self.assertNotIn("fs", options)

    def test_all_adapters_share_load_transcribe_unload_contract(self) -> None:
        fake_funasr = types.SimpleNamespace(AutoModel=FakeModel)
        fake_numpy = types.SimpleNamespace(
            frombuffer=lambda *_args, **_kwargs: FakeArray(),
            float32=float,
        )
        fake_torch = types.SimpleNamespace(from_numpy=lambda audio: audio)
        with TemporaryDirectory() as temporary_directory, patch.dict(
            sys.modules,
            {"funasr": fake_funasr, "numpy": fake_numpy, "torch": fake_torch},
        ):
            for adapter_type, expected_type in (
                ("sensevoice", SenseVoiceAdapter),
                ("fun_asr_nano", FunAsrNanoAdapter),
                ("paraformer_zh", ParaformerZhAdapter),
            ):
                adapter = create_adapter(adapter_type)
                self.assertIsInstance(adapter, expected_type)
                adapter.load(temporary_directory, "cpu")
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

    def test_model_cache_includes_the_resolved_device(self) -> None:
        adapter = SenseVoiceAdapter()
        adapter.model = object()
        adapter.model_path = "model"
        adapter.device = "cpu"

        self.assertFalse(
            adapter_needs_load(adapter, "sensevoice", "sensevoice", "model", "cpu")
        )
        self.assertTrue(
            adapter_needs_load(adapter, "sensevoice", "sensevoice", "model", "cuda")
        )

    def test_audio_limit_and_runtime_dependencies_match_the_host_contract(self) -> None:
        self.assertEqual(validate_audio_bytes(MAX_AUDIO_BYTES), MAX_AUDIO_BYTES)
        with self.assertRaisesRegex(ValueError, "AUDIO_TOO_LARGE"):
            validate_audio_bytes(MAX_AUDIO_BYTES + 1)
        with self.assertRaisesRegex(ValueError, "AUDIO_TOO_LARGE"):
            validate_audio_bytes(-1)
        self.assertEqual(
            RUNTIME_DEPENDENCIES,
            ("funasr", "modelscope", "numpy", "torch", "torchaudio", "transformers"),
        )

    def test_errors_are_normalized(self) -> None:
        self.assertEqual(error_code(FileNotFoundError("missing")), "MODEL_NOT_INSTALLED")
        self.assertEqual(error_code(RuntimeError("CUDA out of memory")), "DEVICE_OUT_OF_MEMORY")
        self.assertEqual(error_code(ValueError("AUDIO_TOO_LARGE")), "AUDIO_TOO_LARGE")


if __name__ == "__main__":
    unittest.main()
