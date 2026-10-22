# Candidate O2A RGB program v0.1 — offline build, 2026-10-22

Disposable evidence for proposed o2a-protocol ADR-0012.
Nothing here is mined; nothing is adopted.

- Source: `spikes/rgb-0.11.1/check/src/program_v01.rs`, base `main`
  `84b82310a317d51002af6913942de12920e6980f` plus branch
  `feat/o2a-program-v0.1`.
- `program-dump.json`: `rgb011-check program-dump`; a second run was
  byte-identical (`cmp`).
- `cargo-test-program_v01.txt`: five tests pass. The test
  `builder_without_digest_outcome_is_recorded` prints `is_err=false`: the
  0.11.1 builder does not itself enforce `transitionDigest` `Once`, so the O2A
  verifier must check it (ADR-0012 rule 2) and the acceptance lineage must
  record consignment-validation behavior.
- The full adapter suite (22 unit + 5 integration tests) passed on this branch.
