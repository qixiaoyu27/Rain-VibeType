#!/usr/bin/env python3
import argparse
from pathlib import Path

import onnx
import sentencepiece as spm
import torch
from funasr import AutoModel


def sensevoice_forward(self, x, x_length, language, text_norm):
    language_query = self.embed(language).unsqueeze(1)
    text_norm_query = self.embed(text_norm).unsqueeze(1)
    event_emotion = torch.tensor([1, 2], dtype=torch.long, device=x.device)
    event_emotion_query = self.embed(event_emotion).unsqueeze(0).expand(x.size(0), -1, -1)
    x = torch.cat((language_query, event_emotion_query, text_norm_query, x), dim=1)
    encoder_out, encoder_out_lens = self.encoder(x, x_length + 4)
    if isinstance(encoder_out, tuple):
        encoder_out = encoder_out[0]
    return self.ctc.ctc_lo(encoder_out), encoder_out_lens


def read_cmvn(path: Path) -> tuple[str, str]:
    values = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("<LearnRateCoef>") and "[" in line and "]" in line:
            values.append(",".join(line.split("[", 1)[1].rsplit("]", 1)[0].split()))
    if len(values) != 2:
        raise ValueError(f"无法从 {path} 读取 CMVN")
    return values[0], values[1]


def write_tokens(model_file: Path, output: Path) -> int:
    processor = spm.SentencePieceProcessor(model_file=str(model_file))
    with output.open("w", encoding="utf-8", newline="\n") as file:
        for token_id in range(processor.vocab_size()):
            file.write(f"{processor.id_to_piece(token_id)} {token_id}\n")
    return processor.vocab_size()


def add_metadata(path: Path, metadata: dict[str, object]) -> None:
    model = onnx.load(str(path))
    del model.metadata_props[:]
    for key, value in metadata.items():
        item = model.metadata_props.add()
        item.key = key
        item.value = str(value)
    onnx.save(model, str(path))


def export(source: Path, output: Path, force: bool) -> None:
    required = [
        source / "model.pt",
        source / "config.yaml",
        source / "am.mvn",
        source / "chn_jpn_yue_eng_ko_spectok.bpe.model",
    ]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        raise FileNotFoundError("模型缺少文件：" + ", ".join(missing))
    output.mkdir(parents=True, exist_ok=True)
    onnx_path = output / "model.onnx"
    tokens_path = output / "tokens.txt"
    if not force and (onnx_path.exists() or tokens_path.exists()):
        raise FileExistsError("输出文件已存在；确认后使用 --force 覆盖")

    wrapper = AutoModel(model=str(source), device="cpu", disable_update=True)
    model = wrapper.model.eval()
    model.__class__.forward = sensevoice_forward
    x = torch.randn(1, 100, 560, dtype=torch.float32)
    x_length = torch.tensor([100], dtype=torch.int32)
    language = torch.tensor([0], dtype=torch.int32)
    text_norm = torch.tensor([14], dtype=torch.int32)
    torch.onnx.export(
        model,
        (x, x_length, language, text_norm),
        str(onnx_path),
        opset_version=13,
        input_names=["x", "x_length", "language", "text_norm"],
        output_names=["logits", "encoder_out_lens"],
        dynamic_axes={
            "x": {0: "N", 1: "T"},
            "x_length": {0: "N"},
            "language": {0: "N"},
            "text_norm": {0: "N"},
            "logits": {0: "N", 1: "T"},
            "encoder_out_lens": {0: "N"},
        },
        dynamo=False,
    )

    frontend = wrapper.kwargs["frontend_conf"]
    neg_mean, inv_stddev = read_cmvn(source / "am.mvn")
    vocab_size = write_tokens(required[3], tokens_path)
    add_metadata(
        onnx_path,
        {
            "lfr_window_size": frontend["lfr_m"],
            "lfr_window_shift": frontend["lfr_n"],
            "normalize_samples": 0,
            "neg_mean": neg_mean,
            "inv_stddev": inv_stddev,
            "model_type": "sense_voice_ctc",
            "version": "2",
            "model_author": "iic",
            "maintainer": "Rain-Vibetype",
            "vocab_size": vocab_size,
            "comment": "iic/SenseVoiceSmall exported without quantization",
            "lang_auto": model.lid_dict["auto"],
            "lang_zh": model.lid_dict["zh"],
            "lang_en": model.lid_dict["en"],
            "lang_yue": model.lid_dict["yue"],
            "lang_ja": model.lid_dict["ja"],
            "lang_ko": model.lid_dict["ko"],
            "lang_nospeech": model.lid_dict["nospeech"],
            "with_itn": model.textnorm_dict["withitn"],
            "without_itn": model.textnorm_dict["woitn"],
            "url": "https://modelscope.cn/models/iic/SenseVoiceSmall",
        },
    )
    print(f"ONNX: {onnx_path} ({onnx_path.stat().st_size} bytes)")
    print(f"Tokens: {tokens_path} ({tokens_path.stat().st_size} bytes)")


def main() -> None:
    parser = argparse.ArgumentParser(description="导出 Rain 原生 Worker 使用的 SenseVoice ONNX 模型")
    parser.add_argument("model_path", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()
    export(args.model_path.resolve(), (args.output or args.model_path).resolve(), args.force)


if __name__ == "__main__":
    torch.manual_seed(20240717)
    main()
