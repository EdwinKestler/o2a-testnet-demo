#!/usr/bin/env python3
"""Render tests for the offline stage screen.

Goldens are HTML only. The QR image and qr-payload.txt stay outside them.
"""

import importlib.util
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import unittest
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "docs" / "block0-screen.py"
GOLDEN = Path(__file__).resolve().parent / "testdata"

VERIFIER_URL = "http://127.0.0.1:8765/o2a-verifier"
PACKAGE_URL = "http://127.0.0.1:8765/o2a-package"
DENY = "refusing a field that matches the secret denylist"
ENTITY = "11" * 32
ENTITY_B = "22" * 32
STATE_ID = "33" * 32
SEAL = "44" * 32 + ":0"
TIP = "55" * 32
ADDRESS = "tb1pstageaddress000000000000000000000000000000"

STATES = [
    "CURRENT",
    "PENDING_CONFIRMATION",
    "INCOMPLETE",
    "INVALID",
    "SEAL_CLOSED_WITHOUT_VALID_TRANSITION",
]

FORBIDDEN = [
    "fetch(",
    "xmlhttprequest",
    "websocket",
    "eventsource",
    "sendbeacon",
    "http://",
    "https://",
    "//cdn",
    "@import",
    "url(",
]


def load_screen():
    spec = importlib.util.spec_from_file_location("block0_screen", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def verify_doc(
    state,
    verifier_id,
    entity,
    name,
    claim_valid,
    unspent=True,
    reasons=None,
    confirmations=2,
    required_depth=1,
    network="signet",
    drop_claim=False,
):
    o2a = {
        "genesis_valid": True,
        "official_name": name,
    }
    if not drop_claim:
        o2a["claim_valid"] = claim_valid
    return {
        "entity_id": entity,
        "identity_history_state": state,
        "layers": {
            "bitcoin": {
                "best_block_hash": TIP,
                "confirmations": confirmations,
                "height": 100,
                "required_depth": required_depth,
                "seal_outpoint": SEAL,
                "source": "bitcoin-rpc getrawtransaction+getblockheader",
                "unspent": unspent,
            },
            "o2a": o2a,
            "rgb": {"status": "Consignment is valid"},
        },
        "network": network,
        "reasons": [] if reasons is None else reasons,
        "state_id": STATE_ID,
        "verifier_id": verifier_id,
    }


def case_for(state):
    if state == "CURRENT":
        return "Rehearsal Name", True, [], True
    if state == "PENDING_CONFIRMATION":
        return "Rehearsal Name", True, ["waiting on depth"], True
    if state == "INCOMPLETE":
        return None, False, ["official_name claim is absent"], True
    if state == "INVALID":
        return None, False, ["claim rejected"], True
    return "Rehearsal Name", False, ["seal closed"], False


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def fill_inbox(inbox, state, same_entity=True, network="signet"):
    inbox.mkdir(parents=True, exist_ok=True)
    name, claim, reasons, unspent = case_for(state)
    entity_b = ENTITY if same_entity else ENTITY_B
    write_json(
        inbox / "verifier-a.json",
        verify_doc(state, "laptop-a", ENTITY, name, claim, unspent, reasons, network=network),
    )
    write_json(
        inbox / "verifier-b.json",
        verify_doc(state, "laptop-b", entity_b, name, claim, unspent, reasons, network=network),
    )
    write_json(
        inbox / "preflight.json",
        {
            "electrs_height": 119,
            "funding_confirmations": 4,
            "node_height": 120,
            "required_depth": 6,
            "seal_address": ADDRESS,
        },
    )


def render(out, inbox, network="signet", no_qr=True, verifier_url=VERIFIER_URL, package_url=PACKAGE_URL):
    command = [
        sys.executable,
        str(SCRIPT),
        "--out",
        str(out),
        "render",
        "--network",
        network,
        "--verifier-url",
        verifier_url,
        "--package-url",
        package_url,
        "--inbox",
        str(inbox),
    ]
    if no_qr:
        command.append("--no-qr")
    return subprocess.run(command, capture_output=True, text=True, cwd=ROOT)


def program_script(page):
    parts = re.findall(r"<script\b([^>]*)>(.*?)</script>", page, re.S)
    programs = [body for attrs, body in parts if "application/json" not in attrs]
    if len(programs) != 1:
        raise AssertionError(f"expected one program script, found {len(programs)}")
    return programs[0]


def badge_tag(page):
    match = re.search(r'<p id="network-badge"[^>]*>.*?</p>', page, re.S)
    if match is None:
        raise AssertionError("network badge missing")
    return match.group(0)


def assert_png(png):
    if not png.startswith(b"\x89PNG\r\n\x1a\n"):
        raise AssertionError("not a PNG")
    offset = 8
    idat = []
    saw_iend = False
    while offset + 12 <= len(png):
        length = struct.unpack(">I", png[offset : offset + 4])[0]
        if offset + 12 + length > len(png):
            raise AssertionError("truncated PNG")
        tag = png[offset + 4 : offset + 8]
        data = png[offset + 8 : offset + 8 + length]
        crc = struct.unpack(">I", png[offset + 8 + length : offset + 12 + length])[0]
        expect = zlib.crc32(tag + data) & 0xFFFFFFFF
        if crc != expect:
            raise AssertionError(f"bad CRC on {tag!r}")
        if tag == b"IDAT":
            idat.append(data)
        offset += 12 + length
        if tag == b"IEND":
            saw_iend = True
            break
    if not saw_iend or not idat:
        raise AssertionError("PNG has no image data")
    zlib.decompress(b"".join(idat))


class StageScreenTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.screen = load_screen()

    def test_goldens_for_each_history_state(self):
        for state in STATES:
            with self.subTest(state=state):
                with tempfile.TemporaryDirectory() as tmp:
                    tmp = Path(tmp)
                    inbox = tmp / "inbox"
                    out = tmp / "out"
                    fill_inbox(inbox, state)
                    result = render(out, inbox)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    page = (out / "index.html").read_text(encoding="utf-8")
                    slug = state.lower().replace("_", "-")
                    expected_path = GOLDEN / f"golden-{slug}.html"
                    expected = expected_path.read_text(encoding="utf-8")
                    self.assertEqual(page, expected)
                    self.assertIn(f'<dd id="a-identity_history_state">{state}</dd>', page)
                    self.assertIn(f'<dd id="b-identity_history_state">{state}</dd>', page)
                    self.assert_displayed_page(page, out)

    def assert_displayed_page(self, page, out):
        self.assertIn("#0E1230", page)
        self.assertIn("#F2B33D", page)
        self.assertIn("#EEF0FA", page)
        self.assertIn("#141A40", page)
        self.assertIn('<span id="funding-depth">4 / 6</span>', page)
        self.assertIn('<span id="node-height">120</span>', page)
        self.assertIn('<span id="electrs-height">119</span>', page)
        self.assertIn(f'<span id="seal-address" class="mono">{ADDRESS}</span>', page)
        self.assertIn('<dd id="a-confirmations">2</dd>', page)
        self.assertIn('<dd id="a-required_depth">1</dd>', page)
        self.assertIn("signature valid", page)
        self.assertIn(ENTITY, page)
        self.assertNotIn(VERIFIER_URL, page)
        self.assertNotIn(PACKAGE_URL, page)
        self.assertNotIn("@@", page)
        self.assertNotIn("differ", page.lower())
        lowered = page.lower()
        for token in FORBIDDEN:
            self.assertNotIn(token, lowered, token)
        script = program_script(page)
        for state in STATES:
            self.assertNotIn(state, script)
        self.assertNotIn("fetch(", script.lower())
        payload = (out / "qr-payload.txt").read_text(encoding="utf-8")
        self.assertEqual(payload, f"verifier\n{VERIFIER_URL}\npackage\n{PACKAGE_URL}\n")
        self.assertIn(
            '<div class="qr-slot"><img id="qr" src="stage-qr.png" alt="QR code"></div>',
            page,
        )
        self.assertNotIn("::after", page)
        self.assertNotIn("::before", page)
        png = (out / "stage-qr.png").read_bytes()
        self.assertEqual(png, self.screen.PLACEHOLDER_PNG)
        assert_png(png)

    def test_offline_page_and_template_make_no_network_requests(self):
        template = (ROOT / "docs" / "stage-screen" / "stage.html").read_text(encoding="utf-8")
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            out = tmp / "out"
            fill_inbox(inbox, "CURRENT")
            result = render(out, inbox)
            self.assertEqual(result.returncode, 0, result.stderr)
            page = (out / "index.html").read_text(encoding="utf-8")
        for label, text in (("template", template), ("page", page)):
            lowered = text.lower()
            for token in FORBIDDEN:
                self.assertNotIn(token, lowered, f"{label} contains {token}")
        self.assertNotIn(VERIFIER_URL, page)
        self.assertNotIn(PACKAGE_URL, page)
        script = program_script(page)
        for state in STATES:
            self.assertNotIn(state, script)

    def test_network_badge(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            fill_inbox(inbox, "CURRENT", network="signet")
            mainnet = render(tmp / "main", inbox, network="mainnet")
            signet = render(tmp / "sig", inbox, network="signet")
            regtest = render(tmp / "reg", inbox, network="regtest")
            self.assertEqual(mainnet.returncode, 0, mainnet.stderr)
            self.assertEqual(signet.returncode, 0, signet.stderr)
            self.assertEqual(regtest.returncode, 0, regtest.stderr)
            main_page = (tmp / "main" / "index.html").read_text(encoding="utf-8")
            sig_page = (tmp / "sig" / "index.html").read_text(encoding="utf-8")
            reg_page = (tmp / "reg" / "index.html").read_text(encoding="utf-8")
        main_badge = badge_tag(main_page)
        sig_badge = badge_tag(sig_page)
        reg_badge = badge_tag(reg_page)
        self.assertIn('class="badge badge-mainnet"', main_badge)
        self.assertIn(">MAINNET<", main_badge)
        self.assertIn("#FF2A2A", main_page)
        self.assertIn("font-size: 4.5rem", main_page)
        self.assertIn('<dd id="a-network">signet</dd>', main_page)
        self.assertIn('class="badge"', sig_badge)
        self.assertNotIn("badge-mainnet", sig_badge)
        self.assertIn(">SIGNET<", sig_badge)
        self.assertIn('class="badge"', reg_badge)
        self.assertNotIn("badge-mainnet", reg_badge)
        self.assertIn(">REGTEST<", reg_badge)

    def test_denylist_refuses_the_field_and_writes_no_page(self):
        samples = [
            {"official_name": "xprvMARKER", "reasons": ["public"]},
            {"official_name": "Rehearsal Name", "reasons": ["see notes/seed"]},
            {"official_name": "wallet/seed.hex", "reasons": []},
            {"official_name": "O2A_DEMO_SEED", "reasons": []},
        ]
        for sample in samples:
            with self.subTest(sample=sample["official_name"]):
                with tempfile.TemporaryDirectory() as tmp:
                    tmp = Path(tmp)
                    inbox = tmp / "inbox"
                    inbox.mkdir()
                    document = verify_doc(
                        "CURRENT",
                        "laptop-a",
                        ENTITY,
                        sample["official_name"],
                        True,
                        reasons=sample["reasons"],
                    )
                    write_json(inbox / "verifier-a.json", document)
                    result = render(tmp / "out", inbox)
                    self.assertEqual(result.returncode, 1)
                    self.assertEqual(result.stderr.strip(), DENY)
                    self.assertNotIn("xprvMARKER", result.stderr)
                    self.assertNotIn("notes/seed", result.stderr)
                    self.assertNotIn("seed.hex", result.stderr)
                    self.assertNotIn("O2A_DEMO_SEED", result.stderr)
                    self.assertFalse((tmp / "out" / "index.html").exists())
                    self.assertFalse((tmp / "out" / "qr-payload.txt").exists())

    def test_bare_seed_word_and_rehearsal_name_render(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            inbox.mkdir()
            write_json(
                inbox / "verifier-a.json",
                verify_doc("CURRENT", "laptop-a", ENTITY, "Seed", True),
            )
            write_json(
                inbox / "verifier-b.json",
                verify_doc("CURRENT", "laptop-b", ENTITY, "Rehearsal Name", True),
            )
            write_json(
                inbox / "preflight.json",
                {
                    "electrs_height": 119,
                    "funding_confirmations": 4,
                    "node_height": 120,
                    "required_depth": 6,
                    "seal_address": ADDRESS,
                },
            )
            result = render(tmp / "out", inbox)
            self.assertEqual(result.returncode, 0, result.stderr)
            page = (tmp / "out" / "index.html").read_text(encoding="utf-8")
            self.assertIn(">Seed<", page)
            self.assertIn(">Rehearsal Name<", page)

    def test_secret_marker_matches_the_rust_denylist(self):
        marked = self.screen.secret_marked
        self.assertTrue(marked("xprvMARKER"))
        self.assertTrue(marked("TPRV"))
        self.assertTrue(marked("mnemonic phrase"))
        self.assertTrue(marked("seed.hex"))
        self.assertTrue(marked("O2A_DEMO_SEED"))
        self.assertTrue(marked("notes/seed"))
        self.assertTrue(marked("wallet/seed.hex"))
        self.assertTrue(marked("C:\\seed"))
        self.assertTrue(marked("dir/seed.txt"))
        self.assertFalse(marked("Seed"))
        self.assertFalse(marked("Rehearsal Name"))
        self.assertFalse(marked("seedling"))
        self.assertFalse(marked("foo/seedling"))
        self.assertFalse(marked(VERIFIER_URL))

    def test_missing_files_and_missing_claim_are_not_false(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            inbox.mkdir()
            empty = render(tmp / "empty", inbox)
            self.assertEqual(empty.returncode, 0, empty.stderr)
            page = (tmp / "empty" / "index.html").read_text(encoding="utf-8")
            self.assertIn('<p id="entity-a" class="entity">missing</p>', page)
            self.assertIn('<span id="funding-depth">missing</span>', page)
            self.assertIn('<span id="sig-a">missing</span>', page)
            self.assertNotIn('<span id="sig-a">false</span>', page)
            write_json(
                inbox / "verifier-a.json",
                verify_doc("CURRENT", "laptop-a", ENTITY, "Rehearsal Name", True, drop_claim=True),
            )
            partial = render(tmp / "partial", inbox)
            self.assertEqual(partial.returncode, 0, partial.stderr)
            partial_page = (tmp / "partial" / "index.html").read_text(encoding="utf-8")
            self.assertIn('<span id="sig-a">missing</span>', partial_page)
            self.assertIn(">Rehearsal Name<", partial_page)
            write_json(
                inbox / "preflight.json",
                {"funding_confirmations": 4, "node_height": 120, "seal_address": ADDRESS},
            )
            depths = render(tmp / "depth", inbox)
            self.assertEqual(depths.returncode, 0, depths.stderr)
            depth_page = (tmp / "depth" / "index.html").read_text(encoding="utf-8")
            self.assertIn('<span id="node-height">120</span>', depth_page)
            self.assertIn('<span id="funding-depth">missing</span>', depth_page)

    def test_null_official_name_is_the_word_null(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            fill_inbox(inbox, "INCOMPLETE")
            result = render(tmp / "out", inbox)
            self.assertEqual(result.returncode, 0, result.stderr)
            page = (tmp / "out" / "index.html").read_text(encoding="utf-8")
            self.assertIn('<span id="name-a">null</span>', page)
            self.assertIn('<span id="sig-a">false</span>', page)
            self.assertIn('<dd id="a-official_name">null</dd>', page)

    def test_different_verifier_files_both_stay_on_the_page(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            inbox.mkdir()
            write_json(
                inbox / "verifier-a.json",
                verify_doc("CURRENT", "laptop-a", ENTITY, "Rehearsal Name", True),
            )
            write_json(
                inbox / "verifier-b.json",
                verify_doc(
                    "PENDING_CONFIRMATION",
                    "laptop-b",
                    ENTITY_B,
                    "Second Name",
                    False,
                    reasons=["second file"],
                ),
            )
            write_json(
                inbox / "preflight.json",
                {
                    "electrs_height": 119,
                    "funding_confirmations": 4,
                    "node_height": 120,
                    "required_depth": 6,
                    "seal_address": ADDRESS,
                },
            )
            result = render(tmp / "out", inbox)
            self.assertEqual(result.returncode, 0, result.stderr)
            page = (tmp / "out" / "index.html").read_text(encoding="utf-8")
            self.assertIn(ENTITY, page)
            self.assertIn(ENTITY_B, page)
            self.assertIn(">CURRENT<", page)
            self.assertIn(">PENDING_CONFIRMATION<", page)
            self.assertIn(">Rehearsal Name<", page)
            self.assertIn(">Second Name<", page)
            self.assertIn('<span id="sig-a">true</span>', page)
            self.assertIn('<span id="sig-b">false</span>', page)
            self.assertNotIn("differ", page.lower())
            for state in STATES:
                self.assertNotIn(state, program_script(page))

    def test_markup_in_a_field_is_escaped(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            inbox.mkdir()
            write_json(
                inbox / "verifier-a.json",
                verify_doc(
                    "CURRENT",
                    "laptop-a",
                    ENTITY,
                    "<script>alert(1)</script>",
                    True,
                ),
            )
            result = render(tmp / "out", inbox)
            self.assertEqual(result.returncode, 0, result.stderr)
            page = (tmp / "out" / "index.html").read_text(encoding="utf-8")
            self.assertIn("&lt;script&gt;alert(1)&lt;/script&gt;", page)
            self.assertIn("\\u003cscript>alert(1)\\u003c/script>", page)
            self.assertNotIn("<script>alert", page)
            self.assertEqual(len(re.findall(r"<script\b", page)), 2)

    def test_unknown_network_and_secret_url_write_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            fill_inbox(inbox, "CURRENT")
            unknown = render(tmp / "bad-net", inbox, network="testnet")
            self.assertEqual(unknown.returncode, 1)
            self.assertIn("unknown network testnet", unknown.stderr)
            self.assertFalse((tmp / "bad-net" / "index.html").exists())
            secret_url = render(
                tmp / "bad-url",
                inbox,
                verifier_url="http://127.0.0.1/wallet/seed.hex",
            )
            self.assertEqual(secret_url.returncode, 1)
            self.assertEqual(secret_url.stderr.strip(), DENY)
            self.assertNotIn("seed.hex", secret_url.stderr)
            self.assertFalse((tmp / "bad-url" / "index.html").exists())

    def test_symlink_inbox_file_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            inbox.mkdir()
            write_json(inbox / "real.json", verify_doc("CURRENT", "laptop-a", ENTITY, "Rehearsal Name", True))
            (inbox / "verifier-a.json").symlink_to(inbox / "real.json")
            result = render(tmp / "out", inbox)
            self.assertEqual(result.returncode, 1)
            self.assertIn("verifier-a.json is a symlink", result.stderr)
            self.assertFalse((tmp / "out" / "index.html").exists())

    def test_pull_refuses_public_urls_before_a_connection(self):
        loopback = self.screen.loopback_http
        self.assertTrue(loopback("http://127.0.0.1:9/verify.json"))
        self.assertTrue(loopback("http://localhost/verify.json"))
        self.assertFalse(loopback("https://127.0.0.1/verify.json"))
        self.assertFalse(loopback("https://example.com/verify.json"))
        self.assertFalse(loopback("http://user:secret@127.0.0.1/verify.json"))
        self.assertFalse(loopback("http://203.0.113.5/verify.json"))
        self.assertFalse(loopback("http://[::1]:9/verify.json"))
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            env = os.environ.copy()
            env["O2A_VERIFIER_A_URL"] = "https://example.com/verify.json"
            env["O2A_VERIFIER_B_URL"] = "http://203.0.113.5/verify.json"
            started = time.monotonic()
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--out",
                    str(tmp / "out"),
                    "pull",
                    "--network",
                    "signet",
                    "--verifier-url",
                    VERIFIER_URL,
                    "--package-url",
                    PACKAGE_URL,
                ],
                capture_output=True,
                text=True,
                cwd=ROOT,
                env=env,
            )
            elapsed = time.monotonic() - started
            self.assertEqual(result.returncode, 1)
            self.assertIn("pull accepts only loopback http", result.stderr)
            self.assertNotIn("example.com", result.stderr)
            self.assertLess(elapsed, 1.0)
            self.assertFalse((tmp / "out" / "index.html").exists())

    def test_legacy_rehearsal_page_still_signs_the_claim_line(self):
        entity = "12" * 32
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            init = subprocess.run(
                [sys.executable, str(SCRIPT), "--out", str(out), "init", entity, "Rehearsal Name"],
                capture_output=True,
                text=True,
                cwd=ROOT,
            )
            self.assertEqual(init.returncode, 0, init.stderr)
            claim = subprocess.run(
                [sys.executable, str(SCRIPT), "--out", str(out), "claim", "Rehearsal Name"],
                capture_output=True,
                text=True,
                cwd=ROOT,
            )
            self.assertEqual(claim.returncode, 0, claim.stderr)
            verifier = subprocess.run(
                [sys.executable, str(SCRIPT), "--out", str(out), "verifier", "A", "CURRENT", "12"],
                capture_output=True,
                text=True,
                cwd=ROOT,
            )
            self.assertEqual(verifier.returncode, 0, verifier.stderr)
            page = (out / "index.html").read_text(encoding="utf-8")
            self.assertIn(entity, page)
            self.assertIn("Name claim signed:", page)
            self.assertIn("Rehearsal Name", page)
            self.assertIn("CURRENT at height 12", page)
            self.assertIn("O2A identity", page)
            self.assertNotIn("xprv", page.lower())
            leaked = subprocess.run(
                [sys.executable, str(SCRIPT), "--out", str(out), "claim", "leak xprvMARKER"],
                capture_output=True,
                text=True,
                cwd=ROOT,
            )
            self.assertEqual(leaked.returncode, 1)
            self.assertEqual(leaked.stderr.strip(), DENY)
            self.assertNotIn("xprvMARKER", leaked.stderr)
            kept = (out / "index.html").read_text(encoding="utf-8")
            self.assertNotIn("xprvMARKER", kept)
            self.assertIn("Name claim signed:", kept)

    def test_legacy_init_refuses_a_secret_label(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--out",
                    str(out),
                    "init",
                    "12" * 32,
                    "has xprvMARKER",
                ],
                capture_output=True,
                text=True,
                cwd=ROOT,
            )
            self.assertEqual(result.returncode, 1)
            self.assertEqual(result.stderr.strip(), DENY)
            self.assertNotIn("xprvMARKER", result.stderr)
            self.assertFalse((out / "index.html").exists())

    def test_watch_renders_the_inbox_once(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            out = tmp / "out"
            fill_inbox(inbox, "CURRENT")
            proc = subprocess.Popen(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--out",
                    str(out),
                    "watch",
                    "--network",
                    "signet",
                    "--verifier-url",
                    VERIFIER_URL,
                    "--package-url",
                    PACKAGE_URL,
                    "--inbox",
                    str(inbox),
                    "--no-qr",
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                cwd=ROOT,
                text=True,
            )
            try:
                deadline = time.monotonic() + 3
                page_path = out / "index.html"
                while time.monotonic() < deadline and not page_path.exists():
                    if proc.poll() is not None:
                        break
                    time.sleep(0.05)
                self.assertTrue(page_path.exists(), "watch did not write the page")
                page = page_path.read_text(encoding="utf-8")
                self.assertIn(ENTITY, page)
                self.assertNotIn(VERIFIER_URL, page)
            finally:
                proc.kill()
                proc.communicate(timeout=2)

    @unittest.skipUnless(shutil.which("qrencode"), "qrencode is not installed")
    def test_qrencode_png_keeps_urls_out_of_the_html(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            inbox = tmp / "inbox"
            out = tmp / "out"
            fill_inbox(inbox, "CURRENT")
            result = render(out, inbox, no_qr=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            png = (out / "stage-qr.png").read_bytes()
            assert_png(png)
            self.assertGreater(len(png), len(self.screen.PLACEHOLDER_PNG))
            page = (out / "index.html").read_text(encoding="utf-8")
            self.assertNotIn(VERIFIER_URL, page)
            self.assertNotIn(PACKAGE_URL, page)
            self.assertNotIn("http://", page.lower())
            self.assertNotIn("https://", page.lower())


if __name__ == "__main__":
    unittest.main()
