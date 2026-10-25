"""Real four-speaker smoke test; models/audio stay in ignored build storage."""
import hashlib
import json
from pathlib import Path
import subprocess
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / 'macos/.build/diarization-fixtures'
FILES = [
    ('segmentation.onnx', 'https://huggingface.co/csukuangfj/sherpa-onnx-pyannote-segmentation-3-0/resolve/9403a6902bb58e3d5ae8c7e77c3422de279db2e0/model.onnx', '220ad67ca923bef2fa91f2390c786097bf305bceb5e261d4af67b38e938e1079'),
    ('embedding.onnx', 'https://huggingface.co/csukuangfj/speaker-embedding-models/resolve/0743f301363dec56491a490f6d6cbc9d67f9a3bf/3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx', '1a331345f04805badbb495c775a6ddffcdd1a732567d5ec8b3d5749e3c7a5e4b'),
    ('four-speakers.wav', 'https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/0-four-speakers-zh.wav', 'bedf036caed208386c67b4ef4b11f83d74dd0d420b102163a1c33cd09cde7010'),
]


def main():
    CACHE.mkdir(parents=True, exist_ok=True)
    for name, url, digest in FILES:
        dest = CACHE / name
        if not dest.exists() or hashlib.sha256(dest.read_bytes()).hexdigest() != digest:
            part = dest.with_suffix('.part')
            with urllib.request.urlopen(url, timeout=60) as response, part.open('wb') as output:
                while data := response.read(1024 * 1024):
                    output.write(data)
            assert hashlib.sha256(part.read_bytes()).hexdigest() == digest, name
            part.replace(dest)
    subprocess.run(['cargo', 'build', '--manifest-path', str(ROOT / 'src-tauri/Cargo.toml')], check=True)
    start = time.monotonic()
    raw = subprocess.check_output([str(ROOT / 'src-tauri/target/debug/localscribe'), '--diarize',
                                   *[str(CACHE / name) for name, _, _ in FILES]], timeout=180)
    turns = json.loads(raw)
    (CACHE / 'result.json').write_text(json.dumps(turns, indent=2))
    speakers = {turn['speaker'] for turn in turns}
    assert len(speakers) == 4, f'Expected 4 automatic speakers, got {speakers}'
    # Public reference turns: A B C A D D D C A. Check identities, not cluster numbering.
    def voice(second):
        matches = [s['speaker'] for s in turns if s['start_ms'] <= second * 1000 < s['end_ms']]
        assert len(matches) == 1, (second, matches)
        return matches[0]
    a, b, c, a2, d, d2, d3, c2, a3 = map(voice, [3, 9, 15, 23, 28, 31, 35, 49, 53])
    assert len({a, b, c, d}) == 4
    assert a == a2 == a3 and c == c2 and d == d2 == d3
    print(json.dumps({'automaticSpeakers': len(speakers), 'turns': len(turns),
                      'referenceTurnsMatched': 9, 'seconds': round(time.monotonic() - start, 2)}))


if __name__ == '__main__':
    main()
