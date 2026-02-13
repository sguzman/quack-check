#!/usr/bin/env python3
import importlib
import inspect
import json
import os
import sys
from pathlib import Path


def doctor():
    out = {
        "python_exe": sys.executable,
        "python_version": sys.version.split()[0],
        "docling_version": None,
        "ok": False,
        "error": None,
    }
    try:
        import docling  # noqa

        out["docling_version"] = getattr(docling, "__version__", None)
        out["ok"] = True
    except Exception as e:
        out["ok"] = False
        out["error"] = str(e)
    print(json.dumps(out))


def set_if_present(obj, name, value, applied, ignored):
    if value is None:
        return
    if hasattr(obj, name):
        try:
            setattr(obj, name, value)
            applied.append(name)
        except Exception as e:
            ignored.append(f"{name} ({e})")
    else:
        ignored.append(name)


def resolve_backend_class(name: str):
    if not name or name == "AUTO":
        return None

    candidates = {
        "PYPDFIUM2": [
            ("docling.backend.pypdfium2_backend", "PyPdfiumDocumentBackend"),
            ("docling.backends.pypdfium2_backend", "PyPdfiumDocumentBackend"),
        ],
        "DLPARSE_V1": [
            ("docling.backend.docling_parse_backend", "DoclingParseDocumentBackend"),
            ("docling.backends.docling_parse_backend", "DoclingParseDocumentBackend"),
        ],
        "DLPARSE_V2": [
            ("docling.backend.docling_parse_v2_backend", "DoclingParseV2DocumentBackend"),
            ("docling.backends.docling_parse_v2_backend", "DoclingParseV2DocumentBackend"),
        ],
        "DLPARSE_V4": [
            ("docling.backend.docling_parse_v4_backend", "DoclingParseV4DocumentBackend"),
            ("docling.backends.docling_parse_v4_backend", "DoclingParseV4DocumentBackend"),
        ],
    }

    for module, attr in candidates.get(name, []):
        try:
            mod = importlib.import_module(module)
            return getattr(mod, attr)
        except Exception:
            continue

    return None


def build_pipeline_options(cfg: dict, do_ocr: bool):
    applied = []
    ignored = []

    pipeline_cfg = cfg["docling"]["pipeline"]

    try:
        from docling.datamodel.pipeline_options import ThreadedPdfPipelineOptions
    except Exception:
        ThreadedPdfPipelineOptions = None

    try:
        from docling.datamodel.pipeline_options import PdfPipelineOptions
    except Exception:
        PdfPipelineOptions = None

    if pipeline_cfg.get("use_threaded_pipeline", False) and ThreadedPdfPipelineOptions:
        pipeline_options = ThreadedPdfPipelineOptions()
    elif PdfPipelineOptions:
        pipeline_options = PdfPipelineOptions()
    else:
        raise RuntimeError("docling PdfPipelineOptions unavailable")

    set_if_present(pipeline_options, "do_ocr", do_ocr, applied, ignored)
    set_if_present(
        pipeline_options,
        "force_backend_text",
        pipeline_cfg.get("force_backend_text", False),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "do_table_structure",
        pipeline_cfg.get("do_table_structure", True),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "do_code_enrichment",
        pipeline_cfg.get("do_code_enrichment", False),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "do_formula_enrichment",
        pipeline_cfg.get("do_formula_enrichment", False),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "do_picture_description",
        pipeline_cfg.get("do_picture_description", False),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "do_picture_classification",
        pipeline_cfg.get("do_picture_classification", False),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "generate_page_images",
        pipeline_cfg.get("generate_page_images", False),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "generate_picture_images",
        pipeline_cfg.get("generate_picture_images", False),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "generate_table_images",
        pipeline_cfg.get("generate_table_images", False),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "generate_parsed_pages",
        pipeline_cfg.get("generate_parsed_pages", False),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "create_legacy_output",
        pipeline_cfg.get("create_legacy_output", False),
        applied,
        ignored,
    )
    offline_only = cfg.get("global", {}).get("offline_only", False)
    enable_remote = pipeline_cfg.get("enable_remote_services", False)
    allow_plugins = pipeline_cfg.get("allow_external_plugins", False)
    if offline_only:
        enable_remote = False
        allow_plugins = False

    set_if_present(pipeline_options, "enable_remote_services", enable_remote, applied, ignored)
    set_if_present(pipeline_options, "allow_external_plugins", allow_plugins, applied, ignored)
    set_if_present(
        pipeline_options,
        "document_timeout",
        int(pipeline_cfg.get("document_timeout_seconds", 0)),
        applied,
        ignored,
    )

    images_scale = pipeline_cfg.get("images_scale", None)
    if images_scale is not None:
        set_if_present(pipeline_options, "images_scale", float(images_scale), applied, ignored)

    # Threaded pipeline tuning (best-effort)
    set_if_present(
        pipeline_options,
        "queue_max_size",
        pipeline_cfg.get("queue_max_size", None),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "num_threads",
        pipeline_cfg.get("num_threads", None),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "layout_batch_size",
        pipeline_cfg.get("layout_batch_size", None),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "table_batch_size",
        pipeline_cfg.get("table_batch_size", None),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "picture_batch_size",
        pipeline_cfg.get("picture_batch_size", None),
        applied,
        ignored,
    )
    set_if_present(
        pipeline_options,
        "page_batch_size",
        pipeline_cfg.get("page_batch_size", None),
        applied,
        ignored,
    )

    # OCR options
    if do_ocr:
        ocr_cfg = cfg["docling"]["ocr"]
        try:
            from docling.datamodel.pipeline_options import (
                EasyOcrOptions,
                TesseractCliOcrOptions,
                TesseractOcrOptions,
            )
        except Exception:
            EasyOcrOptions = None
            TesseractCliOcrOptions = None
            TesseractOcrOptions = None

        ocr_obj = None
        engine = ocr_cfg.get("engine", "easyocr")
        if engine == "tesseract_cli" and TesseractCliOcrOptions:
            ocr_obj = TesseractCliOcrOptions()
        elif engine == "tesseract" and TesseractOcrOptions:
            ocr_obj = TesseractOcrOptions()
        elif EasyOcrOptions:
            ocr_obj = EasyOcrOptions()

        if ocr_obj is not None:
            set_if_present(ocr_obj, "lang", ocr_cfg.get("langs", []), applied, ignored)
            set_if_present(
                ocr_obj,
                "bitmap_area_threshold",
                float(ocr_cfg.get("bitmap_area_threshold", 0.25)),
                applied,
                ignored,
            )
            set_if_present(
                ocr_obj,
                "force_full_page_ocr",
                bool(ocr_cfg.get("force_full_page_ocr", False)),
                applied,
                ignored,
            )
            set_if_present(
                ocr_obj,
                "force_ocr",
                bool(ocr_cfg.get("force_ocr", False)),
                applied,
                ignored,
            )
            extra = ocr_cfg.get("tesseract_cli_args", "")
            if extra:
                set_if_present(ocr_obj, "tesseract_args", extra, applied, ignored)
            set_if_present(pipeline_options, "ocr_options", ocr_obj, applied, ignored)

    # Accelerator options (best-effort)
    acc_cfg = cfg["docling"]["accelerator"]
    try:
        from docling.datamodel.accelerator_options import AcceleratorDevice, AcceleratorOptions

        acc = AcceleratorOptions()
        device = acc_cfg.get("device", "AUTO")
        if device and device != "AUTO":
            try:
                device_enum = AcceleratorDevice[device]
            except Exception:
                device_enum = device
            set_if_present(acc, "device", device_enum, applied, ignored)
        set_if_present(
            acc,
            "inference_threads",
            int(acc_cfg.get("inference_threads", 0)),
            applied,
            ignored,
        )
        set_if_present(acc, "use_fp16", bool(acc_cfg.get("use_fp16", True)), applied, ignored)
        set_if_present(pipeline_options, "accelerator_options", acc, applied, ignored)
    except Exception:
        pass

    # artifacts path
    artifacts_path = cfg["paths"].get("docling_artifacts_dir")
    if artifacts_path:
        set_if_present(pipeline_options, "artifacts_path", artifacts_path, applied, ignored)

    return pipeline_options, applied, ignored


def convert(req, cfg):
    from docling.document_converter import DocumentConverter, PdfFormatOption
    from docling.datamodel.base_models import InputFormat

    input_pdf = req["input_pdf"]
    out_dir = Path(req["out_dir"])
    chunk_index = int(req["chunk_index"])
    start_page = int(req.get("start_page", 1))
    end_page = int(req.get("end_page", 1))
    do_ocr = bool(req.get("do_ocr", False))
    pdf_backend = req.get("pdf_backend", "AUTO")
    use_page_range = bool(req.get("use_page_range", False))

    out_dir.mkdir(parents=True, exist_ok=True)

    artifacts = cfg["paths"].get("docling_artifacts_dir", "")
    if artifacts:
        os.environ.setdefault("DOCLING_ARTIFACTS_PATH", artifacts)

    pipeline_options, applied, ignored = build_pipeline_options(cfg, do_ocr)

    backend_cls = resolve_backend_class(pdf_backend)
    if backend_cls is None:
        pdf_opt = PdfFormatOption(pipeline_options=pipeline_options)
    else:
        pdf_opt = PdfFormatOption(pipeline_options=pipeline_options, backend=backend_cls)

    converter = DocumentConverter(format_options={InputFormat.PDF: pdf_opt})

    kwargs = {
        "raises_on_error": bool(cfg["docling"].get("raises_on_error", False)),
    }
    max_num_pages = int(cfg["docling"].get("max_num_pages", 0))
    max_file_size = int(cfg["docling"].get("max_file_size_bytes", 0))
    if max_num_pages > 0:
        kwargs["max_num_pages"] = max_num_pages
    if max_file_size > 0:
        kwargs["max_file_size"] = max_file_size

    if use_page_range:
        sig = inspect.signature(converter.convert)
        if "page_range" in sig.parameters:
            kwargs["page_range"] = (start_page, end_page)
        else:
            ignored.append("page_range")

    res = converter.convert(input_pdf, **kwargs)

    warnings = []
    ok = True
    md = ""
    meta = {
        "chunk_index": chunk_index,
        "start_page": start_page,
        "end_page": end_page,
        "applied_flags": applied,
        "ignored_flags": ignored,
        "pdf_backend": pdf_backend,
        "use_page_range": use_page_range,
    }

    try:
        doc = res.document
        if hasattr(doc, "export_to_markdown"):
            md = doc.export_to_markdown()
        elif hasattr(doc, "export_to_text"):
            md = doc.export_to_text()
            warnings.append("export_to_markdown missing; used export_to_text")
        else:
            md = str(doc)
            warnings.append("no export_to_markdown/text; used str(doc)")
    except Exception as e:
        ok = False
        warnings.append(f"export failed: {e}")

    out = {"ok": ok, "markdown": md, "warnings": warnings, "meta": meta}
    print(json.dumps(out))


def main():
    payload = json.loads(sys.stdin.read().strip() or "{}")
    cmd = payload.get("cmd")

    if cmd == "doctor":
        doctor()
        return
    if cmd == "convert":
        convert(payload.get("req", {}), payload.get("cfg", {}))
        return

    print(json.dumps({"ok": False, "error": f"unknown cmd: {cmd}"}))


if __name__ == "__main__":
    main()
