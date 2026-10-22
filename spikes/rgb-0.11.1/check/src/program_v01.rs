//! Proposed O2A RGB program v0.1 for rgb-protocol 0.11.1 (ADR-0012 candidate).
//!
//! This module is a candidate for normative adoption. It is separate from the
//! disposable demo schema in `schema.rs`, whose recorded evidence must keep
//! reproducing. Two differences matter:
//!
//! * every transition carries the 32-byte O2A anchor digest as global state
//!   `transitionDigest` (type 3102), so the consignment that closes a seal
//!   commits to exactly one O2A transition or recovery payload; and
//! * revocation keeps one owned output because the O2A REVOKED state still
//!   names a `next_seal`. Revocation is terminal in the O2A layer, not by
//!   ending the RGB contract.
//!
//! Validators stay unset. RGB checks consignment shape and seal closure; O2A
//! checks the signed object, authorization, and lifecycle.

use amplify::{bmap, none, tiny_bmap, zero};
use rgbstd::containers::BuilderSeal;
use rgbstd::contract::{AllocatedState, ContractBuilder, TransitionBuilder};
use rgbstd::stl::StandardTypes;
use rgbstd::txout::BlindSeal;
use rgbstd::{
    AssignmentType, ChainNet, GenesisSeal, GlobalStateSchema, GlobalStateType, GraphSeal, Identity,
    Occurrences, Opout, Outpoint, OwnedStateSchema, Schema, Transition, TransitionType, TypeSystem,
};
use serde_json::{json, Value};
use strict_encoding::StrictSerialize;
use strict_types::{LibBuilder, StrictDecode, StrictDumb, StrictEncode, StrictType};

pub const LIB_NAME: &str = "O2AIdentityV01";
pub const GS_GENESIS_DIGEST: GlobalStateType = GlobalStateType::with(3101);
pub const GS_TRANSITION_DIGEST: GlobalStateType = GlobalStateType::with(3102);
pub const OS_IDENTITY: AssignmentType = AssignmentType::with(4101);
/// O2A operations 1, 2, 4, and 5 (`O2A/v0.1/identity-transition`).
pub const TS_TRANSITION: TransitionType = TransitionType::with(8101);
/// O2A operation 3 (`O2A/v0.1/recovery`).
pub const TS_RECOVER: TransitionType = TransitionType::with(8102);

/// Upper bound for the strict-serialized schema dump.
const MAX_SCHEMA_BYTES: usize = 1 << 20;

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, StrictType, StrictEncode, StrictDecode,
)]
#[strict_type(lib = LIB_NAME)]
pub struct O2aDigest(pub [u8; 32]);

impl StrictDumb for O2aDigest {
    fn strict_dumb() -> Self {
        Self([0u8; 32])
    }
}

impl StrictSerialize for O2aDigest {}

pub struct Program {
    pub schema: Schema,
    pub types: TypeSystem,
    pub lib_id: String,
}

fn transition_details(name: &'static str) -> rgbstd::TransitionDetails {
    rgbstd::TransitionDetails {
        transition_schema: rgbstd::TransitionSchema {
            metadata: none!(),
            globals: tiny_bmap! { GS_TRANSITION_DIGEST => Occurrences::Once },
            inputs: tiny_bmap! { OS_IDENTITY => Occurrences::Once },
            assignments: tiny_bmap! { OS_IDENTITY => Occurrences::Once },
            validator: None,
        },
        name: strict_types::fname!(name),
    }
}

pub fn program() -> Program {
    let lib = LibBuilder::with(strict_types::libname!("O2AIdentityV01"), std::iter::empty())
        .transpile::<O2aDigest>()
        .compile()
        .expect("O2aDigest type library");
    let lib_id = lib.id().to_string();
    let standard = StandardTypes::with(lib);
    let sem_id = standard.get("O2AIdentityV01.O2aDigest");
    let schema = Schema {
        ffv: zero!(),
        name: strict_types::tn!("O2aIdentityV01"),
        meta_types: none!(),
        global_types: tiny_bmap! {
            GS_GENESIS_DIGEST => rgbstd::GlobalDetails {
                global_state_schema: GlobalStateSchema::once(sem_id),
                name: strict_types::fname!("genesisDigest"),
            },
            GS_TRANSITION_DIGEST => rgbstd::GlobalDetails {
                global_state_schema: GlobalStateSchema::once(sem_id),
                name: strict_types::fname!("transitionDigest"),
            }
        },
        owned_types: tiny_bmap! {
            OS_IDENTITY => rgbstd::AssignmentDetails {
                owned_state_schema: OwnedStateSchema::Declarative,
                name: strict_types::fname!("identity"),
                default_transition: TS_TRANSITION,
            }
        },
        genesis: rgbstd::GenesisSchema {
            metadata: none!(),
            globals: tiny_bmap! { GS_GENESIS_DIGEST => Occurrences::Once },
            assignments: tiny_bmap! { OS_IDENTITY => Occurrences::Once },
            validator: None,
        },
        transitions: tiny_bmap! {
            TS_TRANSITION => transition_details("transition"),
            TS_RECOVER => transition_details("recover")
        },
        default_assignment: Some(OS_IDENTITY),
    };
    let types = standard.type_system(schema.clone());
    Program {
        schema,
        types,
        lib_id,
    }
}

pub fn issue(
    program: &Program,
    chain: ChainNet,
    genesis_digest: [u8; 32],
    outpoint: Outpoint,
    blinding: u64,
    timestamp: i64,
) -> Result<rgbstd::containers::ValidConsignment<false>, String> {
    let seal: BlindSeal<rgbstd::Txid> =
        BlindSeal::with_blinding(outpoint.txid, outpoint.vout, blinding);
    let genesis_seal: GenesisSeal = seal;
    ContractBuilder::with(
        Identity::default(),
        program.schema.clone(),
        program.types.clone(),
        none!(),
        chain,
    )
    .add_global_state("genesisDigest", O2aDigest(genesis_digest))
    .map_err(|err| err.to_string())?
    .add_rights("identity", BuilderSeal::from(genesis_seal))
    .map_err(|err| err.to_string())?
    .issue_contract_raw(timestamp)
    .map_err(|err| err.to_string())
}

/// `name` is `transition` for O2A operations 1, 2, 4, 5 and `recover` for 3.
#[allow(clippy::too_many_arguments)]
pub fn transition(
    program: &Program,
    contract_id: rgbstd::ContractId,
    name: &'static str,
    anchor_digest: [u8; 32],
    opout: Opout,
    state: AllocatedState,
    next_vout: u32,
    blinding: u64,
    nonce: u64,
) -> Result<Transition, String> {
    // rgb-consensus 0.11.1 panics on an unknown transition name; refuse first.
    if !matches!(name, "transition" | "recover") {
        return Err(format!("unknown O2A program transition `{name}`"));
    }
    let next: GraphSeal = BlindSeal::with_blinded_vout(next_vout, blinding);
    TransitionBuilder::named_transition(
        contract_id,
        program.schema.clone(),
        strict_types::fname!(name),
        program.types.clone(),
    )
    .map_err(|err| err.to_string())?
    .set_nonce(nonce)
    .add_global_state("transitionDigest", O2aDigest(anchor_digest))
    .map_err(|err| err.to_string())?
    .add_input(opout, state)
    .map_err(|err| err.to_string())?
    .add_rights("identity", BuilderSeal::from(next))
    .map_err(|err| err.to_string())?
    .complete_transition()
    .map_err(|err| err.to_string())
}

/// Deterministic identifiers and bytes of the candidate program.
pub fn dump() -> Result<Value, String> {
    use rgbstd::bitcoin::hashes::{sha256, Hash};
    let program = program();
    let bytes = program
        .schema
        .to_strict_serialized::<MAX_SCHEMA_BYTES>()
        .map_err(|err| err.to_string())?
        .release();
    let types = program
        .types
        .to_strict_serialized::<MAX_SCHEMA_BYTES>()
        .map_err(|err| err.to_string())?
        .release();
    Ok(json!({
        "program": "O2A RGB program v0.1 (candidate)",
        "rgb_line": "rgb-protocol 0.11.1",
        "close_method": "opret1st",
        "type_library": LIB_NAME,
        "type_library_id": program.lib_id,
        "schema_name": "O2aIdentityV01",
        "schema_id": program.schema.schema_id().to_string(),
        "schema_strict_hex": hex::encode(&bytes),
        "schema_strict_sha256": sha256::Hash::hash(&bytes).to_string(),
        "schema_strict_len": bytes.len(),
        "type_system_id": program.types.id().to_string(),
        "type_system_strict_sha256": sha256::Hash::hash(&types).to_string(),
        "type_system_strict_len": types.len(),
        "global_types": { "3101": "genesisDigest", "3102": "transitionDigest" },
        "owned_types": { "4101": "identity (Declarative)" },
        "transition_types": { "8101": "transition", "8102": "recover" },
        "validators": "none",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgbstd::bitcoin::hashes::Hash;
    use rgbstd::containers::ConsignmentExt;
    use rgbstd::Operation;

    #[test]
    fn program_is_deterministic() {
        assert_eq!(dump().unwrap(), dump().unwrap());
    }

    #[test]
    fn differs_from_demo_schema() {
        let demo = crate::schema::identity_schema().schema.schema_id();
        assert_ne!(program().schema.schema_id(), demo);
    }

    #[test]
    fn genesis_and_transition_carry_digests() {
        let p = program();
        let outpoint = Outpoint::new(rgbstd::Txid::from_byte_array([7u8; 32]), 0);
        let issued = issue(
            &p,
            ChainNet::BitcoinRegtest,
            [1u8; 32],
            outpoint,
            5,
            1_759_017_600,
        )
        .expect("issue");
        let contract_id = ConsignmentExt::contract_id(&*issued);
        let opout = Opout::new(ConsignmentExt::genesis(&*issued).id(), OS_IDENTITY, 0);
        for name in ["transition", "recover"] {
            transition(
                &p,
                contract_id,
                name,
                [2u8; 32],
                opout,
                AllocatedState::Void,
                1,
                5,
                9,
            )
            .expect(name);
        }
    }

    /// Records whether the 0.11.1 builder alone enforces `transitionDigest`
    /// Once. Schema validation of a mined consignment is the binding check.
    #[test]
    fn builder_without_digest_outcome_is_recorded() {
        let p = program();
        let outpoint = Outpoint::new(rgbstd::Txid::from_byte_array([7u8; 32]), 0);
        let issued = issue(
            &p,
            ChainNet::BitcoinRegtest,
            [1u8; 32],
            outpoint,
            5,
            1_759_017_600,
        )
        .expect("issue");
        let opout = Opout::new(ConsignmentExt::genesis(&*issued).id(), OS_IDENTITY, 0);
        let next: GraphSeal = BlindSeal::with_blinded_vout(1, 5);
        let result = TransitionBuilder::named_transition(
            ConsignmentExt::contract_id(&*issued),
            p.schema.clone(),
            strict_types::fname!("transition"),
            p.types.clone(),
        )
        .unwrap()
        .add_input(opout, AllocatedState::Void)
        .unwrap()
        .add_rights("identity", BuilderSeal::from(next))
        .unwrap()
        .complete_transition();
        println!("builder_without_digest is_err={}", result.is_err());
    }

    #[test]
    fn unknown_transition_name_is_rejected() {
        let p = program();
        let outpoint = Outpoint::new(rgbstd::Txid::from_byte_array([7u8; 32]), 0);
        let issued = issue(
            &p,
            ChainNet::BitcoinRegtest,
            [1u8; 32],
            outpoint,
            5,
            1_759_017_600,
        )
        .expect("issue");
        let opout = Opout::new(ConsignmentExt::genesis(&*issued).id(), OS_IDENTITY, 0);
        let result = transition(
            &p,
            ConsignmentExt::contract_id(&*issued),
            "rotate",
            [2u8; 32],
            opout,
            AllocatedState::Void,
            1,
            5,
            9,
        );
        assert!(result.is_err());
    }
}
