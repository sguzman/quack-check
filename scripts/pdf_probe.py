#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path

PdfReader = None
try:
    from pypdf import PdfReader
except Exception:
    PdfReader = None

try:
    import pypdfium2 as pdfium
except Exception:
    pdfium = None

GARBAGE_RE = re.compile(r"[\uFFFD]")


def main() -> None:
    req = json.loads(sys.stdin.read().strip() or "{}")
    input_pdf = Path(req["input_pdf"])
    sample_pages = int(req.get("sample_pages", 12))

    reader = None
    doc = None
    if PdfReader is not None:
        try:
            reader = PdfReader(str(input_pdf))
        except Exception as e:
            print(
                json.dumps(
                    {
                        "page_count": 0,
                        "sampled_pages": 0,
                        "avg_chars_per_page": 0,
                        "garbage_ratio": 1.0,
                        "whitespace_ratio": 1.0,
                        "error": f"failed to read pdf: {e}",
                    }
                )
            )
            return
        n_pages = len(reader.pages)
    elif pdfium is not None:
        try:
            doc = pdfium.PdfDocument(str(input_pdf))
        except Exception as e:
            print(
                json.dumps(
                    {
                        "page_count": 0,
                        "sampled_pages": 0,
                        "avg_chars_per_page": 0,
                        "garbage_ratio": 1.0,
                        "whitespace_ratio": 1.0,
                        "error": f"failed to read pdf via pypdfium2: {e}",
                    }
                )
            )
            return
        n_pages = len(doc)
    else:
        print(
            json.dumps(
                {
                    "page_count": 0,
                    "sampled_pages": 0,
                    "avg_chars_per_page": 0,
                    "garbage_ratio": 1.0,
                    "whitespace_ratio": 1.0,
                    "error": "missing pypdf and pypdfium2 imports",
                }
            )
        )
        return
    if n_pages == 0:
        out = dict(
            page_count=0,
            sampled_pages=0,
            avg_chars_per_page=0,
            garbage_ratio=1.0,
            whitespace_ratio=1.0,
        )
        print(json.dumps(out))
        return

    k = min(sample_pages, n_pages)
    idxs = []
    if k == 1:
        idxs = [0]
    else:
        for i in range(k):
            idxs.append(round(i * (n_pages - 1) / (k - 1)))

    total_chars = 0
    total_ws = 0
    total_garbage = 0

    for i in idxs:
        if reader is not None:
            txt = reader.pages[i].extract_text() or ""
        else:
            page = doc[i]
            text_page = page.get_textpage()
            txt = text_page.get_text_range() or ""
            text_page.close()
            page.close()
        total_chars += len(txt)
        total_ws += sum(1 for c in txt if c.isspace())
        total_garbage += len(GARBAGE_RE.findall(txt))

    avg = int(total_chars / max(1, len(idxs)))
    garbage_ratio = float(total_garbage / max(1, total_chars))
    whitespace_ratio = float(total_ws / max(1, total_chars))

    out = dict(
        page_count=n_pages,
        sampled_pages=len(idxs),
        avg_chars_per_page=avg,
        garbage_ratio=garbage_ratio,
        whitespace_ratio=whitespace_ratio,
    )
    print(json.dumps(out))
    if doc is not None:
        doc.close()


if __name__ == "__main__":
    main()
