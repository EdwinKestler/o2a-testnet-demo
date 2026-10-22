# Lock review — rgb-protocol 0.11.1 adapter, 2026-10-22

Disposable review evidence for proposed o2a-protocol ADR-0013. It adopts
nothing by itself.

- Workspace: `spikes/rgb-0.11.1` (separate from the archived 0.12 graph).
- Base commit: `84b82310a317d51002af6913942de12920e6980f` (`main`); the lock
  file is unchanged since `2ddfe8c`.
- Lock: `Cargo.lock`, 139 packages, SHA-256 in `lock.sha256`
  (`f110a9292dd5fd57e71951133b5781869bc0778cdba7330412e6d93e1d1c515e`).
- Tools: `tools.txt` (rustc/cargo 1.98.1, cargo-audit 0.22.2, cargo-deny 0.20.2).

## Results

| Check | File | Result |
| --- | --- | --- |
| `cargo audit --file Cargo.lock` | `cargo-audit.txt` | exit 0; 1348 advisories loaded; no vulnerability and no warning reported for 139 crates |
| `cargo deny check advisories bans sources` | `cargo-deny.txt` | `advisories ok, bans ok, sources ok` |
| `cargo deny check licenses` (report-only, D14) | `cargo-deny.txt` | exit 4: only `hex_lit 0.1.1` (MITNFA), already accepted in the o2a-protocol license register |
| Normal-edge dependency tree | `cargo-tree-normal.txt` | recorded |

Observations for the ADR:

- `paste` is absent from this graph; the 0.12 `RUSTSEC-2024-0436` warning
  does not apply.
- `bitcoin 0.32.102` is present through `rgb-consensus 0.11.1`. The adapter's
  only signing path is BIP341 script-path and key-path `TapSighashType::Default`
  (`check/src/spend.rs`), not the SegWit-v0 path named in the standing
  exclusion. Consensus validation stays with Bitcoin Core.
- Two `secp256k1` versions (0.29.1 and 0.33.1) are present; ADR-0010 item 7
  (byte-level bridge) applies.
- Duplicate-version warnings (13 crates) are `warn` under `deny.toml`.
