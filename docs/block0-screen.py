#!/usr/bin/env python3
"""Write the local projector page for the block-0 ceremony.

The page receives a public EntityID, a QR code, and public status lines.
Seed words and extended private keys are refused.
"""

import html
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
TEMPLATE = ROOT / "block0-display.html"
OUT = Path("/tmp/block0-screen")
ID_RE = re.compile(r"^[0-9a-fA-F]{64}$")


def die(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def require_id(value: str) -> str:
    if any(char.isspace() for char in value):
        die("refusing input that contains spaces")
    if "xprv" in value.lower():
        die("refusing an extended private key")
    if ID_RE.fullmatch(value) is None:
        die("EntityID must be 64 hex characters")
    return value.lower()


def require_label(value: str) -> str:
    if "xprv" in value.lower():
        die("refusing an extended private key")
    if "\n" in value or "\r" in value:
        die("refusing a label that spans lines")
    return value


def read_state() -> dict[str, str]:
    path = OUT / "state.txt"
    if not path.exists():
        die("run init first")
    state: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, value = line.split("=", 1)
        state[key] = value
    return state


def write_page(state: dict[str, str]) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    page = TEMPLATE.read_text(encoding="utf-8")
    replacements = {
        "NAME_HERE": html.escape(state["name"]),
        "ENTITY_ID_HERE": html.escape(state["entity_id"]),
        "VERIFIER_A_HERE": html.escape(state["verifier_a"]),
        "VERIFIER_B_HERE": html.escape(state["verifier_b"]),
        "CLAIM_HERE": html.escape(state["claim"]),
    }
    for token, value in replacements.items():
        page = page.replace(token, value)
    (OUT / "index.html").write_text(page, encoding="utf-8")
    lines = [f"{key}={value}" for key, value in state.items()]
    (OUT / "state.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (OUT / "entity-id.txt").write_text(state["entity_id"] + "\n", encoding="utf-8")


def main() -> None:
    if len(sys.argv) < 2:
        die(
            "usage: block0-screen.py init ENTITYID NAME"
            " | verifier A|B STATE HEIGHT | claim NAME"
        )
    command = sys.argv[1]
    if command == "init":
        if len(sys.argv) != 4:
            die("usage: block0-screen.py init ENTITYID NAME")
        if shutil.which("qrencode") is None:
            die("qrencode is not installed")
        entity_id = require_id(sys.argv[2])
        name = require_label(sys.argv[3])
        state = {
            "name": name,
            "entity_id": entity_id,
            "verifier_a": "waiting",
            "verifier_b": "waiting",
            "claim": "Name claim: waiting",
        }
        write_page(state)
        subprocess.run(
            [
                "qrencode",
                "-o",
                str(OUT / "entity-qr.png"),
                "-t",
                "PNG",
                "-l",
                "M",
                "-s",
                "12",
                "--",
                entity_id,
            ],
            check=True,
        )
    elif command == "verifier":
        if len(sys.argv) != 5:
            die("usage: block0-screen.py verifier A|B STATE HEIGHT")
        which = sys.argv[2].upper()
        if which not in {"A", "B"}:
            die("verifier must be A or B")
        state_word = require_label(sys.argv[3])
        height = sys.argv[4]
        if not height.isdigit():
            die("height must be a number")
        state = read_state()
        state[f"verifier_{which.lower()}"] = f"{state_word} at height {height}"
        write_page(state)
    elif command == "claim":
        if len(sys.argv) != 3:
            die("usage: block0-screen.py claim NAME")
        state = read_state()
        state["claim"] = "Name claim signed: " + require_label(sys.argv[2])
        write_page(state)
    else:
        die("unknown command")
    print(f"screen={OUT}")


if __name__ == "__main__":
    main()
