#!/usr/bin/env python3
import json
import sys
from pathlib import Path

try:
    from pypdf import PdfReader, PdfWriter
except Exception as e:
    print(json.dumps({"ok": False, "error": f"missing pypdf import: {e}"}))
    sys.exit(0)


def main() -> None:
    req = json.loads(sys.stdin.read().strip() or "{}")
    input_pdf = Path(req["input_pdf"])
    out_dir = Path(req["out_dir"])
    chunks = req.get("chunks", [])

    try:
        reader = PdfReader(str(input_pdf))
    except Exception as e:
        print(json.dumps({"ok": False, "error": f"failed to read pdf: {e}"}))
        return

    n_pages = len(reader.pages)
    out_dir.mkdir(parents=True, exist_ok=True)

    outputs = []
    for i, ch in enumerate(chunks):
        s = int(ch["start_page"])
        e = int(ch["end_page"])
        if s < 1 or e < s or e > n_pages:
            print(
                json.dumps(
                    {
                        "ok": False,
                        "error": f"invalid chunk range: {s}-{e} (pages={n_pages})",
                    }
                )
            )
            return

        w = PdfWriter()
        for p in range(s - 1, e):
            w.add_page(reader.pages[p])

        out_path = out_dir / f"chunk_{i:05d}_p{s:05d}-p{e:05d}.pdf"
        with out_path.open("wb") as f:
            w.write(f)

        outputs.append(
            {
                "chunk_index": i,
                "start_page": s,
                "end_page": e,
                "path": str(out_path),
            }
        )

    print(json.dumps({"ok": True, "outputs": outputs}))


if __name__ == "__main__":
    main()
