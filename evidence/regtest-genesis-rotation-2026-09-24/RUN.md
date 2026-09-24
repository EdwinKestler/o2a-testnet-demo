# Regtest genesis and controller-rotation evidence

**Run date:** 2026-09-24

**Network:** isolated Bitcoin regtest through local electrs

**Status:** successful disposable demo-lineage evidence; not a Phase 0 gate closure

## Authority and environment

- O2A specification authority: sibling `../o2a-protocol` at
  `3ca98ea9b60256f271e71fa89caa09448e804e87`.
- RGB lineage: RGB-WG `rgb` `v0.12.0-rc.3`, commit
  `a1e6b41524131f6d6f183b2235fdaacb5c1abb31`.
- Toolchain: `rustc 1.98.1`, `cargo 1.98.1`, `cargo-audit 0.22.2`, and
  `cargo-deny 0.20.2` in the pinned toolchain container.
- Chain services: Bitcoin Core 31.1 and electrs 0.12.0, with no published host
  ports.
- Identity derivation: demo-stable v0.1, entity index `0`, using the published
  BIP39 `abandon` x11 + `about` / `TREZOR` test seed. This seed and every
  identity derived from it are permanently unsafe and disposable.

## Procedure

1. Start Bitcoin Core and electrs on the internal Compose regtest network.
2. Create a descriptor wallet and mine 101 blocks to mature coinbase funds.
3. Derive two taproot funding addresses, fund each with 0.01 BTC, and mine the
   transactions. The two already-funded outpoints avoid a circular commitment:
   one is the genesis identity seal and the other is the successor seal.
4. Issue the RGB genesis with the canonical O2A genesis digest in its owned
   state at `ef571a9c947523ce0838f062529fcebca40b3adb2b264d48b2f75c4aaafcfb1f:1`.
5. Build the canonical controller-rotation object for controller index `1`.
   Spend the genesis seal in a PSBT carrying the RC3 RGB commitment, assign the
   RGB identity state to
   `aa4c4706b1a8cf13228d67b3a29b9472bcde889e06900f447689752d9d777944:1`,
   broadcast, and mine one confirmation.
6. Give each of two fresh processes only `rotation.rgb`, `rotation.o2a`, and
   access to the same chain through local electrs. Each process imports and
   validates independently. Their outputs are byte-identical.

The OP_RETURN produced by the RC3 PSBT implementation is an implementation
detail of this disposable run. It does not freeze a normative O2A Bitcoin
carrier or RGB program.

## Identifiers

- Contract:
  `contract:kmCwN~Co-omXaJq~-axMdmZI-k0HTJgX-G2yXCDK-9O~MjBk`
- Genesis cell: `bOFyMD63lZ4DxFaBadVN3VsUKDMSTwPMhfRzhTzY3Qs:0`
- Rotation cell: `vZTLQrfPxiAvLVPRw8g8A6UhLMd2vSOBaX7bYSAGJRU:0`
- Rotation anchor txid:
  `a703450cf1d0c427836537a3ad1ee8e928944b0e82ee36174303c0060fa6bed9`
- Anchor block:
  `766efdc33487611996c373ed269d3b5675edcc6d6fe5ff206660501b3ad6a4b7`
- RC3 anchor commitment:
  `b1f0b96a56a4c6299212575f44f8ed5d7948ed5056cf140c9485300a0e4f2d57`
- Canonical O2A transition digest:
  `57222a18ab43fe5ef9d8d637d5a1598d9e6e993bdc3755999da2eff00ba066a9`
- RGB consignment SHA-256:
  `f939b65f4877542d8794dee21cf4d62f0c1deaebba3cf49aa87b37f7912ecc64`
- Signed O2A object SHA-256:
  `caa859e950515953d94a3467006e4de936c1fd74a82b38d7014b5a17173035c6`

## Three independent verification layers

| Layer | Result | What was checked |
|---|---|---|
| Bitcoin order/spend | pass | the anchor is mined, spends the exact genesis outpoint, and the successor outpoint remains unspent |
| RGB history | pass | RC3 imported the consignment, resolved witnesses through electrs, closed the prior seal, and projected the same current cell and seal |
| O2A authorization | pass | the object has the exact regtest controller-rotation shape, controller role and capability, BIP340 signature, consistent prior state, and a digest equal to the RGB owned-state commitment |

See `wallet-before.txt`, `wallet-after.txt`, `validator-a.txt`,
`validator-b.txt`, and `checks.txt`. `MANIFEST.sha256` covers every retained
file in this bundle except the manifest itself.
