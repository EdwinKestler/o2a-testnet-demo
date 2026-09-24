# Upstream patches

## 0001 — RGB witness height off by one

- Spec tracking: `../o2a-protocol/docs/upstream-needs.md`, item 5.
- Upstream base: RGB-WG `rgb` commit `a1e6b41524131f6d6f183b2235fdaacb5c1abb31`.
- Branch: `o2a/fix-witness-height-offbyone`.
- Fork: `https://github.com/EdwinKestler/rgb.git`.
- Patched revision: `f1e5a68992700ce208da81682ff092c0d5576e82`.
- Patch file: `patches/0001-witness-height-offbyone.patch`.
- Diff SHA-256: `379ea93ff5d706450f83547d307b6039368b9a12378a0c7e2859723796ae732b`.
- Scope: the Electrum resolver's one-line witness-height correction. The
  Esplora resolver reads `block_height` directly and does not have the same
  calculation.
- Cargo source consistency: `rgb-psbt` is also resolved from the same fork
  revision so Rust does not load two distinct copies of the RGB PSBT traits;
  no `rgb-psbt` source is modified by this patch.

remove when RGB-WG releases the fix
