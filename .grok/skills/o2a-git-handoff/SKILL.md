---
name: o2a-git-handoff
description: Commit, PR and merge conventions for the O2A repositories, including remote layout and evidence reachability.
when-to-use: Before committing, and whenever preparing a handoff, PR or merge instructions.
---

# Git handoff

- Commit locally in the repository's style (imperative, sentence case). Never push, open PRs, tag, release or deploy unless the maintainer authorizes it in the current session.
- Merges use merge commits only, never squash or rebase, so evidence commits cited by specs (for example demo bce2b58) stay reachable.
- Merge with the verified head: `gh pr merge <NUMBER> --repo <owner/repo> --merge --match-head-commit <sha>`. With `--repo`, the PR number or branch name is required.
- o2a-testnet-demo has one remote, `origin` = EdwinKestler/o2a-testnet-demo. Pushing main redeploys the demo site (pages.yml runs on every push to main).
- If another agent has the working tree checked out, update main without switching: `git fetch origin main:main`.
- Report states separately: committed locally / pushed / PR open / merged / deployed / independently verified.
