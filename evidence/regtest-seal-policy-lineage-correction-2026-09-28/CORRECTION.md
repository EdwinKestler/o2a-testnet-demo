Disposable demo-lineage evidence. This note closes no Phase 0 gate.

This note corrects the reading of `evidence/regtest-seal-policy-lineage-2026-09-26/`. That bundle is unchanged.

# Identities 1 and 2 share one root

Identity 1 and identity 2 in the 2026-09-26 bundle were both built from demo entity index 0.

`o2a-demo-core` `demo_keys()` derives that index from the published unsafe seed:

```text
identity_key(coin = 1, entity = 0, role, index)
```

The ceremony CLI calls that function for every seal-policy identity. Identity 2 used a separate data directory and a separate RGB contract. It did not use a new entity index.

Both genesis preparations printed the same address:

```text
bcrt1pkxp9f8n0eh0ld2xtnqh4flm433f8ckv4g44yjsef6c9rtsmlfapsm5w6ls
```

That match is in `raw/f1-prepare.txt` and `raw/f5-genesis-prepare.txt` of the 2026-09-26 bundle. Bitcoin Core `deriveaddresses` returned the same address for that descriptor (`raw/f1-core-crosscheck.txt`, `raw/f5-genesis-core-crosscheck.txt`).

The two contracts are:

| Bundle label | Contract |
| --- | --- |
| Identity 1 | `contract:mPgNX4HK-bAnSf6V-iOyTKZE-JcV8Eyc-KyA8ZsV-fBUL5ws` |
| Identity 2 | `contract:nKjqQ5l_-3qOUD_J-vlzHvQ0-0eCNyZq-Lf0xdrX-9G4K3m8` |

They are two contracts under one root. They are not two independent entities.

The 2026-09-26 `RUN.md` does not say this.

This note does not rerun the fixtures and does not create a second entity index. A later lineage can do that. This file records the correction only.
