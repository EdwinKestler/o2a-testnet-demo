//! O2A identity schema on RGB 0.11.1. Validators stay unset.

use amplify::{bmap, none, tiny_bmap, zero};
use rgbstd::containers::BuilderSeal;
use rgbstd::contract::{AllocatedState, ContractBuilder, TransitionBuilder};
use rgbstd::stl::{rgb_contract_stl, StandardTypes};
use rgbstd::txout::BlindSeal;
use rgbstd::validation::Scripts;
use rgbstd::{
    AssignmentType, ChainNet, GenesisSeal, GlobalStateSchema, GlobalStateType, GraphSeal, Identity,
    Occurrences, Opout, Outpoint, OwnedStateSchema, Schema, Transition, TransitionType, TypeSystem,
};
use strict_types::{
    LibBuilder, StrictDecode, StrictDumb, StrictEncode, StrictSerialize, StrictType,
};

pub const LIB_NAME_O2A: &str = "O2AIdentity011";
pub const GS_DIGEST: GlobalStateType = GlobalStateType::with(3101);
pub const OS_IDENTITY: AssignmentType = AssignmentType::with(4101);
pub const TS_ROTATE: TransitionType = TransitionType::with(8101);
pub const TS_RECOVER: TransitionType = TransitionType::with(8102);
pub const TS_REVOKE: TransitionType = TransitionType::with(8103);

pub const TS_ISSUE: i64 = 1_759_017_600;

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, StrictType, StrictEncode, StrictDecode,
)]
#[strict_type(lib = LIB_NAME_O2A)]
pub struct O2aDigest(pub [u8; 32]);

impl StrictDumb for O2aDigest {
    fn strict_dumb() -> Self {
        Self([0u8; 32])
    }
}

impl StrictSerialize for O2aDigest {}

pub struct IdentitySchema {
    pub schema: Schema,
    pub types: TypeSystem,
}

pub fn identity_schema() -> IdentitySchema {
    let lib = LibBuilder::with(strict_types::libname!("O2AIdentity011"), std::iter::empty())
        .transpile::<O2aDigest>()
        .compile()
        .expect("O2aDigest type library");
    let standard = StandardTypes::with(lib);
    let _ = rgb_contract_stl();
    let sem_id = standard.get("O2AIdentity011.O2aDigest");
    let schema = Schema {
        ffv: zero!(),
        name: strict_types::tn!("O2aIdentity"),
        meta_types: none!(),
        global_types: tiny_bmap! {
            GS_DIGEST => rgbstd::GlobalDetails {
                global_state_schema: GlobalStateSchema::once(sem_id),
                name: strict_types::fname!("digest"),
            }
        },
        owned_types: tiny_bmap! {
            OS_IDENTITY => rgbstd::AssignmentDetails {
                owned_state_schema: OwnedStateSchema::Declarative,
                name: strict_types::fname!("identity"),
                default_transition: TS_ROTATE,
            }
        },
        genesis: rgbstd::GenesisSchema {
            metadata: none!(),
            globals: tiny_bmap! {
                GS_DIGEST => Occurrences::Once,
            },
            assignments: tiny_bmap! {
                OS_IDENTITY => Occurrences::Once,
            },
            validator: None,
        },
        transitions: tiny_bmap! {
            TS_ROTATE => rgbstd::TransitionDetails {
                transition_schema: rgbstd::TransitionSchema {
                    metadata: none!(),
                    globals: none!(),
                    inputs: tiny_bmap! { OS_IDENTITY => Occurrences::Once },
                    assignments: tiny_bmap! { OS_IDENTITY => Occurrences::Once },
                    validator: None,
                },
                name: strict_types::fname!("rotate"),
            },
            TS_RECOVER => rgbstd::TransitionDetails {
                transition_schema: rgbstd::TransitionSchema {
                    metadata: none!(),
                    globals: none!(),
                    inputs: tiny_bmap! { OS_IDENTITY => Occurrences::Once },
                    assignments: tiny_bmap! { OS_IDENTITY => Occurrences::Once },
                    validator: None,
                },
                name: strict_types::fname!("recover"),
            },
            TS_REVOKE => rgbstd::TransitionDetails {
                transition_schema: rgbstd::TransitionSchema {
                    metadata: none!(),
                    globals: none!(),
                    inputs: tiny_bmap! { OS_IDENTITY => Occurrences::Once },
                    assignments: none!(),
                    validator: None,
                },
                name: strict_types::fname!("revoke"),
            },
        },
        default_assignment: Some(OS_IDENTITY),
    };
    let types = standard.type_system(schema.clone());
    IdentitySchema { schema, types }
}

pub fn empty_scripts() -> Scripts {
    none!()
}

pub fn issue_at(
    prepared: &IdentitySchema,
    digest: [u8; 32],
    outpoint: Outpoint,
    blinding: u64,
    timestamp: i64,
) -> Result<rgbstd::containers::ValidConsignment<false>, String> {
    issue_on(
        prepared,
        ChainNet::BitcoinRegtest,
        digest,
        outpoint,
        blinding,
        timestamp,
    )
}

pub fn issue_on(
    prepared: &IdentitySchema,
    chain: ChainNet,
    digest: [u8; 32],
    outpoint: Outpoint,
    blinding: u64,
    timestamp: i64,
) -> Result<rgbstd::containers::ValidConsignment<false>, String> {
    let seal: BlindSeal<rgbstd::Txid> =
        BlindSeal::with_blinding(outpoint.txid, outpoint.vout, blinding);
    let genesis_seal: GenesisSeal = seal;
    ContractBuilder::with(
        Identity::default(),
        prepared.schema.clone(),
        prepared.types.clone(),
        empty_scripts(),
        chain,
    )
    .add_global_state("digest", O2aDigest(digest))
    .map_err(|err| err.to_string())?
    .add_rights("identity", BuilderSeal::from(genesis_seal))
    .map_err(|err| err.to_string())?
    .issue_contract_raw(timestamp)
    .map_err(|err| err.to_string())
}

pub fn right_transition(
    prepared: &IdentitySchema,
    contract_id: rgbstd::ContractId,
    name: &'static str,
    opout: Opout,
    state: AllocatedState,
    next_vout: u32,
    blinding: u64,
    nonce: u64,
) -> Result<Transition, String> {
    let next: GraphSeal = BlindSeal::with_blinded_vout(next_vout, blinding);
    TransitionBuilder::named_transition(
        contract_id,
        prepared.schema.clone(),
        strict_types::fname!(name),
        prepared.types.clone(),
    )
    .map_err(|err| err.to_string())?
    .set_nonce(nonce)
    .add_input(opout, state)
    .map_err(|err| err.to_string())?
    .add_rights("identity", BuilderSeal::from(next))
    .map_err(|err| err.to_string())?
    .complete_transition()
    .map_err(|err| err.to_string())
}

pub fn revoke_transition(
    prepared: &IdentitySchema,
    contract_id: rgbstd::ContractId,
    opout: Opout,
    state: AllocatedState,
    nonce: u64,
) -> Result<Transition, String> {
    TransitionBuilder::named_transition(
        contract_id,
        prepared.schema.clone(),
        strict_types::fname!("revoke"),
        prepared.types.clone(),
    )
    .map_err(|err| err.to_string())?
    .set_nonce(nonce)
    .add_input(opout, state)
    .map_err(|err| err.to_string())?
    .complete_transition()
    .map_err(|err| err.to_string())
}
