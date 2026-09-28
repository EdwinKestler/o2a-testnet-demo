---
name: rgb-ecosystem-reference
description: Map of RGB repositories, their version lines and their fit for O2A, so dependencies are chosen deliberately.
when-to-use: Before adding, upgrading or evaluating any RGB, RGB wallet or RGB SDK dependency, or when asked which RGB tooling to use.
---

# RGB ecosystem reference (checked 2026-09-28)

## Two incompatible lines: never mix them in one dependency graph
- RGB-WG v0.12: `github.com/RGB-WG/rgb` 0.12.0-rc.3 (release candidate). Current O2A demo pin, including a patched fork (PATCHES.md).
- rgb-protocol v0.11.1: live on Bitcoin mainnet since July 2025 (RGB Protocol Association, rgb.info). Which line O2A uses for its first transitions is decided by ADR-0010 (pending the 0.11.1 compatibility check).

## rgb-protocol v0.11.1 (Apache-2.0, Rust 1.85)
- rgb-consensus 0.11.1: consensus and validation. Seals are txid+vout (no script constraint); Opret and Tapret; Declarative, Fungible and Structured owned state; validators optional; ResolveWitness for independent validation.
- rgb-schemas 0.11.1: NIA, CFA, UDA, IFA, PFA. A custom O2A identity schema is possible.
- rgb-ops 0.11.1 (standard library), rgb-api 0.11.1 (wallet runtime, BDK support; its PSBT layer is generic).
- rgb-aluvm 0.11.1, rgb-strict-types 1.0.4.
- For O2A seal handling use rgb-ops / rgb-api low-level APIs.

## Tools and wallets (built on rgb-lib)
- rgb-lib (RGB-Tools) 0.3.0-beta.7, pinning 0.11.1-rc.11: the recommended starting point for wallets. It exclusively manages all UTXOs of its wallet, so it is NOT suitable for O2A seal custody. Use it for artist wallets and SplitNight.
- rgb-sandbox (0.11.1 RC6, regtest demo), faucet-rgb (Flask + rgb-lib-python), iris-wallet-android (rgb-lib-kotlin), iris-wallet-desktop (rgb-lightning-node).

## Ecosystem SDKs
- kaleido-sdk: RGB + Lightning; testnet/signet only, not for mainnet.
- UTEXO rgb-sdk-web: browser SDK over rgb-lib (WASM).
- UTEXO wdk-wallet-rgb: Tether WDK module; beta; a single BIP-86 account.

## Rules
Pin exact versions. Run cargo audit and deny and check licenses (DEPENDENCIES.md) before adoption. Record the line and version in DEMO-GATE.md. Standard wallets only display standard schemas, so an O2A identity contract will not appear in Iris or rgb-lib.
