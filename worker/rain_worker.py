from __future__ import annotations

import contextlib
import gc
import importlib.util
import json
import sys
import time
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Any

MAX_AUDIO_BYTES = 3_600 * 16_000 * 2
RUNTIME_DEPENDENCIES = (
    "funasr",
    "modelscope",
    "numpy",
    "torch",
    "torchaudio",
    "transformers",
)


def emit(event: str, request_id: str | None = None, **payload: Any) -> None:
    message = {"event": event, **payload}
    if request_id is not None:
        message["request_id"] = request_id
    sys.stdout.buffer.write(json.dumps(message, ensure_ascii=False).encode("utf-8") + b"\n")
    sys.stdout.buffer.flush()


def extract_text(result: Any) -> str:
    if isinstance(result, list) and result:
        result = result[0]
    if not isinstance(result, dict):
        return ""
    text = str(result.get("text", "")).strip()
    try:
        from funasr.utils.postprocess_utils import rich_transcription_postprocess

        return rich_transcription_postprocess(text).strip()
    except ImportError:
        return text


class AsrAdapter(ABC):
    def __init__(self) -> None:
        self.model: Any = None
        self.model_path = ""
        self.device = ""

    def load(self, model_path: str, device: str) -> None:
        path = Path(model_path)
        if not path.is_dir():
            raise FileNotFoundError(f"模型目录不存在：{path}")
        from funasr import AutoModel

        with contextlib.redirect_stdout(sys.stderr):
            self.model = AutoModel(model=str(path), device=device, **self.load_options())
        self.model_path = str(path)
        self.device = device

    def transcribe(self, pcm: bytes, sample_rate: int) -> str:
        if self.model is None:
            raise RuntimeError("模型尚未加载")
        if not pcm:
            return ""
        import numpy as np

        audio = np.frombuffer(pcm, dtype="<i2").astype(np.float32) / 32768.0
        with contextlib.redirect_stdout(sys.stderr):
            result = self.model.generate(**self.generate_options(audio, sample_rate))
        return extract_text(result)

    def unload(self) -> None:
        self.model = None
        self.model_path = ""
        device = self.device
        self.device = ""
        gc.collect()
        if device.startswith("cuda"):
            try:
                import torch

                torch.cuda.empty_cache()
            except (ImportError, RuntimeError):
                pass

    def health_check(self) -> bool:
        return self.model is not None

    @abstractmethod
    def load_options(self) -> dict[str, Any]:
        raise NotImplementedError

    @abstractmethod
    def generate_options(self, audio: Any, sample_rate: int) -> dict[str, Any]:
        raise NotImplementedError


class SenseVoiceAdapter(AsrAdapter):
    def load_options(self) -> dict[str, Any]:
        return {"trust_remote_code": True, "disable_update": True}

    def generate_options(self, audio: Any, sample_rate: int) -> dict[str, Any]:
        return {
            "input": audio,
            "fs": sample_rate,
            "language": "auto",
            "use_itn": True,
            "batch_size_s": 60,
        }


class FunAsrNanoAdapter(AsrAdapter):
    def load_options(self) -> dict[str, Any]:
        return {"trust_remote_code": True, "disable_update": True}

    def generate_options(self, audio: Any, sample_rate: int) -> dict[str, Any]:
        import torch

        return {
            "input": [torch.from_numpy(audio)],
            "cache": {},
            "batch_size": 1,
            "language": "中文",
            "itn": True,
        }


class ParaformerZhAdapter(AsrAdapter):
    def load_options(self) -> dict[str, Any]:
        return {"disable_update": True}

    def generate_options(self, audio: Any, sample_rate: int) -> dict[str, Any]:
        return {"input": audio, "fs": sample_rate, "batch_size_s": 300}


def create_adapter(adapter_type: str) -> AsrAdapter:
    adapters: dict[str, type[AsrAdapter]] = {
        "sensevoice": SenseVoiceAdapter,
        "fun_asr_nano": FunAsrNanoAdapter,
        "paraformer_zh": ParaformerZhAdapter,
    }
    try:
        return adapters[adapter_type]()
    except KeyError as error:
        raise ValueError(f"不支持的模型适配器：{adapter_type}") from error


def resolve_device(device: str) -> str:
    if device == "cpu":
        return "cpu"
    if device == "cuda":
        return "cuda:0"
    if device != "auto":
        raise ValueError(f"无效推理设备：{device}")
    import torch

    return "cuda:0" if torch.cuda.is_available() else "cpu"


def error_code(error: Exception) -> str:
    message = str(error).lower()
    if "audio_too_large" in message:
        return "AUDIO_TOO_LARGE"
    if isinstance(error, FileNotFoundError):
        return "MODEL_NOT_INSTALLED"
    if "out of memory" in message or "cuda" in message and "memory" in message:
        return "DEVICE_OUT_OF_MEMORY"
    if "model" in message or "weight" in message or "config" in message:
        return "MODEL_INTEGRITY_FAILED"
    return "WORKER_CRASHED"


def adapter_needs_load(
    adapter: AsrAdapter | None,
    adapter_type: str,
    requested_adapter: str,
    model_path: str,
    device: str,
) -> bool:
    return (
        adapter is None
        or adapter_type != requested_adapter
        or adapter.model_path != model_path
        or adapter.device != device
        or not adapter.health_check()
    )


def validate_audio_bytes(value: Any) -> int:
    audio_bytes = int(value)
    if not 0 <= audio_bytes <= MAX_AUDIO_BYTES:
        raise ValueError(f"AUDIO_TOO_LARGE：录音数据必须在 0 到 {MAX_AUDIO_BYTES} 字节之间")
    return audio_bytes


def read_message() -> tuple[dict[str, Any], bytes] | None:
    line = sys.stdin.buffer.readline()
    if not line:
        return None
    message = json.loads(line)
    audio_bytes = validate_audio_bytes(message.pop("audio_bytes", 0))
    audio = sys.stdin.buffer.read(audio_bytes) if audio_bytes else b""
    if len(audio) != audio_bytes:
        raise EOFError("音频数据不完整")
    return message, audio


def run() -> None:
    adapter: AsrAdapter | None = None
    adapter_type = ""
    emit("worker_ready")

    while True:
        packet = read_message()
        if packet is None:
            break
        message, audio = packet
        command = message.get("command")
        request_id = str(message.get("request_id", ""))

        try:
            if command == "get_status":
                missing_dependencies = [
                    name
                    for name in RUNTIME_DEPENDENCIES
                    if importlib.util.find_spec(name) is None
                ]
                emit(
                    "status",
                    request_id,
                    model_ready=bool(adapter and adapter.health_check()),
                    model_path=adapter.model_path if adapter else "",
                    device=adapter.device if adapter else "",
                    adapter_type=adapter_type,
                    runtime_ready=not missing_dependencies,
                    missing_dependencies=missing_dependencies,
                )
            elif command == "load_model":
                requested_adapter = str(message["adapter_type"])
                model_path = str(message["model_path"])
                device = resolve_device(str(message.get("device", "auto")))
                if adapter_needs_load(
                    adapter, adapter_type, requested_adapter, model_path, device
                ):
                    if adapter:
                        adapter.unload()
                    adapter = create_adapter(requested_adapter)
                    adapter.load(model_path, device)
                    adapter_type = requested_adapter
                emit("model_ready", request_id, device=adapter.device)
            elif command == "unload_model":
                if adapter:
                    adapter.unload()
                adapter = None
                adapter_type = ""
                emit("model_unloaded", request_id)
            elif command == "transcribe":
                if adapter is None:
                    raise RuntimeError("模型尚未加载")
                started = time.perf_counter()
                text = adapter.transcribe(audio, int(message.get("sample_rate", 16000)))
                emit(
                    "transcription_completed",
                    request_id,
                    text=text,
                    duration_ms=int(len(audio) / 2 / 16),
                    inference_ms=int((time.perf_counter() - started) * 1000),
                )
            elif command == "shutdown":
                if adapter:
                    adapter.unload()
                emit("shutdown_complete", request_id)
                break
            else:
                raise ValueError(f"未知命令：{command}")
        except Exception as error:
            if command == "load_model" and adapter:
                adapter.unload()
                adapter = None
                adapter_type = ""
            emit(
                "transcription_failed" if command == "transcribe" else "worker_error",
                request_id,
                code=error_code(error),
                message=str(error),
            )


if __name__ == "__main__":
    import multiprocessing

    multiprocessing.freeze_support()
    try:
        run()
    except Exception as error:
        emit("worker_error", code=error_code(error), message=str(error))
        raise SystemExit(1)
