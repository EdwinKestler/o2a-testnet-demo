---
name: rgb-ecosystem-reference
description: Map of RGB repositories, their version lines and their fit for O2A, so dependencies are chosen deliberately.
when-to-use: Before adding, upgrading or evaluating any RGB, RGB wallet or RGB SDK dependency, or when asked which RGB tooling to use.
---

# RGB ecosystem reference (checked 2026-09-28)

## Two incompatible lines: never mix them in one dependency graph
- RGB-WG v0.12: `github.com/RGB-WG/rgb` 0.12.0-rc.3 (release candidate). Its O2A compatibility evidence and patched fork remain archived.
- rgb-protocol v0.11.1: the accepted ADR-0010 line for O2A's first transitions, using Opret. Demo commits `31dc5df` and `a86421f` record the compatibility addendum, maintained adapter, regtest lineage, and signet rehearsal; the maintainer reviewed the license register.

O2A's genesis is RGB-line-agnostic. Keep every 0.11.1 adapter in a separate
Cargo workspace from all 0.12 experiments; never place both lines in one
dependency graph. Bridge duplicate `secp256k1` versions with validated bytes,
not Rust types.

## rgb-protocol v0.11.1 (Apache-2.0, Rust 1.85)
- rgb-consensus 0.11.1: consensus and validation. Seals are txid+vout (no script constraint); Opret and Tapret; Declarative, Fungible and Structured owned state; validators optional; ResolveWitness for independent validation.
- rgb-schemas 0.11.1: NIA, CFA, UDA, IFA, PFA. A custom O2A identity schema is possible.
- rgb-ops 0.11.1 (standard library), rgb-api 0.11.1 (wallet runtime, BDK support; its PSBT layer is generic).
- rgb-aluvm 0.11.1, rgb-strict-types 1.0.4.
- For O2A seal handling use rgb-ops / rgb-api low-level APIs with O2A-owned
  seal records and script-path finalization. The demonstrated port is estimated
  at three to five focused days.

## Tools and wallets (built on rgb-lib)
- rgb-lib (RGB-Tools) 0.3.0-beta.7, pinning 0.11.1-rc.11: it exclusively manages all UTXOs of its wallet and MUST NOT be used for O2A seal custody.
- rgb-sandbox (0.11.1 RC6, regtest demo), faucet-rgb (Flask + rgb-lib-python), iris-wallet-android (rgb-lib-kotlin), iris-wallet-desktop (rgb-lightning-node).

## Ecosystem SDKs
- kaleido-sdk: RGB + Lightning; testnet/signet only, not for mainnet.
- UTEXO rgb-sdk-web: browser SDK over rgb-lib (WASM).
- UTEXO wdk-wallet-rgb: Tether WDK module; beta; a single BIP-86 account.

## Rules
Pin exact versions. Keep `cargo audit` and cargo-deny advisory, ban, and source
checks blocking. Run the cargo-deny license check as report-only and record
every non-allowlisted result in the license register before adoption. Under the
2026-09-28 assessment, C1–C7 pass, C8 passes with `hex_lit 0.1.1` registered as
accepted, and C9 selects Opret only. Standard wallets display standard schemas,
so an O2A identity contract will not appear in Iris or rgb-lib.
