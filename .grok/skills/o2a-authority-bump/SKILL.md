---
name: o2a-authority-bump
description: Move the demo to a new o2a-protocol authority commit and prove conformance against every vector at that commit.
when-to-use: When a protocol PR merges and the demo must adopt it (SPEC_COMMIT / DEMO-GATE.md change).
---

# Authority bump

1. Set `SPEC_COMMIT` in crates/o2a-demo-core and the authority line in DEMO-GATE.md to the FULL merge-commit hash on o2a-protocol main. Confirm `git -C ../o2a-protocol rev-parse HEAD` matches, or stop.
2. The conformance test reads vectors with `git -C ../o2a-protocol show <authority>:<path>`, enumerating every tests/vectors/*.json with `git ls-tree`. In Docker, mount the sibling read-only at /o2a-protocol, exactly as dev/README.md documents.
3. The reject mapping is table-driven and bidirectional: an unmapped reject id fails, and so does a mapped id the vectors no longer contain. Cases that exist only as logic in check_protocol_objects.py are ported as tests citing the checker function. Re-diff that file on every bump.
4. Genesis freeze: the demo must reproduce all 63 outputs of genesis-freeze-v0.1.json. Run the protocol's check_genesis_freeze.py on the HOST, because /tmp in the demo image cannot execute Cargo build scripts. Record that it ran on the host.
5. A spec change that alters encodings means a NEW lineage. Never migrate an old lineage; label the old evidence as superseded in a new note, without editing old bundles.
6. Gates: cargo fmt --all --check, cargo check --workspace --locked, cargo test --workspace, cargo audit --ignore RUSTSEC-2024-0436, cargo deny check. The Cargo.lock diff may only add local workspace packages.
