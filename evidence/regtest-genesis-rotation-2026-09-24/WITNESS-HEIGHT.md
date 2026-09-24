# RGB witness-height follow-up

**Recorded:** 2026-09-24

This append-only follow-up explains why the milestone-1 validator printed
`Mined(102)` while Bitcoin Core reported that the anchor transaction was mined
at height `103`.

## Definition in the pinned source

The inspected source is RGB-WG `rgb` commit
`a1e6b41524131f6d6f183b2235fdaacb5c1abb31`, file
`src/resolvers/electrum.rs`, line 58. Its defining line is:

> `let height = last_height - verbose.confirmations as u64;`

The resolver passes that result directly to `WitnessStatus::Mined(height)`.
The matching `rgb-std 0.12.0-rc.3` enum documents `Mined` as a public witness
included at a specific block height. Thus the integer is intended to denote
the witness block height. It is not a confirmation count or the last-synced
height.

Bitcoin confirmations are inclusive of the block containing the transaction,
so the correct conversion is `last_height - confirmations + 1`. The pinned
resolver omits the final `+ 1` and reports the anchor height one too low.

## Retained-volume reproduction

Before mining the additional block:

```text
Bitcoin Core tip:             103
Bitcoin Core anchor height:   103
Bitcoin Core confirmations:   1
RGB witness status:           Mined(102)
```

One block was mined to the retained regtest wallet:

```text
new block: 41ed9acd4ff0a6ffd68b049a07da00cefeb6e03e3c606b368b1468c4aa86057e
```

After electrs observed the additional block, a fresh validator imported the
same consignment and O2A object:

```text
Bitcoin Core tip:             104
Bitcoin Core anchor height:   103
Bitcoin Core confirmations:   2
RGB witness status:           Mined(102)
RGB history:                  valid
O2A authorization:            valid
```

The RGB value stayed `102`; it did not advance to `103`. This matches the
off-by-one expression in the pinned resolver. The three verification layers
remain reported separately, and the discrepancy is not normalized or treated
as agreement.

## Classification and disposition

This is an `rgb-runtime` Electrum-resolver off-by-one bug in the pinned RC3
lineage. No O2A verifier rule or normative fixture is changed by this note.
The upstream request is tracked as RGB-WG item 5 in the specification
repository's `docs/upstream-needs.md`.
