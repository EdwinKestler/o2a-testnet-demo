# Verify document and public package

`verify --json` prints one verification document.
`publish-package` writes the public file set.
Both formats are closed: a reader rejects extra fields.

## Verify document

Command:

```text
rgb011-check verify --json
rgb011-check signet-verify --json
```

Stdout is one JSON document and a trailing newline.
The text report is not printed in this mode.
Errors stay on stderr.
`--json` with any other command is rejected.

The process exits 1 after the document when any of these are true:

- `identity_history_state` is not `CURRENT`;
- the RGB consignment check did not succeed;
- `claim.o2a` is present and the claim is not a valid `official_name`.

A missing `claim.o2a` sets `claim_valid` to false, `official_name` to null, and adds the reason `official_name claim is absent`.
That absence alone does not fail the process.

| Field | Where it comes from |
| --- | --- |
| `network` | Active profile (`regtest`, `signet`, or `mainnet`) |
| `verifier_id` | `O2A_VERIFIER_ID`, or `local` when unset |
| `entity_id` | Recomputed from `genesis.o2a` |
| `state_id` | Recomputed from the signed genesis state |
| `identity_history_state` | Lineage evaluator |
| `layers.bitcoin.seal_outpoint` | Next seal inside the signed genesis |
| `layers.bitcoin.confirmations` | Tip height and the seal-creating block |
| `layers.bitcoin.required_depth` | Active profile |
| `layers.bitcoin.unspent` | `gettxout` on that outpoint |
| `layers.bitcoin.source` | `bitcoin-rpc getrawtransaction+getblockheader` |
| `layers.bitcoin.best_block_hash` | `getblockchaininfo` |
| `layers.bitcoin.height` | Tip height |
| `layers.rgb.status` | Consignment validation text |
| `layers.o2a.genesis_valid` | Genesis signature check |
| `layers.o2a.claim_valid` | Claim signature, authorization, and `official_name` predicate |
| `layers.o2a.official_name` | Claim object bytes after those checks, otherwise null |
| `reasons` | Non-ok layer strings |

`public.txt` and `package.json` are not inputs.
The tool does not copy `entity_id`, `state_id`, `seal_outpoint`, `network`, or `official_name` from either file.

`genesis.strict` is the RGB consignment.
`verify` still requires it.
It is not part of the public package.
A verifier who has only the three public files cannot finish the RGB check.

Schema: [verify-v1.schema.json](verify-v1.schema.json).

## Public package

Command:

```text
rgb011-check publish-package <empty-directory>
```

The source directory is `RGB011_EVIDENCE`.
The command writes exactly these files:

| File | Role |
| --- | --- |
| `genesis.o2a` | Signed genesis. Copied as a regular file. |
| `claim.o2a` | Signed `official_name` claim. Required. |
| `package.json` | Hints written by this command. |

The allow-list is those two source files.
`package.json` in the source is not read and not copied.
Seed files, key files, wallet files, RPC files, and symlinks are not copied.
A symlink at `genesis.o2a` or `claim.o2a` is refused.
The destination must be absent or empty.
The command does not derive keys and does not open a node connection.

Every `package.json` field is a hint.
A verifier recomputes `entity_id`, `state_id`, `seal_outpoint`, and `official_name` from the two signed files.
`created_at_height` is the one value taken from the tool-written `created_at_height` line in `public.txt`.
The other `public.txt` lines are ignored.
`network` in the package is the active profile, not the `network` line in `public.txt`.

`format` is `o2a.public-package/v1`.

Schema: [public-package-v1.schema.json](public-package-v1.schema.json).
