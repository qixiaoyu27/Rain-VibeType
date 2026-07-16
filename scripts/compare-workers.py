#!/usr/bin/env python3
"""Compare native and Python workers with identical PCM or an AISHELL corpus."""

from __future__ import annotations

import argparse
import json
import os
import random
import subprocess
import sys
import tarfile
import time
import unicodedata
import wave
from pathlib import Path
from typing import BinaryIO

import numpy as np
import requests

ROOT = Path(__file__).resolve().parents[1]
AISHELL_REVISION = "bbe295d530192a4cd41644b711c9aecd087df653"
AISHELL_REPOSITORY = "https://huggingface.co/datasets/AISHELL/AISHELL-1"
AISHELL_DOWNLOAD_BASE = (
    f"https://hf-mirror.com/datasets/AISHELL/AISHELL-1/resolve/{AISHELL_REVISION}"
)
AISHELL_SPEAKERS = tuple(f"S{i:04d}" for i in range(2, 8))


def read_audio(path: Path) -> bytes:
    try:
        import soundfile as sf

        samples, sample_rate = sf.read(path, dtype="float32", always_2d=True)
        mono = samples.mean(axis=1)
    except (ImportError, RuntimeError):
        if path.suffix.lower() != ".wav":
            raise RuntimeError("Non-WAV audio requires soundfile") from None
        with wave.open(str(path), "rb") as source:
            if source.getsampwidth() != 2:
                raise RuntimeError("Only 16-bit PCM WAV is supported without soundfile")
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


class WorkerSession:
    def __init__(self, command: list[str], model: Path, log_path: Path):
        log_path.parent.mkdir(parents=True, exist_ok=True)
        self.log = log_path.open("w", encoding="utf-8")
        self.process = subprocess.Popen(
            command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self.log,
            cwd=ROOT,
        )
        assert self.process.stdin and self.process.stdout
        try:
            wait_for(self.process.stdout, "worker_ready")
            started = time.perf_counter()
            send(
                self.process.stdin,
                {
                    "command": "load_model",
                    "request_id": "load",
                    "model_path": str(model),
                    "adapter_type": "sensevoice",
                    "device": "cpu",
                },
            )
            ready = wait_for(self.process.stdout, "model_ready", "load")
            self.load_ms = round((time.perf_counter() - started) * 1000)
            self.device = str(ready.get("device", ""))
        except BaseException:
            self.close(force=True)
            raise

    def transcribe(self, audio: bytes, request_id: str) -> dict[str, object]:
        assert self.process.stdin and self.process.stdout
        send(
            self.process.stdin,
            {
                "command": "transcribe",
                "request_id": request_id,
                "sample_rate": 16_000,
            },
            audio,
        )
        result = wait_for(self.process.stdout, "transcription_completed", request_id)
        return {
            "text": str(result.get("text", "")),
            "inference_ms": result.get("inference_ms"),
        }

    def close(self, force: bool = False) -> None:
        if self.process.poll() is None and not force:
            try:
                assert self.process.stdin and self.process.stdout
                send(self.process.stdin, {"command": "shutdown", "request_id": "shutdown"})
                wait_for(self.process.stdout, "shutdown_complete", "shutdown")
            except (BrokenPipeError, RuntimeError):
                force = True
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait()
        self.log.close()

    def __enter__(self) -> "WorkerSession":
        return self

    def __exit__(self, *_args: object) -> None:
        self.close()


def run_worker(command: list[str], model: Path, audio: bytes, log_path: Path) -> dict[str, object]:
    with WorkerSession(command, model, log_path) as worker:
        result = worker.transcribe(audio, "transcribe")
        return {
            **result,
            "load_ms": worker.load_ms,
            "device": worker.device,
        }


def download(url: str, path: Path) -> None:
    if path.is_file() and path.stat().st_size:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".part")
    print(f"Downloading {path.name}...", file=sys.stderr, flush=True)
    try:
        with requests.get(
            url,
            headers={"User-Agent": "RainVibetype-benchmark/1.0"},
            stream=True,
            timeout=(30, 60),
        ) as response, temporary.open("wb") as target:
            response.raise_for_status()
            for chunk in response.iter_content(1024 * 1024):
                target.write(chunk)
        temporary.replace(path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def extract_archive(archive_path: Path, output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    root = output.resolve()
    with tarfile.open(archive_path, "r:gz") as archive:
        members = archive.getmembers()
        for member in members:
            target = (output / member.name).resolve()
            if not target.is_relative_to(root) or member.issym() or member.islnk():
                raise RuntimeError(f"Unsafe AISHELL archive member: {member.name}")
        archive.extractall(output, members=members)


def parse_transcripts(path: Path) -> dict[str, str]:
    transcripts: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        utterance_id, separator, text = line.partition(" ")
        if separator and text.strip():
            transcripts[utterance_id] = text.strip()
    return transcripts


def prepare_aishell(cache: Path, sample_count: int) -> list[dict[str, str]]:
    if not 1 <= sample_count <= 300:
        raise ValueError("AISHELL sample count must be between 1 and 300")
    transcript_path = cache / "aishell_transcript_v0.8.txt"
    download(
        f"{AISHELL_DOWNLOAD_BASE}/data_aishell/transcript/aishell_transcript_v0.8.txt?download=true",
        transcript_path,
    )
    transcripts = parse_transcripts(transcript_path)
    audio_root = cache / "audio"
    available: dict[str, list[Path]] = {}
    for speaker in AISHELL_SPEAKERS:
        speaker_files = sorted(audio_root.rglob(f"*{speaker}*.wav"))
        if not speaker_files:
            archive_path = cache / f"{speaker}.tar.gz"
            download(
                f"{AISHELL_DOWNLOAD_BASE}/data_aishell/wav/{speaker}.tar.gz?download=true",
                archive_path,
            )
            extract_archive(archive_path, audio_root)
            speaker_files = sorted(audio_root.rglob(f"*{speaker}*.wav"))
        available[speaker] = [path for path in speaker_files if path.stem in transcripts]
        if not available[speaker]:
            raise RuntimeError(f"No transcript-backed WAV files found for {speaker}")

    randomizer = random.Random(20260715)
    samples: list[dict[str, str]] = []
    base, remainder = divmod(sample_count, len(AISHELL_SPEAKERS))
    for index, speaker in enumerate(AISHELL_SPEAKERS):
        count = base + (1 if index < remainder else 0)
        if count > len(available[speaker]):
            raise RuntimeError(f"Not enough WAV files for {speaker}: {len(available[speaker])}")
        for path in sorted(randomizer.sample(available[speaker], count)):
            samples.append(
                {
                    "id": path.stem,
                    "speaker": speaker,
                    "audio": str(path.resolve()),
                    "reference": transcripts[path.stem],
                }
            )
    samples.sort(key=lambda sample: sample["id"])
    (cache / f"manifest-{sample_count}.json").write_text(
        json.dumps(samples, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return samples


def normalize_for_cer(text: str) -> str:
    normalized = unicodedata.normalize("NFKC", text).lower()
    return "".join(
        character
        for character in normalized
        if not character.isspace()
        and not unicodedata.category(character).startswith(("P", "S"))
    )


def edit_distance(reference: str, hypothesis: str) -> int:
    previous = list(range(len(hypothesis) + 1))
    for reference_index, reference_character in enumerate(reference, 1):
        current = [reference_index]
        for hypothesis_index, hypothesis_character in enumerate(hypothesis, 1):
            current.append(
                min(
                    current[-1] + 1,
                    previous[hypothesis_index] + 1,
                    previous[hypothesis_index - 1]
                    + (reference_character != hypothesis_character),
                )
            )
        previous = current
    return previous[-1]


def run_corpus(
    command: list[str],
    model: Path,
    samples: list[dict[str, str]],
    label: str,
    log_path: Path,
) -> dict[str, object]:
    rows: list[dict[str, object]] = []
    total_errors = 0
    total_characters = 0
    inference_times: list[float] = []
    exact_matches = 0
    with WorkerSession(command, model, log_path) as worker:
        for index, sample in enumerate(samples, 1):
            audio = read_audio(Path(sample["audio"]))
            result = worker.transcribe(audio, sample["id"])
            reference = normalize_for_cer(sample["reference"])
            hypothesis = normalize_for_cer(str(result["text"]))
            errors = edit_distance(reference, hypothesis)
            total_errors += errors
            total_characters += len(reference)
            exact_matches += errors == 0
            if isinstance(result["inference_ms"], (int, float)):
                inference_times.append(float(result["inference_ms"]))
            rows.append(
                {
                    **sample,
                    "duration_ms": len(audio) // 32,
                    "hypothesis": result["text"],
                    "normalized_reference": reference,
                    "normalized_hypothesis": hypothesis,
                    "errors": errors,
                    "inference_ms": result["inference_ms"],
                }
            )
            if index % 20 == 0 or index == len(samples):
                print(f"{label}: {index}/{len(samples)}", file=sys.stderr, flush=True)
        return {
            "device": worker.device,
            "load_ms": worker.load_ms,
            "reference_characters": total_characters,
            "errors": total_errors,
            "cer": total_errors / total_characters if total_characters else 0.0,
            "exact_matches": exact_matches,
            "mean_inference_ms": (
                sum(inference_times) / len(inference_times) if inference_times else None
            ),
            "rows": rows,
        }


def corpus_report(
    native: dict[str, object],
    python: dict[str, object],
    sample_count: int,
    threshold: float,
) -> dict[str, object]:
    native_rows = {row["id"]: row for row in native["rows"]}  # type: ignore[index]
    python_rows = {row["id"]: row for row in python["rows"]}  # type: ignore[index]
    disagreements = sum(
        native_rows[key]["normalized_hypothesis"]
        != python_rows[key]["normalized_hypothesis"]
        for key in native_rows
    )
    native_cer = float(native["cer"])
    python_cer = float(python["cer"])
    return {
        "dataset": {
            "name": "AISHELL-1",
            "repository": AISHELL_REPOSITORY,
            "download_mirror": "https://hf-mirror.com",
            "revision": AISHELL_REVISION,
            "license": "Apache-2.0",
            "samples": sample_count,
            "speakers": list(AISHELL_SPEAKERS),
            "selection_seed": 20260715,
        },
        "gate": {
            "maximum_native_cer_gap": threshold,
            "native_cer_gap": native_cer - python_cer,
            "passed": native_cer - python_cer <= threshold,
        },
        "worker_disagreements": disagreements,
        "native": native,
        "python": python,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True, type=Path)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--audio", type=Path)
    source.add_argument("--aishell-samples", type=int)
    parser.add_argument("--native-worker", required=True, type=Path)
    parser.add_argument("--python", default=sys.executable, type=Path)
    parser.add_argument(
        "--cache",
        type=Path,
        default=Path(os.environ.get("LOCALAPPDATA", ROOT))
        / "Rain-Vibetype"
        / "benchmarks"
        / "aishell1",
    )
    parser.add_argument("--output", type=Path)
    parser.add_argument("--max-cer-gap", type=float, default=0.005)
    args = parser.parse_args()

    assert edit_distance("开饭时间", "开放时间") == 1
    assert normalize_for_cer("你 好，世界！") == "你好世界"

    model = args.model.resolve()
    native_worker = args.native_worker.resolve()
    python = args.python.resolve()
    cache = args.cache.resolve()
    cache.mkdir(parents=True, exist_ok=True)

    if args.audio:
        audio_path = args.audio.resolve()
        audio = read_audio(audio_path)
        native = run_worker(
            [str(native_worker)], model, audio, cache / "native-single.log"
        )
        python_worker = run_worker(
            [str(python), "-m", "worker.rain_worker"],
            model,
            audio,
            cache / "python-single.log",
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

    samples = prepare_aishell(cache, args.aishell_samples)
    native = run_corpus(
        [str(native_worker)], model, samples, "native", cache / "native-corpus.log"
    )
    python_worker = run_corpus(
        [str(python), "-m", "worker.rain_worker"],
        model,
        samples,
        "python",
        cache / "python-corpus.log",
    )
    report = corpus_report(
        native, python_worker, args.aishell_samples, args.max_cer_gap
    )
    output = (args.output or cache / f"result-{args.aishell_samples}.json").resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    compact = {
        "output": str(output),
        "gate": report["gate"],
        "worker_disagreements": report["worker_disagreements"],
        "native": {key: value for key, value in native.items() if key != "rows"},
        "python": {key: value for key, value in python_worker.items() if key != "rows"},
    }
    print(json.dumps(compact, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
