---
name: o2a-gate-review
description: Review another agent's work at a gate, verifying claims against the repository instead of trusting the report.
when-to-use: When asked to review or approve an agent's report, a branch, or evidence before merge.
disable-model-invocation: true
---

# Gate review

1. State: branch, clean worktree, commits relative to base, nothing pushed (`git ls-remote`), the authority commit pinned.
2. Reproduce the counts yourself (suite, freeze 63/63, regression 27/27, conformance, Palimnex). A report is a claim, not evidence.
3. Evidence: `sha256sum -c MANIFEST.sha256`, secret scan, old bundles untouched, RUN.md header honest.
4. Diff review of every fix: does it REPAIR the check, or remove or weaken it? Look for trust in package-supplied data, substring matching, skipped checks, and demo keys on verification paths.
5. Public surfaces: site copy scan for event, ceremony, name and path leaks, and for readiness claims. Run build_site.py.
6. Verdict: pass / pass with notes / not passed, with exact file:line evidence, and the merge commands (merge commit, --match-head-commit).
