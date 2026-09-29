#!/usr/bin/env python3
"""Write the local projector pages for the block-0 ceremony.

`init`, `verifier`, and `claim` fill the recorded rehearsal page.
`render` and `watch` fill the stage screen from verify JSON files.
The stage screen displays those files. It does not choose a history state.
`pull` is the loopback fallback that writes those files into the inbox.
The browser loads only the local page.
"""

import html
import json
import os
import re
import shutil
import subprocess
import sys
import time
import urllib.request
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parent
TEMPLATE = ROOT / "block0-display.html"
STAGE_TEMPLATE = ROOT / "stage-screen" / "stage.html"
OUT = Path("/tmp/block0-screen")
ID_RE = re.compile(r"^[0-9a-fA-F]{64}$")
DENY_MESSAGE = "refusing a field that matches the secret denylist"
NETWORKS = {"regtest", "signet", "mainnet"}
PANEL_PATHS = [
    ("verifier_id", ("verifier_id",)),
    ("network", ("network",)),
    ("entity_id", ("entity_id",)),
    ("state_id", ("state_id",)),
    ("identity_history_state", ("identity_history_state",)),
    ("seal_outpoint", ("layers", "bitcoin", "seal_outpoint")),
    ("confirmations", ("layers", "bitcoin", "confirmations")),
    ("required_depth", ("layers", "bitcoin", "required_depth")),
    ("unspent", ("layers", "bitcoin", "unspent")),
    ("source", ("layers", "bitcoin", "source")),
    ("best_block_hash", ("layers", "bitcoin", "best_block_hash")),
    ("height", ("layers", "bitcoin", "height")),
    ("rgb_status", ("layers", "rgb", "status")),
    ("genesis_valid", ("layers", "o2a", "genesis_valid")),
    ("claim_valid", ("layers", "o2a", "claim_valid")),
    ("official_name", ("layers", "o2a", "official_name")),
    ("reasons", ("reasons",)),
]

# 1x1 white PNG used when a test asks not to run qrencode. Goldens compare HTML only.
PLACEHOLDER_PNG = (
    b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01"
    b"\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\xff\xff?"
    b"\x00\x05\xfe\x02\xfe\r\xefF\xb8\x00\x00\x00\x00IEND\xaeB`\x82"
)


def die(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def secret_marked(value: str) -> bool:
    lower = value.lower()
    if any(marker in lower for marker in ("xprv", "tprv", "mnemonic", "seed.hex", "o2a_demo_seed")):
        return True
    normalized = lower.replace("\\", "/")
    if "/" not in normalized:
        return False
    for segment in normalized.split("/"):
        segment = segment.strip(" \t\"',:")
        if segment == "seed" or segment.startswith("seed."):
            return True
    return False


def require_clean(value: str) -> str:
    if secret_marked(value):
        die(DENY_MESSAGE)
    return value


def require_id(value: str) -> str:
    require_clean(value)
    if any(char.isspace() for char in value):
        die("refusing input that contains spaces")
    if ID_RE.fullmatch(value) is None:
        die("EntityID must be 64 hex characters")
    return value.lower()


def require_label(value: str) -> str:
    require_clean(value)
    if "\n" in value or "\r" in value:
        die("refusing a label that spans lines")
    return value


def read_state(out: Path) -> dict[str, str]:
    path = out / "state.txt"
    if not path.exists():
        die("run init first")
    state: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, value = line.split("=", 1)
        state[key] = value
    return state


def write_page(out: Path, state: dict[str, str]) -> None:
    for value in state.values():
        require_clean(value)
    out.mkdir(parents=True, exist_ok=True)
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
    (out / "index.html").write_text(page, encoding="utf-8")
    lines = [f"{key}={value}" for key, value in state.items()]
    (out / "state.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (out / "entity-id.txt").write_text(state["entity_id"] + "\n", encoding="utf-8")


def run_legacy(out: Path, command: str, args: list[str]) -> None:
    if command == "init":
        if len(args) != 3:
            die("usage: block0-screen.py init ENTITYID NAME")
        if shutil.which("qrencode") is None:
            die("qrencode is not installed")
        entity_id = require_id(args[1])
        name = require_label(args[2])
        state = {
            "name": name,
            "entity_id": entity_id,
            "verifier_a": "waiting",
            "verifier_b": "waiting",
            "claim": "Name claim: waiting",
        }
        write_page(out, state)
        subprocess.run(
            [
                "qrencode",
                "-o",
                str(out / "entity-qr.png"),
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
        return
    if command == "verifier":
        if len(args) != 4:
            die("usage: block0-screen.py verifier A|B STATE HEIGHT")
        which = args[1].upper()
        if which not in {"A", "B"}:
            die("verifier must be A or B")
        state_word = require_label(args[2])
        height = args[3]
        if not height.isdigit():
            die("height must be a number")
        state = read_state(out)
        state[f"verifier_{which.lower()}"] = f"{state_word} at height {height}"
        write_page(out, state)
        return
    if command == "claim":
        if len(args) != 2:
            die("usage: block0-screen.py claim NAME")
        state = read_state(out)
        state["claim"] = "Name claim signed: " + require_label(args[1])
        write_page(out, state)
        return
    die("unknown command")


def walk_strings(value: object, found: list[str]) -> None:
    if isinstance(value, str):
        found.append(value)
    elif isinstance(value, dict):
        for key, item in value.items():
            found.append(str(key))
            walk_strings(item, found)
    elif isinstance(value, list):
        for item in value:
            walk_strings(item, found)


def refuse_secrets(value: object) -> None:
    found: list[str] = []
    walk_strings(value, found)
    for item in found:
        if secret_marked(item):
            die(DENY_MESSAGE)


def lookup(document: dict, path: tuple[str, ...]) -> tuple[bool, object]:
    current: object = document
    for key in path:
        if not isinstance(current, dict) or key not in current:
            return False, None
        current = current[key]
    return True, current


def display_value(present: bool, value: object) -> str:
    if not present:
        return "missing"
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, list):
        return "\n".join(display_value(True, item) for item in value)
    if isinstance(value, (int, float)):
        return str(value)
    return str(value)


def display_verifier(document: dict | None) -> dict[str, str]:
    if document is None:
        shown = {name: "missing" for name, _path in PANEL_PATHS}
        return shown
    shown = {}
    for name, path in PANEL_PATHS:
        present, value = lookup(document, path)
        shown[name] = display_value(present, value)
    return shown


def display_preflight(document: dict | None) -> dict[str, str]:
    if document is None:
        return {
            "node_height": "missing",
            "electrs_height": "missing",
            "seal_address": "missing",
            "funding_depth": "missing",
        }
    node_present, node = lookup(document, ("node_height",))
    electrs_present, electrs = lookup(document, ("electrs_height",))
    address_present, address = lookup(document, ("seal_address",))
    conf_present, confirmations = lookup(document, ("funding_confirmations",))
    depth_present, depth = lookup(document, ("required_depth",))
    if conf_present and depth_present:
        funding = f"{display_value(True, confirmations)} / {display_value(True, depth)}"
    else:
        funding = "missing"
    return {
        "node_height": display_value(node_present, node),
        "electrs_height": display_value(electrs_present, electrs),
        "seal_address": display_value(address_present, address),
        "funding_depth": funding,
    }


def load_optional(path: Path) -> dict | None:
    if not path.exists():
        return None
    if path.is_symlink():
        die(f"{path.name} is a symlink")
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        die(f"{path.name} is not JSON")
    if not isinstance(value, dict):
        die(f"{path.name} is not a JSON object")
    refuse_secrets(value)
    return value


def panel_html(prefix: str, fields: dict[str, str]) -> str:
    rows = []
    for name, _path in PANEL_PATHS:
        rows.append(
            f"<dt>{html.escape(name)}</dt>"
            f"<dd id=\"{prefix}-{name}\">{html.escape(fields[name])}</dd>"
        )
    return "<dl>" + "".join(rows) + "</dl>"


def badge_for(network: str) -> tuple[str, str]:
    if network == "mainnet":
        return "badge badge-mainnet", "MAINNET"
    return "badge", network.upper()


def qr_payload(verifier_url: str, package_url: str) -> str:
    return f"verifier\n{verifier_url}\npackage\n{package_url}\n"


def write_qr(out: Path, payload: str, use_qr: bool) -> None:
    destination = out / "stage-qr.png"
    if not use_qr:
        destination.write_bytes(PLACEHOLDER_PNG)
        return
    if shutil.which("qrencode") is None:
        die("qrencode is not installed")
    subprocess.run(
        ["qrencode", "-o", str(destination), "-t", "PNG", "-l", "M", "-s", "12", "--", payload],
        check=True,
    )


def render_screen(
    out: Path,
    inbox: Path,
    network: str,
    verifier_url: str,
    package_url: str,
    use_qr: bool,
) -> None:
    require_clean(network)
    require_clean(verifier_url)
    require_clean(package_url)
    if network not in NETWORKS:
        die(f"unknown network {network}")
    verifier_a = load_optional(inbox / "verifier-a.json")
    verifier_b = load_optional(inbox / "verifier-b.json")
    preflight_file = load_optional(inbox / "preflight.json")
    shown_a = display_verifier(verifier_a)
    shown_b = display_verifier(verifier_b)
    preflight = display_preflight(preflight_file)
    refuse_secrets({"a": shown_a, "b": shown_b, "preflight": preflight, "network": network})
    badge_class, badge_text = badge_for(network)
    payload = {
        "network": network,
        "badge": badge_text,
        "badge_class": badge_class,
        "preflight": preflight,
        "verifier_a": shown_a,
        "verifier_b": shown_b,
    }
    encoded = json.dumps(payload, indent=2, sort_keys=True).replace("<", "\\u003c")
    page = STAGE_TEMPLATE.read_text(encoding="utf-8")
    replacements = {
        "@@BADGE_CLASS@@": html.escape(badge_class, quote=True),
        "@@BADGE_TEXT@@": html.escape(badge_text),
        "@@NODE_HEIGHT@@": html.escape(preflight["node_height"]),
        "@@ELECTRS_HEIGHT@@": html.escape(preflight["electrs_height"]),
        "@@SEAL_ADDRESS@@": html.escape(preflight["seal_address"]),
        "@@FUNDING_DEPTH@@": html.escape(preflight["funding_depth"]),
        "@@ENTITY_A@@": html.escape(shown_a["entity_id"]),
        "@@ENTITY_B@@": html.escape(shown_b["entity_id"]),
        "@@NAME_A@@": html.escape(shown_a["official_name"]),
        "@@NAME_B@@": html.escape(shown_b["official_name"]),
        "@@SIG_A@@": html.escape(shown_a["claim_valid"]),
        "@@SIG_B@@": html.escape(shown_b["claim_valid"]),
        "@@PANEL_A@@": panel_html("a", shown_a),
        "@@PANEL_B@@": panel_html("b", shown_b),
        "@@SCREEN_JSON@@": encoded,
    }
    for token, value in replacements.items():
        page = page.replace(token, value)
    if "@@" in page:
        die("stage template has an unfilled token")
    out.mkdir(parents=True, exist_ok=True)
    (out / "qr-payload.txt").write_text(qr_payload(verifier_url, package_url), encoding="utf-8")
    write_qr(out, qr_payload(verifier_url, package_url), use_qr)
    (out / "index.html").write_text(page, encoding="utf-8")


def parse_stage_args(args: list[str]) -> tuple[str, str, str, Path | None, bool]:
    network = None
    verifier_url = None
    package_url = None
    inbox = None
    use_qr = True
    index = 0
    while index < len(args):
        arg = args[index]
        if arg == "--no-qr":
            use_qr = False
            index += 1
            continue
        if arg not in {"--network", "--verifier-url", "--package-url", "--inbox"}:
            die(f"unknown argument {arg}")
        if index + 1 >= len(args):
            die(f"{arg} needs a value")
        value = args[index + 1]
        if arg == "--network":
            network = value
        elif arg == "--verifier-url":
            verifier_url = value
        elif arg == "--package-url":
            package_url = value
        else:
            inbox = Path(value)
        index += 2
    if not network or not verifier_url or not package_url:
        die("render needs --network, --verifier-url, and --package-url")
    return network, verifier_url, package_url, inbox, use_qr


def folder_stamp(inbox: Path) -> tuple:
    if not inbox.exists():
        return ()
    stamp = []
    for path in sorted(inbox.iterdir()):
        if path.is_symlink():
            die(f"{path.name} is a symlink")
        stat = path.stat()
        stamp.append((path.name, stat.st_mtime_ns, stat.st_size))
    return tuple(stamp)


def watch_screen(
    out: Path,
    inbox: Path,
    network: str,
    verifier_url: str,
    package_url: str,
    use_qr: bool,
) -> None:
    print(f"screen={out}", flush=True)
    previous = None
    while True:
        stamp = folder_stamp(inbox)
        if stamp != previous:
            render_screen(out, inbox, network, verifier_url, package_url, use_qr)
            previous = stamp
        time.sleep(1)


def loopback_http(url: str) -> bool:
    parsed = urlparse(url)
    if parsed.scheme != "http":
        return False
    if parsed.username is not None or parsed.password is not None:
        return False
    if parsed.hostname not in {"127.0.0.1", "localhost"}:
        return False
    return True


class RefuseRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        die("pull refuses a redirect")


def pull_one(url: str, destination: Path) -> None:
    require_clean(url)
    if not loopback_http(url):
        die("pull accepts only loopback http")
    opener = urllib.request.build_opener(RefuseRedirect)
    with opener.open(url, timeout=5) as response:
        body = response.read()
    destination.write_bytes(body)


def pull_screen(
    out: Path,
    inbox: Path,
    network: str,
    verifier_url: str,
    package_url: str,
    use_qr: bool,
) -> None:
    inbox.mkdir(parents=True, exist_ok=True)
    mapping = (
        ("O2A_VERIFIER_A_URL", inbox / "verifier-a.json"),
        ("O2A_VERIFIER_B_URL", inbox / "verifier-b.json"),
    )
    for env_name, destination in mapping:
        url = os.environ.get(env_name, "").strip()
        if not url:
            die(f"{env_name} is required")
        pull_one(url, destination)
    render_screen(out, inbox, network, verifier_url, package_url, use_qr)


def main(argv: list[str] | None = None) -> None:
    args = list(sys.argv[1:] if argv is None else argv)
    out = OUT
    if args and args[0] == "--out":
        if len(args) < 2:
            die("usage: block0-screen.py --out DIR COMMAND")
        out = Path(args[1])
        args = args[2:]
    if not args:
        die(
            "usage: block0-screen.py init ENTITYID NAME"
            " | verifier A|B STATE HEIGHT | claim NAME"
            " | render --network NETWORK --verifier-url URL --package-url URL"
            " | watch ... | pull ..."
        )
    command = args[0]
    if command in {"init", "verifier", "claim"}:
        run_legacy(out, command, args)
    elif command in {"render", "watch", "pull"}:
        network, verifier_url, package_url, inbox, use_qr = parse_stage_args(args[1:])
        inbox_path = inbox if inbox is not None else out / "inbox"
        if command == "render":
            render_screen(out, inbox_path, network, verifier_url, package_url, use_qr)
        elif command == "watch":
            watch_screen(out, inbox_path, network, verifier_url, package_url, use_qr)
        else:
            pull_screen(out, inbox_path, network, verifier_url, package_url, use_qr)
    else:
        die("unknown command")
    print(f"screen={out}")


if __name__ == "__main__":
    main()
