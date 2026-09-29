# Block 0 stage ceremony

This is the operator runbook for the first O2A EntityID minted live with an artist.

The rehearsal network is the public Bitcoin signet. A rehearsal identity is disposable. The network for a later public event waits for the maintainer's explicit authorization. This document does not mint on mainnet. It contains no mainnet key, address, or transaction.

The show follows ADR-0008 for the EntityID, ADR-0009 for the state id and the `official_name` claim, and ADR-0010 for rgb-protocol 0.11.1 with Opret. The adapter is [rgb-0.11.1-adapter.md](rgb-0.11.1-adapter.md).

The EntityID is:

```text
TaggedHash("O2A/v0.1/entity-id", genesis payload)
```

The genesis payload sets `signer_entity` to 32 zero bytes. The root still signs the genesis. A verifier recomputes the EntityID from that payload.

The EntityID exists when the genesis is signed. It is final for the show when the seal's funding transaction is already confirmed at depth. The ceremony funds and confirms that output before the artist walks on stage. Signing then only adds the artist's signature. No controller rotation, recovery, or seal spend happens on this identity until the RGB program is final. The stage flow is genesis plus one `official_name` claim. A claim spends no Bitcoin.

Section "Rehearsal checklist" is the order for a signet dress rehearsal. Rehearsal 3, on 2026-09-28, is the current signet record. Its bundle is `evidence/signet-block0-rehearsal-3-2026-09-28`. Do not spend its seal `513f69c7d07b1bf6b664f2ca6e723938a73fd8ea8052d8b1ccc07a2ecdbfcede:0`. A new artist gets a new entity index and a new seal.

## Tooling

Every O2A command in this runbook is the 0.11.1 binary built by rehearsal 3:

```text
cargo build --manifest-path spikes/rgb-0.11.1/Cargo.toml --locked --bin rgb011-check
```

The binary is `spikes/rgb-0.11.1/target/debug/rgb011-check`. The commands used here are `plan`, `signet-genesis`, `signet-claim`, and `signet-verify`.

The signet node command used by rehearsal 3 is Bitcoin Core inside container `signet-infra-bitcoind-1`. The base compose file leaves the wallet disabled. The rehearsal wallet comes from `dev/signet/compose.wallet.yaml` on project `signet-infra`. Set the context on the command. Do not leave the user's Docker context switched. The `desktop-linux` context does not see this container.

```text
DOCKER_CONTEXT=default docker exec signet-infra-bitcoind-1 bitcoin-cli -signet -datadir=/data
```

Core is published on `127.0.0.1:38332`. Electrum for this stack is `127.0.0.1:60601`. Copy the cookie from the container path `/data/signet/.cookie` to a file outside the repository, and point `BITCOIN_COOKIE` at that copy. Do not print the cookie. Do not commit it. Do not edit `dev/bitcoin.conf` or `dev/compose.yaml`.

`o2a-demo-core` defaults the recovery delay to 10 blocks and the entity index to 0. The ceremony exports `O2A_DEMO_DELAY`, `O2A_DEMO_THRESHOLD`, and `O2A_DEMO_ENTITY` for both `plan` and `signet-genesis`. The seed file stays outside the repository. The binary refuses `O2A_DEMO_NETWORK=mainnet` and `RGB_CHAIN=mainnet`.

The projector stays on the operator laptop:

```text
python3 docs/block0-screen.py init ENTITYID "Rehearsal Name"
```

That page is written to `/tmp/block0-screen/`, not to `site/`.

## People

| Role | What they do | What they hold |
| --- | --- | --- |
| Operator | Runs the signet node, the funding wallet, and the projector machine | The show's signet spending wallet. No recovery key. |
| Artist | Creates the seed, chooses the recovery set, signs the genesis and the name | The seed, the root, and the controller key, on one offline device |
| Verifier A | Checks the identity on their own laptop and reads the result aloud | A copy of the public package after signing. No seed. |
| Verifier B | Repeats that check on a second laptop | A copy of the public package after signing. No seed. |
| Backup operator, optional | Keeps the second backup and can refresh the projector | A copy of the public package. No recovery key. |

The artist decides the recovery set. O2A staff do not hold recovery keys unless the artist asks for that in writing, and the default offer is that they do not.

A recovery key can, after the delay, authorize a new controller for this identity. If staff hold enough recovery keys to meet the threshold, staff can take the identity without the artist. The artist is the custodian. Staff can help the artist write down the public plan. Staff do not keep a share of the seed or a recovery key.

## Words used on the night

- **Seed.** The secret words on the artist's device. They stay on that device.
- **Root.** The key that signs the genesis. It comes from the seed. After genesis it does not run the identity.
- **Controller.** The key that signs the name claim.
- **Seal.** The already-confirmed signet output that the genesis names. Its outpoint is part of the EntityID.
- **EntityID.** The 64-character hex id of this genesis. The projector shows this and a QR code of these characters.
- **CURRENT.** The word a verifier shows when Bitcoin, RGB, and O2A all agree, at the depth the ceremony is using.

Bitcoin says the seal payment is in the chain at the required depth. RGB says the consignment matches that payment. O2A says the signature is the artist's key. The projector shows CURRENT only after both verifiers say it.

An `official_name` claim says this controller signed that spelling. Another person can sign the same spelling on a different EntityID. The EntityID is the identity. The name is the artist's claim. The claim spends no Bitcoin.

## Machines

The stage uses separate laptops for the two verifiers.

1. **Artist device.** Offline after the seed exists. Screen faces the artist. It never joins the projector cable and never shares its whole disk.
2. **Operator laptop.** Talks to the O2A signet node. Pays for the seal. Drives the projector.
3. **Verifier laptop A.** Independent person. It gets the public package on a USB stick after signing.
4. **Verifier laptop B.** A second person, on a second laptop. It gets the same public package.

The two verifiers run on two separate laptops. Ideally those laptops use different Bitcoin backends. One may use the operator's electrs. The other may use a second electrs or another signet source the operator has checked against the same tip before doors.

The backup operator may use another machine for the restore test after the show.

Rehearsal 3 did not meet this stage rule. Verifier A, verifier B, and the restore were three directories on one host, one electrs, and one node. Both reports were CURRENT and byte-identical. That was a dress rehearsal of the check. The stage uses two laptops.

The projector is plugged into the operator laptop. It shows the local page from `/tmp/block0-screen/index.html` and, beside it, the two verifier results once they are copied onto that page. The page is `docs/block0-display.html` filled in by `docs/block0-screen.py`. Nothing on that page is published to `site/`. The folder `/tmp/block0-screen/` is outside the git repository.

## Keys

Identity keys are derived under the O2A branch of the seed. In the paths below, `m` is that O2A root, and signet uses coin `1'`.

| Role | Path | Used for |
| --- | --- | --- |
| 0 root | `m/1'/entity'/0'/0'` | Signs the genesis |
| 1 controller | `m/1'/entity'/1'/0'` | Signs the `official_name` claim |
| 2 recovery | `m/1'/entity'/2'/0'`, then `1'`, then `2'` | The recovery set the artist chose |
| 4 seal | `m/1'/entity'/4'/0'` | The seal script for this genesis |

`entity'` is a new index for this artist. It is written down in the public plan and is not reused. Rehearsal 3 used index 31. Leave that index with that rehearsal.

Role 3 (Nostr) is not used in this ceremony.

The operator's payment wallet is a separate signet wallet. Its keys do not sign the genesis or the claim. Rehearsal 3 used the wallet name `rehearsal`.

## Recovery proposal

Offer this plan, then let the artist change it.

| Choice | What it means |
| --- | --- |
| 2 of 3 | Any two of the three recovery keys can recover after the delay. |
| Who holds them | The artist holds two shares, in different physical places (this device, and a paper share or second device kept somewhere else). One person the artist names holds the third. |
| Delay | The default offer is 1008 blocks. The artist can choose another number. Write the chosen number into the public plan and into `O2A_DEMO_DELAY`. |
| Staff | Hold none of the three. |

Trade-offs to say out loud, once:

- One recovery key is simple, and a single loss or theft decides recovery.
- Two of three survives one lost share, and any two holders can act after the delay.
- A short delay makes a real recovery fast, and it also makes a bad recovery fast.
- A long delay gives the artist time to notice, and a real recovery waits.

Stop after the artist chooses. Record only public key ids, the threshold, and the delay.

## T-48h to T-2h

### Seed ceremony

Do this with the artist device offline and the projector turned off or showing a blank local page.

1. The artist creates the seed on their device, or opens a seed they already created.
2. The artist writes the seed onto paper they will keep, if they want a paper copy. They do that facing a wall, with no camera on that screen.
3. The operator does not ask the artist to read the seed aloud.
4. The operator does not type the seed.
5. The device then shows only public information: entity index, x-only public keys for roles 0, 1, 2, and 4, and the seal address.

The projector, the verifier laptops, and any photo of the room stay on the public lines. The seed stays on the artist device.

### Policy and address

`rgb011-check plan` prints the address, the descriptor, the delay, the threshold, and the public keys. Set the same environment that genesis will use. The binary is `spikes/rgb-0.11.1/target/debug/rgb011-check`.

```text
O2A_DEMO_SEED_FILE=PATH_OUTSIDE_THE_REPO \
O2A_DEMO_NETWORK=signet \
O2A_DEMO_DELAY=1008 \
O2A_DEMO_THRESHOLD=2 \
O2A_DEMO_ENTITY=NEW_INDEX \
RGB_CHAIN=signet \
spikes/rgb-0.11.1/target/debug/rgb011-check plan
```

Then read that address back from Bitcoin Core. The two strings match before anyone sends money.

```text
DOCKER_CONTEXT=default docker exec signet-infra-bitcoind-1 \
  bitcoin-cli -signet -datadir=/data getdescriptorinfo "DESCRIPTOR_FROM_PLAN"
DOCKER_CONTEXT=default docker exec signet-infra-bitcoind-1 \
  bitcoin-cli -signet -datadir=/data deriveaddresses "DESCRIPTOR_FROM_PLAN"
```

Write the address, the entity index, and the public keys into the public plan. In rehearsal 3, delay 1007 produced the unfunded address `tb1pxlm4rc3nl8pd8ervyc273tyjzrhflczf0kn59rht5ncq4un53t6qj9l23t`. The funded address, at delay 1008, was `tb1pcvdv0m82jkj70xxlut5a7yqzr4p7lmqtm3x2py338h892r2th02scy38ay`, and Core `deriveaddresses` matched it.

### Fund the seal

Send the seal coins in a transaction that cannot be replaced.

The EntityID commits to the seal outpoint. A replace-by-fee transaction spends the same inputs and creates a new transaction id. The outpoint the artist is about to sign would no longer exist, and the EntityID on the page would not match the coins. Child-pays-for-parent adds a fee without changing that outpoint. Use that if the fee needs help later.

Rehearsal 3 paid 10,000 sats from wallet `rehearsal` with `replaceable=false`. A later show uses the wallet the maintainer assigns, and a new address. The command form is the one rehearsal 3 ran:

```text
DOCKER_CONTEXT=default docker exec signet-infra-bitcoind-1 \
  bitcoin-cli -signet -datadir=/data -rpcwallet=rehearsal \
  -named sendtoaddress address="SEAL_ADDRESS" amount=0.00010000 replaceable=false
```

`replaceable=false` asks Core not to signal replacement. Named arguments keep that flag on the replacement setting. Not signaling replacement is operator discipline. Since Bitcoin Core 28, a node may accept a replacement even when the original transaction did not signal it. The protection for the EntityID is to wait for 6 confirmations before the artist signs the genesis, and to never fee-bump that funding transaction. If the fee needs help before those 6 confirmations, use child-pays-for-parent, which keeps the same outpoint.

If the funding transaction is built with `createrawtransaction`, the replaceable argument defaults to true. `fundrawtransaction` with `"replaceable": false` does not rewrite a sequence that is already on an input. Set the input sequence to `4294967294`, pass locktime `0`, and pass `false` as the fourth `createrawtransaction` argument. `rgb011-check` does that in `fund_output` when it funds a regtest output that must keep its txid.

Then check the transaction:

```text
DOCKER_CONTEXT=default docker exec signet-infra-bitcoind-1 \
  bitcoin-cli -signet -datadir=/data getrawtransaction TXID 1
```

Every input `sequence` is `4294967294` or `4294967295`. A smaller sequence is a replacement signal. If you see one, stop, leave the transaction unspent, and fund a new seal output that does not signal replacement. Do not run `bumpfee` on the seal transaction.

`rgb011-check signet-genesis` repeats this check. When a sequence is smaller it prints `funding sequences […] are replaceable; genesis stays unsigned` and does not sign.

Record `txid`, output index, amount, and the block hash once it confirms. Rehearsal 3 recorded sequence `4294967294`, output index 0, and anchor block 324124, hash `00000011ebb467178dfac1055a719a0013e14bf01b9697c86b2d3ad74001addd`.

### Wait for depth

`rgb011-check signet-verify` uses depth 2, the `DEPTH` constant in `spikes/rgb-0.11.1/check/src/lineage.rs`. `signet-genesis` still refuses to sign before 6 confirmations. The stage clock uses 6. The live-event network is still a maintainer decision, and the stage still waits for 6.

```text
DOCKER_CONTEXT=default docker exec signet-infra-bitcoind-1 \
  bitcoin-cli -signet -datadir=/data getrawtransaction TXID 1
```

Read `confirmations`. Continue when the number is at least 6. Below 6, `rgb011-check signet-genesis` prints `depth N; genesis stays unsigned until 6 confirmations`.

Signet blocks are uneven. Rehearsal 3 went from the first zero-confirmation reading at 21:32:31 to 6 confirmations at 22:11:06, which is 38 minutes 35 seconds. Plan on at least that long, and start the wait inside the T-48h window so the show is not the thing that waits. The seal is funded at least 48 hours ahead.

A `signet-verify` run can print CURRENT once the seal has 2 confirmations. The stage clock still waits for 6, and `signet-genesis` will not sign before that. The operator does not put the EntityID on the projector as final before 6.

### Pre-flight, by T-2h

Each line is a yes before the audience comes in.

- The O2A signet node and its electrs agree on a height, and that height is the public signet tip or the operator can explain the gap. Rehearsal 3 saw the local tip, electrs, and blockstream.info all at 324129 when genesis was signed.
- The seal transaction has at least 6 confirmations.
- No seal input signals replacement. Every sequence is `4294967294` or `4294967295`.
- Core `deriveaddresses` matches the seal address in the public plan.
- `O2A_DEMO_DELAY` matches the delay that was funded. A mismatch is a different address.
- The artist device is charged, offline, and opens on the public-key screen.
- The seed is not on the operator laptop, either verifier laptop, or the projector.
- Both verifier laptops have been synced before doors, ideally on different Bitcoin backends.
- The projector test page used a 64-character test id, labeled as a test, and that page was cleared.
- USB sticks for the artist and the backup operator are empty and labeled.
- The operator has a sentence ready if a step fails: "We will finish the signature on this device and show the check as soon as this screen is honest."

`scripts/status.py` needs `dev/signet/.env`. That file was absent during rehearsal 3, so the script could not open it. The height check above is the one this runbook requires.

## On stage

The funding wait is already finished. In rehearsal 3 the signing script reached restore at 1.477 seconds and finished at 1.644 seconds (`evidence/signet-block0-rehearsal-3-2026-09-28/raw/timings.txt`). That is under 2 seconds from the start of the signing script through the restore. The stage still leaves room for two people to read the result aloud.

| Clock | Who | Action |
| --- | --- | --- |
| 0:00 | Artist | On the offline device, confirm the outpoint and the public policy. Sign the genesis with `rgb011-check signet-genesis`. |
| 0:45 | Operator | Read the EntityID from the signed genesis. It is the tagged hash of the payload, with `signer_entity` all zeros. |
| 1:00 | Operator | Build the projector page with `docs/block0-screen.py`. |
| 1:30 | Verifier A | On laptop A, run `rgb011-check signet-verify` with the seed unset. Say the state aloud. |
| 2:30 | Verifier B | On laptop B, run the same check. Ideally this laptop uses a different Bitcoin backend. |
| 3:15 | Artist | Sign the `official_name` claim with `rgb011-check signet-claim`. |
| 4:00 | Verifier A | Show the claim on laptop A. The operator copies that public line to the projector. |
| 4:30 | Operator | Stop. Take no more signatures. |

Genesis, with the seed file set and the evidence directory outside any published site:

```text
O2A_DEMO_SEED_FILE=PATH_OUTSIDE_THE_REPO \
O2A_DEMO_NETWORK=signet \
O2A_DEMO_DELAY=1008 \
O2A_DEMO_THRESHOLD=2 \
O2A_DEMO_ENTITY=NEW_INDEX \
RGB_CHAIN=signet \
BITCOIN_RPC=http://127.0.0.1:38332 \
BITCOIN_COOKIE=PATH_TO_COPIED_COOKIE \
BITCOIN_WALLET=rehearsal \
ELECTRUM=127.0.0.1:60601 \
SEAL_OUTPOINT=TXID:VOUT \
RGB011_EVIDENCE=EVIDENCE_DIR \
spikes/rgb-0.11.1/target/debug/rgb011-check signet-genesis
```

The claim uses the same evidence directory. It writes `claim.o2a`. It spends no Bitcoin output.

```text
O2A_DEMO_SEED_FILE=PATH_OUTSIDE_THE_REPO \
O2A_DEMO_NETWORK=signet \
O2A_REHEARSAL_NAME="Rehearsal Name" \
RGB011_EVIDENCE=EVIDENCE_DIR \
spikes/rgb-0.11.1/target/debug/rgb011-check signet-claim
```

Each verifier gets a directory with `genesis.o2a`, `genesis.strict`, `public.txt`, and, after the claim, `claim.o2a`. The seed variables stay unset:

```text
env -u O2A_DEMO_SEED_FILE -u O2A_DEMO_ENTITY \
RGB_CHAIN=signet \
BITCOIN_RPC=http://127.0.0.1:38332 \
BITCOIN_COOKIE=PATH_TO_COPIED_COOKIE \
ELECTRUM=127.0.0.1:60601 \
RGB011_EVIDENCE=VERIFIER_DIR \
spikes/rgb-0.11.1/target/debug/rgb011-check signet-verify
```

Projector commands, on the operator laptop. The name is the public display name for that rehearsal. Rehearsal 3 used `Rehearsal Name`. Do not put a person's legal name on this page unless the artist has asked for that spelling in the public plan.

```text
python3 docs/block0-screen.py init ENTITYID "Rehearsal Name"
python3 docs/block0-screen.py verifier A CURRENT HEIGHT
python3 docs/block0-screen.py verifier B CURRENT HEIGHT
python3 docs/block0-screen.py claim "Rehearsal Name"
```

Open `file:///tmp/block0-screen/index.html` in a browser window with no other tabs. Refresh after each command.

The USB stick handed to each verifier contains the signed genesis, the consignment, the seal outpoint in `public.txt`, and the public plan. It does not contain the seed.

The verifier derives the seal script from the seal policy inside the signed genesis. The seal record only helps locate the outpoint. A script written into the record is not the expected script, even when those policy bytes also appear somewhere else in the genesis. A different recovery delay is a different address. The pre-flight compares that address with Core and does not fund the mismatch.

### What the projector shows

Only these lines:

- The words "O2A identity"
- The artist's display name
- The 64-character EntityID
- A QR code of that EntityID and of nothing else
- "Verifier A" and the state and height
- "Verifier B" and the state and height
- After the claim: "Name claim signed:" and the name

The browser address bar shows a `file://` path under `/tmp/block0-screen/`.

### What stays off every shared screen

The artist device is the only screen that ever shows the seed. These stay off the projector, the verifier laptops, and any photo of the stage:

- Seed words and the passphrase
- Extended private keys and private key files
- Recovery seeds and recovery private keys
- The show wallet's balance and its other addresses
- RPC passwords and the cookie
- A PSBT or a descriptor that carries private keys

A photograph of the projector then shows the EntityID, the name, and the word CURRENT. That is public on purpose.

## After the show

Make two copies of the genesis package and the consignment.

The package is:

- The signed genesis
- The public plan: entity index, public keys, threshold, delay, seal address, txid, output index
- The consignment file `genesis.strict`
- The `official_name` claim
- The EntityID hex

Copy 1 stays with the artist, on a USB stick they take home. Copy 2 stays with the backup operator, in a place agreed before the show.

On a machine that does not have the artist seed, run the same `rgb011-check signet-verify` command as the verifiers, with `O2A_DEMO_SEED_FILE` and `O2A_DEMO_ENTITY` unset:

1. Copy the package into an empty folder.
2. Run `spikes/rgb-0.11.1/target/debug/rgb011-check signet-verify` against the signet node.
3. The EntityID matches the projector.
4. The state is CURRENT.
5. The name claim prints `name_claim=valid`.

Write down that the restore test passed, with the height it used. If it fails, keep both copies and tell the artist before anyone leaves.

Do not spend the seal. Do not rotate the controller. Those wait until the RGB program is final.

## If something fails

| What happened | What to do | What to say |
| --- | --- | --- |
| Venue network is down | Sign on the artist device anyway. Hand the verifiers the USB. They verify when they can see a signet backend. Leave the projector on the EntityID and the words "check follows". | "The signature is on this device. The public check will show as soon as we can see the chain." |
| Artist device fails before signing | Stop. The identity has not been signed. Use the backup device only if the artist already put the seed there themselves. | "We will not invent a second identity on this stage." |
| Artist device fails after signing | Read the EntityID from the signed file if it is on the USB. If the file never left the broken device, stop and recover that file before showing an id. | "We show the id from the signed file, or we wait." |
| Fee is too low and the seal has no confirmation | Add a child-pays-for-parent from the show wallet. Do not replace the seal transaction. The stage waits. | "The payment is the same one. We are waiting for it to confirm." |
| Someone starts a replacement transaction | Treat that outpoint as unused. Fund a new output that does not signal replacement. The unsigned plan is discarded. | "That payment was replaced, so it is not this identity. We will use a new one." |
| `signet-genesis` prints `funding sequences` and `genesis stays unsigned` | Leave that output unspent. Fund a new seal whose inputs are `4294967294` or `4294967295`. | "That payment can still be replaced. We will use a new one." |
| Operator shows the wrong window | Switch to the `file://` page. If the seed was visible, the artist treats that seed as public and starts a new seed ceremony later. Do not continue with a seed that was on the projector. | "That screen was wrong. We stop." |
| A camera is pointed at the artist device | Turn that screen away. The projector may stay on the public page. | "The wall is the public screen." |
| Verifiers disagree | Leave both results on the page. Do not hide the one that is not CURRENT. | "The two checks do not match yet. We will not call this final." |
| The seal was funded but has fewer than 6 confirmations at door time | Keep the audience plan. Sign only if the artist and the operator agree to show the id as waiting. The page does not say CURRENT. | "The identity is signed. We still wait for confirmations before we call it final." |

## Rehearsal 3

These facts are the 2026-09-28 signet rehearsal on rgb-protocol 0.11.1. They are a record, not a second identity to reuse.

| Field | Value |
| --- | --- |
| Entity index | 31 |
| Network byte | 3 |
| Delay | 1008 |
| Threshold | 2 of 3 |
| EntityID | `bc7c1031ad9daf1a30d2edd1b4dc91079c89ccb8bd8997e6c08fb97570861e37` |
| State id | `45e11a2bd42c0dbca3f06bcf5bec672e92d08dc2615c6172699923ad9feb3e9a` |
| Seal | `513f69c7d07b1bf6b664f2ca6e723938a73fd8ea8052d8b1ccc07a2ecdbfcede:0` |
| Amount | 10,000 sats |
| Sequence | `4294967294` |
| Confirmations at signing | 6 |
| Anchor | block 324124, `00000011ebb467178dfac1055a719a0013e14bf01b9697c86b2d3ad74001addd` |
| Display name | Rehearsal Name |
| Wait for 6 confirmations | 38 minutes 35 seconds, 21:32:31 to 22:11:06 |
| Signing script through restore | under 2 seconds; restore at 1.477 seconds in `raw/timings.txt` |

Both verifier directories reported CURRENT. The restore reported CURRENT with the seed unset and `seal_unspent true`. The claim `Rehearsal Name` verified. Wallet `rehearsal` had trusted balance 0.00103301 signet bitcoin after the payment.

## Rehearsal checklist

Run these steps in this order, on signet, with a disposable identity. Each step ends with the line in the "Done when" column. The O2A binary in every signing step is `spikes/rgb-0.11.1/target/debug/rgb011-check`.

| Step | Done when |
| --- | --- |
| 1. Confirm the O2A signet node is running and electrs is at the same height. Use `bitcoin-cli -signet -datadir=/data` inside `signet-infra-bitcoind-1`, with `DOCKER_CONTEXT=default`. | Heights match, or the gap is written down. |
| 2. Create a new entity index. Run `rgb011-check plan` with `O2A_DEMO_NETWORK=signet`, `O2A_DEMO_DELAY=1008`, `O2A_DEMO_THRESHOLD=2`, and `O2A_DEMO_ENTITY` set. | The public plan lists x-only keys and the entity index. The seed is on the artist device only. |
| 3. Choose the recovery set with the artist. Default offer: 2 of 3, the artist's two shares in different physical places, staff hold zero, delay 1008 blocks unless the artist changes it. | The public plan has threshold, delay, and recovery key ids. The same delay is in the environment. |
| 4. Match the plan address with Core `getdescriptorinfo` and `deriveaddresses`. | The two address strings are identical. |
| 5. Fund that address with `bitcoin-cli -signet -datadir=/data -rpcwallet=rehearsal -named sendtoaddress` and `replaceable=false`. | Every input sequence is `4294967294` or `4294967295`. |
| 6. Wait until `confirmations` is at least 6. | The raw transaction shows 6 or more. Rehearsal 3 needed 38 minutes 35 seconds. The stage funds this at least 48 hours ahead. |
| 7. Run the pre-flight list above, including two verifier laptops. | Every line is yes. |
| 8. Sign the genesis with `rgb011-check signet-genesis`. `signer_entity` is 32 zero bytes. | A signed genesis file exists. The command did not print `genesis stays unsigned`. |
| 9. Read the EntityID the command prints. | 64 hex characters. |
| 10. Run `python3 docs/block0-screen.py init ENTITYID "Rehearsal Name"`. | `/tmp/block0-screen/index.html` shows that id and a QR code. The browser has no other tabs. |
| 11. Give each verifier a USB with the genesis, the consignment, and `public.txt`. | Two laptops, two people. Ideally two Bitcoin backends. |
| 12. Each laptop runs `rgb011-check signet-verify` with the seed unset. | Both say CURRENT. Run the projector `verifier` command for A and for B with the height they used. |
| 13. The artist signs one `official_name` claim with `rgb011-check signet-claim`. | The claim file names this EntityID and the predicate `official_name`. No new Bitcoin transaction. |
| 14. One verifier checks the claim. Run `python3 docs/block0-screen.py claim "Rehearsal Name"`. | The page says "Name claim signed:" and the name. The verifier prints `name_claim=valid`. |
| 15. Copy the package to the artist USB and to the backup operator. On a machine without the seed, run `rgb011-check signet-verify` again. | The same EntityID, CURRENT, and `name_claim=valid`. Rehearsal 3 finished this restore in under 2 seconds. |
| 16. Leave the seal unspent. | No rotation, recovery, or close follows. `gettxout` still returns the output. |

Rehearsal 3 completed step 1 on 2026-09-28. At signing, the local tip, electrs, and blockstream.info were all at height 324129. A later rehearsal starts at step 1 again.
