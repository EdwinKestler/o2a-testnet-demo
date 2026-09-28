---
name: o2a-block0-ceremony
description: Execute the block-0 stage ceremony runbook (first EntityID minted live) as a rehearsal or, only when explicitly authorized, for real.
when-to-use: Only when the maintainer invokes it for a rehearsal or the authorized ceremony.
disable-model-invocation: true
---

# Block-0 ceremony

docs/block0-runbook.md is authoritative. Follow it in order and time every step.

## Preconditions
- Rehearsal: signet only (see o2a-signet-ops).
- Real ceremony: ADR-0009 Accepted, the maintainer has explicitly authorized mainnet in this session, and the seal was funded at least 48 hours ahead with 6 confirmations. Otherwise refuse.

## Pre-show
- Seed ceremony: the seed is never shown on any screen.
- Recovery set: 2-of-3. The artist holds two shares in different physical places, and a person the artist names holds the third. O2A staff hold none. Default delay 1008 blocks; the artist may change it.
- Pre-flight: recompute the seal address from the planned policy and confirm it with Core deriveaddresses. A wrong parameter (for example the delay) must produce a different, unfunded address.
- Fund the seal with a non-replaceable transaction; never fee-bump it yourself.

## Stage (target: seconds)
Sign the genesis offline, show the EntityID and its QR code, have two independent validators report CURRENT, have the artist sign the official_name claim, and show the claim in the verifier view. The projector shows only the local page built by docs/block0-screen.py. No seeds, xprv, private paths or wallet internals.

## Post-show
Back up the genesis package and consignment in at least two places, one held by the artist. Restore on a clean directory and re-verify CURRENT without any seed.

## Hard rules
Zero manual edits of records or files during the flow; any improvisation is a runbook failure to report. No transitions on the identity until the RGB stack is final (ADR-0009).
