# Addendum: D14 license-gate rerun

Date: 2026-09-28. Workspace: `spikes/rgb-0.11.1` on `spike/rgb-0.11.1`.

This directory is a new bundle. `evidence/regtest-rgb011-compat-2026-09-28/` is unchanged. That bundle still records the 2026-09-24 gate, including the license failure for `hex_lit 0.1.1`.

Decision D14 in `../o2a-protocol/docs/24-decision-audit-2026-09.md` makes licenses an assessment. The register entry for `hex_lit 0.1.1` (MITNFA) is `accepted` in `../o2a-protocol/docs/22-license-and-adoption-assessment.md`. MITNFA is not on the allowlist.

| Command | Exit | Summary |
| --- | --- | --- |
| `cargo deny check advisories bans sources` | 0 | `advisories ok, bans ok, sources ok` |
| `cargo deny check licenses` | 4 | `licenses FAILED`. The only rejection is `hex_lit 0.1.1`, license `MITNFA`. |
| `bash check-dependency-policy.sh` | 0 | Blocking checks passed. The script printed `cargo deny licenses report-only: exit 4`. |

Duplicate-version warnings remain warnings. No chain rerun and no adapter port are in this addendum.

| File | Contents |
| --- | --- |
| `deny-advisories-bans-sources.txt` | Blocking command and `EXIT:0` |
| `deny-licenses.txt` | Report-only command and `EXIT:4` |
| `check-dependency-policy.txt` | Spike gate script, exit 0 |
| `MANIFEST.sha256` | Hashes of the files in this directory |
