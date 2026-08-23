#!/usr/bin/env python3
"""Check ordinary repository-local Markdown links and heading fragments."""

from __future__ import annotations

import argparse
import re
import sys
import unicodedata
from collections import Counter
from pathlib import Path
from urllib.parse import unquote


INLINE_LINK = re.compile(r"(?<!!)\[[^]\n]*\]\(([^)\n]+)\)")
HEADING = re.compile(r"^ {0,3}#{1,6}[ \t]+(.+?)[ \t]*#*[ \t]*$")
EXTERNAL_TARGETS = ("https://", "http://", "mailto:", "tel:", "data:")


def markdown_files(paths: list[Path]) -> list[Path]:
    files: set[Path] = set()
    for path in paths:
        if path.is_dir():
            files.update(candidate for candidate in path.rglob("*.md") if candidate.is_file())
        elif path.is_file() and path.suffix.lower() == ".md":
            files.add(path)
    return sorted(files)


def heading_slug(heading: str) -> str:
    text = re.sub(r"`([^`]*)`", r"\1", heading)
    text = re.sub(r"\[([^]]+)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"[*_~]", "", text)
    text = unicodedata.normalize("NFKD", text).lower().strip()
    text = re.sub(r"[^\w\- ]", "", text)
    return re.sub(r"\s+", "-", text).strip("-")


def anchors(path: Path) -> set[str]:
    occurrences: Counter[str] = Counter()
    available: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        match = HEADING.match(line)
        if match is None:
            continue
        base = heading_slug(match.group(1))
        suffix = occurrences[base]
        occurrences[base] += 1
        available.add(base if suffix == 0 else f"{base}-{suffix}")
    return available


def target_parts(raw_target: str) -> tuple[str, str]:
    target = raw_target.strip().split(maxsplit=1)[0].strip("<>")
    path_part, separator, fragment = target.partition("#")
    return unquote(path_part), unquote(fragment) if separator else ""


def check_link(source: Path, raw_target: str, cache: dict[Path, set[str]]) -> str | None:
    path_part, fragment = target_parts(raw_target)
    if not path_part and not fragment:
        return None
    if path_part.startswith(EXTERNAL_TARGETS):
        return None
    destination = source if not path_part else (source.parent / path_part).resolve()
    if destination.is_dir() and not fragment:
        return None
    if not destination.is_file():
        return f"missing target: {raw_target}"
    if fragment:
        if destination.suffix.lower() != ".md":
            return f"fragment on non-Markdown target: {raw_target}"
        destination_anchors = cache.setdefault(destination, anchors(destination))
        if fragment not in destination_anchors:
            return f"missing heading fragment: {raw_target}"
    return None


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Check repository-local inline Markdown links and heading fragments."
    )
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        default=[Path("README.md"), Path("docs")],
        help="Markdown files or directories to inspect (default: README.md docs)",
    )
    args = parser.parse_args()
    files = markdown_files(args.paths)
    errors: list[str] = []
    anchor_cache: dict[Path, set[str]] = {}

    for source in files:
        for line_number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), start=1):
            for match in INLINE_LINK.finditer(line):
                error = check_link(source, match.group(1), anchor_cache)
                if error:
                    errors.append(f"{source}:{line_number}: {error}")

    if errors:
        print("Local Markdown link check failed:", file=sys.stderr)
        print("\n".join(errors), file=sys.stderr)
        return 1

    print(f"Checked local Markdown links in {len(files)} file(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
