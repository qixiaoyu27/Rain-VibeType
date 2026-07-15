#!/usr/bin/env python3
"""Compare the native and Python workers with the same local audio."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
import wave
from pathlib import Path
from typing import BinaryIO

import numpy as np

ROOT = Path(__file__).resolve().parents[1]


def read_audio(path: Path) -> bytes:
    try:
        import soundfile as sf

        samples, sample_rate = sf.read(path, dtype="float32", always_2d=True)
        mono = samples.mean(axis=1)
    except (ImportError, RuntimeError):
        if path.suffix.lower() != ".wav":
            raise RuntimeError("非 WAV 音频需要安装 soundfile") from None
        with wave.open(str(path), "rb") as source:
            if source.getsampwidth() != 2:
                raise RuntimeError("对比脚本只支持 16-bit PCM WAV")
            sample_rate = source.getframerate()
            channels = source.getnchannels()
            samples = np.frombuffer(source.readframes(source.getnframes()), dtype="<i2")
            mono = samples.reshape(-1, channels).mean(axis=1).astype(np.float32) / 32768
    if sample_rate != 16_000:
        output_size = round(len(mono) * 16_000 / sample_rate)
        mono = np.interp(
            np.arange(output_size) / 16_000,
            np.arange(len(mono)) / sample_rate,
            mono,
        )
    return (np.clip(mono, -1, 1) * 32767).astype("<i2").tobytes()


def send(stream: BinaryIO, message: dict[str, object], audio: bytes = b"") -> None:
    message = {**message, "audio_bytes": len(audio)}
    stream.write(json.dumps(message, ensure_ascii=False).encode("utf-8") + b"\n")
    if audio:
        stream.write(audio)
    stream.flush()


def wait_for(stream: BinaryIO, event: str, request_id: str = "") -> dict[str, object]:
    while line := stream.readline():
        message = json.loads(line)
        if message.get("event") in {"worker_error", "transcription_failed"}:
            raise RuntimeError(str(message.get("message", message)))
        if message.get("event") == event and (
            not request_id or message.get("request_id") == request_id
        ):
            return message
    raise RuntimeError(f"Worker exited before event: {event}")


def run_worker(command: list[str], model: Path, audio: bytes) -> dict[str, object]:
    process = subprocess.Popen(
        command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=None,
        cwd=ROOT,
    )
    assert process.stdin and process.stdout
    try:
        wait_for(process.stdout, "worker_ready")
        load_started = time.perf_counter()
        send(
            process.stdin,
            {
                "command": "load_model",
                "request_id": "load",
                "model_path": str(model),
                "adapter_type": "sensevoice",
                "device": "cpu",
            },
        )
        ready = wait_for(process.stdout, "model_ready", "load")
        load_ms = round((time.perf_counter() - load_started) * 1000)
        send(
            process.stdin,
            {
                "command": "transcribe",
                "request_id": "transcribe",
                "sample_rate": 16_000,
            },
            audio,
        )
        result = wait_for(process.stdout, "transcription_completed", "transcribe")
        send(process.stdin, {"command": "shutdown", "request_id": "shutdown"})
        wait_for(process.stdout, "shutdown_complete", "shutdown")
        return {
            "text": result.get("text", ""),
            "load_ms": load_ms,
            "inference_ms": result.get("inference_ms"),
            "device": ready.get("device", ""),
        }
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True, type=Path)
    parser.add_argument("--audio", required=True, type=Path)
    parser.add_argument("--native-worker", required=True, type=Path)
    parser.add_argument("--python", default=sys.executable, type=Path)
    args = parser.parse_args()
    model = args.model.resolve()
    audio_path = args.audio.resolve()
    native_worker = args.native_worker.resolve()
    python = args.python.resolve()
    audio = read_audio(audio_path)
    native = run_worker([str(native_worker)], model, audio)
    python_worker = run_worker(
        [str(python), "-m", "worker.rain_worker"], model, audio
    )
    print(
        json.dumps(
            {
                "audio": str(audio_path),
                "duration_ms": len(audio) // 32,
                "texts_match": native["text"] == python_worker["text"],
                "native": native,
                "python": python_worker,
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
