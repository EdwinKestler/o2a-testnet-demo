#!/usr/bin/env python3
"""Fail when pending-decision chips drift from the demonstration script."""

from __future__ import annotations

from html.parser import HTMLParser
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "docs" / "guion-demo.md"
PAGES = (ROOT / "site" / "index.html", ROOT / "site" / "en" / "index.html")
BRACKET = re.compile(r"\[([^\]\n]*)\]")


class PendingChipParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.sources: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = dict(attrs)
        classes = (values.get("class") or "").split()
        if "pending-chip" in classes:
            source = values.get("data-source")
            if source is None:
                raise ValueError("pending-chip is missing data-source")
            self.sources.append(source)


def source_brackets() -> list[str]:
    text = SOURCE.read_text(encoding="utf-8")
    return [match.strip() for match in BRACKET.findall(text) if match.strip()]


def page_chips(path: Path) -> list[str]:
    parser = PendingChipParser()
    parser.feed(path.read_text(encoding="utf-8"))
    return parser.sources


def main() -> int:
    expected = source_brackets()
    if not expected:
        print("error: source guion has no non-empty bracketed decisions", file=sys.stderr)
        return 1

    failed = False
    print(f"source brackets: {len(expected)}")
    for page in PAGES:
        actual = page_chips(page)
        print(f"{page.relative_to(ROOT)} pending chips: {len(actual)}")
        if actual != expected:
            failed = True
            print(f"error: {page.relative_to(ROOT)} does not match source order/content", file=sys.stderr)

    if failed:
        return 1
    print("bracket check: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
