# RGB identity program for the disposable demo

**Status:** demo-only program for RGB 0.12 RC3. These program bytes and IDs do
not settle dependency-gate item 2, do not enter the normative O2A
specification, and may change or disappear with the demo lineage.

## Boundary

The demo keeps three verification layers separate:

1. Bitcoin proves best-chain order, confirmation depth, and the spend of the
   outpoint holding the identity seal.
2. This RGB program validates the supplied seal-linked contract history
   against its Bitcoin anchors.
3. `o2a-demo-core` validates the O2A-CANON-1 object, its BIP340 signature, the
   exact prior authorizing state, key role, and capability.

The RC3 Codex uses a success verifier for its declared methods. It therefore
does not replace O2A authorization or shape validation. The adapter constructs
only the shapes below and the independent validator reports the RGB result and
the O2A result separately.

## State

Immutable genesis state:

- `root`: the 32-byte root identity x-only public key;
- `entityId`: the 32-byte O2A EntityID derived from that root and network.

Append-only global state with a deterministic current-state projection:

- `controller`: the current 32-byte controller x-only public key; its latest
  dependency-ordered value is current;
- `policyHash`: the 32-byte recovery-policy commitment, unchanged in this
  pass; and
- `profileVersion`: demo-stable derivation profile version `1`.

Owned state:

- `identity`: one non-fungible state cell assigned to the current Bitcoin
  outpoint. Its value is the 32-byte canonical O2A resulting-state commitment.
  Spending that outpoint closes the identity seal.

The single-controller cardinality is a demo restriction, not a normative O2A
rule.

## Supported operations

### Genesis

Genesis uses the `O2A/v0.1/entity-genesis` canonical payload, an absent prior
state, sequence zero, ACTIVE status, the initial controller and policy, and a
root-key BIP340 signature. The RGB genesis assigns `identity` to an already
funded regtest outpoint. The funding transaction is the genesis seal source;
it is not mislabeled as a transition anchor.

### Controller rotation

`rotateController` consumes the prior `identity` cell and assigns the same
non-fungible identity to a new seal. It appends the replacement controller and
the canonical resulting-state commitment. O2A verification separately
requires operation `1`, sequence increment by one, the exact previous state
and seal, controller role, identity-transition capability, and a valid
`O2A/v0.1/identity-transition` BIP340 signature.

The Bitcoin transaction spending the prior seal carries the RGB commitment
for this transition and is the rotation anchor.

### Revocation

`revoke` is declared for the demo program but is not implemented in this
pass. Its eventual transition must consume the current cell, produce a
canonical REVOKED state, and prevent later transitions. It must not be treated
as an unrecorded burn.

## Network and lifecycle

Regtest is the execution network for this evidence. The default public Bitcoin
signet is the later live target. Every resulting identity is disposable. The
derivation profile is demo-stable v0.1, not frozen.
