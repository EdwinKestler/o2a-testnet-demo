# Block 0 stage ceremony

This is the rehearsal runbook for the first O2A identity minted live with an artist.

The identity in this rehearsal is on the public Bitcoin signet. It is disposable. The eventual real mint is on mainnet, and this document does not do that mint. It contains no mainnet key, address, or transaction.

The show follows proposed ADR-0008. The EntityID is:

```text
TaggedHash("O2A/v0.1/entity-id", genesis payload)
```

The genesis payload sets `signer_entity` to 32 zero bytes. The root still signs the genesis. A verifier recomputes the EntityID from that payload.

The EntityID exists when the genesis is signed. It is final for the show when the seal's funding transaction is already confirmed at depth. The ceremony funds and confirms that output before the artist walks on stage. Signing then only adds the artist's signature. No controller rotation, recovery, or seal spend happens on this identity until the RGB stack is final. The stage flow is genesis plus one `official_name` claim.

ADR-0008 is proposed. It is the ceremony rule for this rehearsal. It is not an accepted change to the specification in this commit. The demo binary on this branch still derives its test identities from entity index 0 and does not yet display this EntityID. P3b lands that display. Section "Rehearsal checklist" is what P3b-2 runs, in this order.

## People

| Role | What they do | What they hold |
| --- | --- | --- |
| Operator | Runs the signet node, the funding wallet, and the projector machine | The show's signet spending wallet. No recovery key. |
| Artist | Creates the seed, chooses the recovery set, signs the genesis and the name | The seed, the root, and the controller key, on one offline device |
| Independent verifier | Checks the identity on a second machine and reads the result aloud | A copy of the public package after signing. No seed. |
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

An `official_name` claim says this controller signed that spelling. Another person can sign the same spelling on a different EntityID. The EntityID is the identity. The name is the artist's claim.

## Machines

Use three machines.

1. **Artist device.** Offline after the seed exists. Screen faces the artist. It never joins the projector cable and never shares its whole disk.
2. **Operator laptop.** Talks to the O2A signet node. Pays for the seal. Drives the projector.
3. **Verifier laptop.** Independent person. It gets the public package on a USB stick after signing. It has its own view of the O2A signet node.

The backup operator may use a fourth machine for the restore test after the show.

The projector is plugged into the operator laptop. It shows the local page from `/tmp/block0-screen/index.html` and, beside it, the two verifier results once they are copied onto that page. The page is `docs/block0-display.html` filled in by `docs/block0-screen.py`. Nothing on that page is published to `site/`. The folder `/tmp/block0-screen/` is outside the git repository.

## Keys

Identity keys are derived under the O2A branch of the seed. In the paths below, `m` is that O2A root, and signet uses coin `1'`.

| Role | Path | Used for |
| --- | --- | --- |
| 0 root | `m/1'/entity'/0'/0'` | Signs the genesis |
| 1 controller | `m/1'/entity'/1'/0'` | Signs the `official_name` claim |
| 2 recovery | `m/1'/entity'/2'/0'`, then `1'`, then `2'` | The recovery set the artist chose |
| 4 seal | `m/1'/entity'/4'/0'` | The seal script for this genesis |

`entity'` is a new index for this artist. It is written down in the public plan and is not reused.

Role 3 (Nostr) is not used in this ceremony.

The operator's payment wallet is a separate signet wallet. Its keys do not sign the genesis or the claim.

## Recovery proposal

Offer this plan, then let the artist change it.

| Choice | What it means |
| --- | --- |
| 2 of 3 | Any two of the three recovery keys can recover after the delay. |
| Who holds them | The artist holds two shares, in different physical places (this device, and a paper share or second device kept somewhere else). One person the artist names holds the third. |
| Delay | The default is 1008 blocks. The artist can choose another number. Write the chosen number into the public plan. |
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

The projector, the verifier laptop, and any photo of the room stay on the public lines. The seed stays on the artist device.

### Policy and address

1. Build the seal policy from the artist's public keys, the threshold, and the delay.
2. Compute the signet seal address from that policy.
3. Read the address back from Bitcoin Core `getdescriptorinfo` and `deriveaddresses` on the O2A signet node. The two strings match before anyone sends money.
4. Write the address, the entity index, and the public keys into the public plan.

### Fund the seal

Send the seal coins in a transaction that cannot be replaced.

The EntityID commits to the seal outpoint. A replace-by-fee transaction spends the same inputs and creates a new transaction id. The outpoint the artist is about to sign would no longer exist, and the EntityID on the page would not match the coins. Child-pays-for-parent adds a fee without changing that outpoint. Use that if the fee needs help later.

On the show's signet wallet, when the maintainer has created one:

```text
bitcoin-cli -signet -rpcwallet=SHOW -named sendtoaddress address="SEAL_ADDRESS" amount=AMOUNT replaceable=false
```

`replaceable=false` asks Core not to signal replacement. Named arguments keep that flag on the replacement setting.

Then check the transaction:

```text
bitcoin-cli -signet getrawtransaction TXID 1
```

Every input `sequence` is `4294967294` or `4294967295`. A smaller sequence is a replacement signal. If you see one, stop, leave the transaction unspent, and fund a new seal output that does not signal replacement. Do not run `bumpfee` on the seal transaction.

Record `txid`, output index, amount, and the block hash once it confirms.

### Wait for depth

The signet demonstration policy uses depth 1. This rehearsal waits for **6 confirmations**, because mainnet uses 6 and the stage is a rehearsal of that wait.

```text
bitcoin-cli -signet getrawtransaction TXID 1
```

Read `confirmations`. Continue when the number is at least 6.

Signet blocks are uneven. Plan on about an hour, and start the wait inside the T-48h window so the show is not the thing that waits.

A verifier set to signet depth 1 may say CURRENT after one confirmation. The stage clock still waits for 6. The operator does not put the EntityID on the projector as final before that.

### Pre-flight, by T-2h

Each line is a yes before the audience comes in.

- The O2A signet node and its electrs agree on a height, and that height is the public signet tip or the operator can explain the gap.
- The seal transaction has at least 6 confirmations.
- No seal input signals replacement.
- Core `deriveaddresses` matches the seal address in the public plan.
- The artist device is charged, offline, and opens on the public-key screen.
- The seed is not on the operator laptop, the verifier laptop, or the projector.
- Both verifiers have been synced to the O2A node before doors.
- The projector test page used a 64-character test id, labeled as a test, and that page was cleared.
- USB sticks for the artist and the backup operator are empty and labeled.
- The operator has a sentence ready if a step fails: "We will finish the signature on this device and show the check as soon as this screen is honest."

## On stage

Target: under 5 minutes. The funding wait is already finished.

| Clock | Who | Action |
| --- | --- | --- |
| 0:00 | Artist | On the offline device, confirm the outpoint and the public policy. Sign the genesis. |
| 0:45 | Operator | Read the EntityID from the signed genesis. It is the tagged hash of the payload, with `signer_entity` all zeros. |
| 1:00 | Operator | Build the projector page. |
| 1:30 | Verifier | On the verifier laptop, check the package. Say the state aloud. |
| 2:30 | Second check | The independent verifier repeats that check on their own machine. |
| 3:15 | Artist | Sign the `official_name` claim with the controller key. |
| 4:00 | Verifier | Show the claim on the verifier laptop. The operator copies that public line to the projector. |
| 4:30 | Operator | Stop. Take no more signatures. |

Projector command, run on the operator laptop, with the EntityID hex and the name the artist wants on the wall:

```text
python3 docs/block0-screen.py init ENTITYID "Artist Name"
python3 docs/block0-screen.py verifier A CURRENT HEIGHT
python3 docs/block0-screen.py verifier B CURRENT HEIGHT
python3 docs/block0-screen.py claim "Artist Name"
```

Open `file:///tmp/block0-screen/index.html` in a browser window with no other tabs. Refresh after each command.

The USB stick handed to each verifier contains the signed genesis, the consignment, and the public plan. It does not contain the seed.

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

The artist device is the only screen that ever shows the seed. These stay off the projector, the verifier laptop, and any photo of the stage:

- Seed words and the passphrase
- Extended private keys and private key files
- Recovery seeds and recovery private keys
- The show wallet's balance and its other addresses
- RPC passwords
- A PSBT or a descriptor that carries private keys

A photograph of the projector then shows the EntityID, the name, and the word CURRENT. That is public on purpose.

## After the show

Make two copies of the genesis package and the consignment.

The package is:

- The signed genesis
- The public plan: entity index, public keys, threshold, delay, seal address, txid, output index
- The consignment file
- The EntityID hex

Copy 1 stays with the artist, on a USB stick they take home. Copy 2 stays with the backup operator, in a place agreed before the show.

On a second machine that does not have the artist seed:

1. Copy the package into an empty folder.
2. Run the verifier against the O2A signet node.
3. The EntityID matches the projector.
4. The state is CURRENT.
5. The name claim verifies under the same EntityID.

Write down that the restore test passed, with the height it used. If it fails, keep both copies and tell the artist before anyone leaves.

Do not spend the seal. Do not rotate the controller. Those wait until the RGB stack is final.

## If something fails

| What happened | What to do | What to say |
| --- | --- | --- |
| Venue network is down | Sign on the artist device anyway. Hand the verifiers the USB. They verify when they can see the O2A node. Leave the projector on the EntityID and the words "check follows". | "The signature is on this device. The public check will show as soon as we can see our node." |
| Artist device fails before signing | Stop. The identity has not been signed. Use the backup device only if the artist already put the seed there themselves. | "We will not invent a second identity on this stage." |
| Artist device fails after signing | Read the EntityID from the signed file if it is on the USB. If the file never left the broken device, stop and recover that file before showing an id. | "We show the id from the signed file, or we wait." |
| Fee is too low and the seal has no confirmation | Add a child-pays-for-parent from the show wallet. Do not replace the seal transaction. The stage waits. | "The payment is the same one. We are waiting for it to confirm." |
| Someone starts a replacement transaction | Treat that outpoint as unused. Fund a new output that does not signal replacement. The unsigned plan is discarded. | "That payment was replaced, so it is not this identity. We will use a new one." |
| Operator shows the wrong window | Switch to the `file://` page. If the seed was visible, the artist treats that seed as public and starts a new seed ceremony later. Do not continue with a seed that was on the projector. | "That screen was wrong. We stop." |
| A camera is pointed at the artist device | Turn that screen away. The projector may stay on the public page. | "The wall is the public screen." |
| Verifiers disagree | Leave both results on the page. Do not hide the one that is not CURRENT. | "The two checks do not match yet. We will not call this final." |
| The seal was funded but has fewer than 6 confirmations at door time | Keep the audience plan. Sign only if the artist and the operator agree to show the id as waiting. The page does not say CURRENT. | "The identity is signed. We still wait for confirmations before we call it final." |

## Rehearsal checklist

P3b-2 runs these steps in this order, on signet, with a disposable identity. Each step ends with the line in the "Done when" column.

| Step | Done when |
| --- | --- |
| 1. Confirm the O2A signet node is running, electrs is at the same height, and the toolchain can reach that electrs. | Heights match, and a toolchain TCP check to electrs succeeds. |
| 2. Create a new entity index for this rehearsal. Derive roles 0, 1, 2, and 4. | The public plan lists x-only keys and the entity index. The seed is on the artist device only. |
| 3. Choose the recovery set with the artist. Default offer: 2 of 3, the artist's two shares in different physical places, staff hold zero, delay 1008 blocks unless the artist changes it. | The public plan has threshold, delay, and recovery key ids. |
| 4. Compute the seal address and match it with Core `getdescriptorinfo` and `deriveaddresses`. | The two address strings are identical. |
| 5. Fund that address with `-named sendtoaddress` and `replaceable=false`. | Every input sequence is `4294967294` or `4294967295`. |
| 6. Wait until `confirmations` is at least 6. | The raw transaction shows 6 or more. |
| 7. Run the pre-flight list above. | Every line is yes. |
| 8. Sign the genesis offline. `signer_entity` is 32 zero bytes. | A signed genesis file exists on the artist device. |
| 9. Compute the EntityID as the tagged hash of that payload. | 64 hex characters. |
| 10. Run `python3 docs/block0-screen.py init ENTITYID "Rehearsal Name"`. | `/tmp/block0-screen/index.html` shows that id and a QR code. The browser has no other tabs. |
| 11. Give each verifier a USB with the genesis, the consignment, and the public plan. | Two machines, two people. |
| 12. Each verifier reports the state. | Both say CURRENT. Run the `verifier` command for A and for B with the height they used. |
| 13. The artist signs one `official_name` claim with the controller key. | The claim file names this EntityID and the predicate `official_name`. |
| 14. One verifier checks the claim. Run `python3 docs/block0-screen.py claim "Rehearsal Name"`. | The page says "Name claim signed:" and the name. |
| 15. Copy the package to the artist USB and to the backup operator. On a second machine, verify again. | The same EntityID and CURRENT. |
| 16. Leave the seal unspent. | No rotation, recovery, or close follows. |

Step 1 was not true on 2026-09-28: the signet containers were stopped, and the demo toolchain could not reach signet electrs. P3b-2 starts at step 1 again rather than skipping it.
