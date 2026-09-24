//! RGB 0.12 RC3 adapter for the disposable O2A demo.
//!
//! The Codex in this module validates RGB seal-linked history. O2A object
//! authorization is deliberately a separate `o2a-demo-core` result.

#[macro_use]
extern crate amplify;
extern crate alloc;
#[macro_use]
extern crate strict_types;

use aluvm::{aluasm, CoreConfig, Lib, LibSite};
use amplify::confinement::SmallOrdMap;
use bpstd::seals::WTxoSeal;
use bpstd::Outpoint;
use rgb::{Assignment, CallParams, CoreParams, CreateParams, RgbSealDef};
use sonicapi::{
    Aggregator, Api, GlobalApi, Issuer, OwnedApi, RawBuilder, RawConvertor, Semantics, StateArithm,
    StateAtom, StateBuilder, StateConvertor, SubAggregator,
};
use strict_types::stl::std_stl;
use strict_types::{LibBuilder, SemId, StrictVal, SymbolicSys, SystemBuilder, TypeLib, TypeSystem};
use ultrasonic::aluvm::FIELD_ORDER_SECP;
use ultrasonic::{CellAddr, Codex, Identity};

pub const RGB_RUNTIME_REV: &str = "a1e6b41524131f6d6f183b2235fdaacb5c1abb31";
pub const DEMO_PROFILE_VERSION: u16 = 1;
const LIB_NAME_O2A_DEMO: &str = "O2ADemo";

#[derive(
    Copy,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Debug,
    StrictType,
    StrictDumb,
    StrictEncode,
    StrictDecode,
)]
#[strict_type(lib = LIB_NAME_O2A_DEMO)]
pub struct Bytes32(pub [u8; 32]);

#[derive(
    Copy,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Debug,
    StrictType,
    StrictDumb,
    StrictEncode,
    StrictDecode,
)]
#[strict_type(lib = LIB_NAME_O2A_DEMO)]
pub struct ProfileVersion(pub u16);

#[derive(
    Copy,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Debug,
    StrictType,
    StrictDumb,
    StrictEncode,
    StrictDecode,
)]
#[strict_type(lib = LIB_NAME_O2A_DEMO)]
pub struct IdentityCommitment(pub [u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenesisInput {
    pub root_xonly: [u8; 32],
    pub entity_id: [u8; 32],
    pub controller_xonly: [u8; 32],
    pub policy_hash: [u8; 32],
    pub state_commitment: [u8; 32],
    pub seal: Outpoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RotationInput {
    pub previous_cell: CellAddr,
    pub controller_xonly: [u8; 32],
    pub state_commitment: [u8; 32],
    pub next_seal: WTxoSeal,
}

#[derive(Clone, Debug)]
pub struct RotationCall {
    pub params: CallParams,
    pub seals: SmallOrdMap<u16, WTxoSeal>,
}

pub fn runtime_type_name() -> &'static str {
    core::any::type_name::<rgbp::MemUtxos>()
}

fn success_lib() -> Lib {
    Lib::assemble(&aluasm! {
        stop;
    })
    .expect("the fixed demo verifier must assemble")
}

fn codex() -> Codex {
    let lib = success_lib();
    let lib_id = lib.lib_id();
    Codex {
        name: tiny_s!("O2AIdentityDemo"),
        developer: Identity::default(),
        version: default!(),
        timestamp: 1_790_208_000,
        features: none!(),
        field_order: FIELD_ORDER_SECP,
        input_config: CoreConfig::default(),
        verification_config: CoreConfig::default(),
        verifiers: tiny_bmap! {
            0 => LibSite::new(lib_id, 0),
            1 => LibSite::new(lib_id, 0),
            2 => LibSite::new(lib_id, 0),
        },
    }
}

#[derive(Debug)]
struct DemoTypes(SymbolicSys);

fn demo_stl() -> TypeLib {
    LibBuilder::with(
        libname!(LIB_NAME_O2A_DEMO),
        [std_stl().to_dependency_types()],
    )
    .transpile::<Bytes32>()
    .transpile::<ProfileVersion>()
    .transpile::<IdentityCommitment>()
    .compile()
    .expect("invalid O2A demo type library")
}

impl DemoTypes {
    fn new() -> Self {
        Self(
            SystemBuilder::new()
                .import(std_stl())
                .expect("standard strict type library")
                .import(demo_stl())
                .expect("O2A demo strict type library")
                .finalize()
                .expect("O2A demo type system"),
        )
    }

    fn type_system(&self) -> TypeSystem {
        let library = demo_stl();
        let types = library.types.iter().map(|(name, ty)| ty.sem_id_named(name));
        self.0
            .as_types()
            .extract(types)
            .expect("O2A demo type extraction")
    }

    fn get(&self, name: &'static str) -> SemId {
        *self
            .0
            .resolve(name)
            .unwrap_or_else(|| panic!("type '{name}' is absent in the O2A demo library"))
    }
}

fn global_api(types: &DemoTypes, name: &'static str) -> GlobalApi {
    GlobalApi {
        published: true,
        sem_id: types.get(name),
        convertor: StateConvertor::TypedEncoder(default!()),
        builder: StateBuilder::TypedEncoder(default!()),
        raw_convertor: RawConvertor::StrictDecode(SemId::unit()),
        raw_builder: RawBuilder::StrictEncode(SemId::unit()),
    }
}

fn api() -> Api {
    let types = DemoTypes::new();
    let codex = codex();
    Api {
        codex_id: codex.codex_id(),
        conforms: none!(),
        default_call: None,
        global: tiny_bmap! {
            vname!("root") => global_api(&types, "O2ADemo.Bytes32"),
            vname!("entityId") => global_api(&types, "O2ADemo.Bytes32"),
            vname!("controller") => global_api(&types, "O2ADemo.Bytes32"),
            vname!("policyHash") => global_api(&types, "O2ADemo.Bytes32"),
            vname!("profileVersion") => global_api(&types, "O2ADemo.ProfileVersion"),
        },
        owned: tiny_bmap! {
            vname!("identity") => OwnedApi {
                sem_id: types.get("O2ADemo.IdentityCommitment"),
                arithmetics: StateArithm::NonFungible,
                convertor: StateConvertor::TypedEncoder(default!()),
                builder: StateBuilder::TypedEncoder(default!()),
                witness_sem_id: SemId::unit(),
                witness_builder: StateBuilder::TypedEncoder(default!()),
            },
        },
        aggregators: tiny_bmap! {
            vname!("currentController") => Aggregator::Take(SubAggregator::Last(vname!("controller"))),
            vname!("currentPolicyHash") => Aggregator::Take(SubAggregator::Last(vname!("policyHash"))),
            vname!("currentProfileVersion") => Aggregator::Take(SubAggregator::Last(vname!("profileVersion"))),
        },
        verifiers: tiny_bmap! {
            vname!("issue") => 0,
            vname!("rotateController") => 1,
            vname!("revoke") => 2,
        },
        errors: Default::default(),
    }
}

pub fn demo_issuer() -> Issuer {
    let types = DemoTypes::new();
    let codex = codex();
    let semantics = Semantics {
        version: 0,
        default: api(),
        custom: none!(),
        codex_libs: small_bset![success_lib()],
        api_libs: none!(),
        types: types.type_system(),
    };
    Issuer::new(codex, semantics).expect("fixed demo semantics must match its Codex")
}

fn bytes32(value: [u8; 32]) -> StrictVal {
    svbytes!(value)
}

/// Builds RGB genesis parameters after O2A genesis authorization succeeds.
pub fn genesis_params(input: GenesisInput) -> CreateParams<Outpoint> {
    let issuer = demo_issuer();
    let mut params = CreateParams::new_bitcoin_testnet(issuer.codex_id(), "O2AIdentity");
    params = params
        .with_global_verified("root", bytes32(input.root_xonly))
        .with_global_verified("entityId", bytes32(input.entity_id))
        .with_global_verified("controller", bytes32(input.controller_xonly))
        .with_global_verified("policyHash", bytes32(input.policy_hash))
        .with_global_verified("profileVersion", svnum!(DEMO_PROFILE_VERSION));
    params.push_owned_unlocked(
        "identity",
        Assignment::new_internal(input.seal, bytes32(input.state_commitment)),
    );
    params
}

/// Builds the only controller-rotation shape accepted by the demo adapter.
pub fn controller_rotation(input: RotationInput) -> RotationCall {
    let mut params = CallParams {
        core: CoreParams {
            method: vname!("rotateController"),
            global: none!(),
            owned: none!(),
        },
        using: none!(),
        reading: none!(),
    };
    params.using.insert(input.previous_cell, None);
    params.core.push_global_verified(
        "controller",
        StateAtom::new_verified(bytes32(input.controller_xonly)),
    );
    params.core.push_owned_unlocked(
        "identity",
        input.next_seal.auth_token(),
        bytes32(input.state_commitment),
    );
    RotationCall {
        params,
        seals: small_bmap![0 => input.next_seal],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;
    use strict_encoding::StrictDumb;

    use bpstd::seals::{TxoSealExt, WOutpoint};

    const TXID: &str = "0000000000000000000000000000000000000000000000000000000000000001";

    fn outpoint(vout: u32) -> Outpoint {
        Outpoint::from_str(&format!("{TXID}:{vout}")).expect("test outpoint")
    }

    fn seal(vout: u32) -> WTxoSeal {
        WTxoSeal {
            primary: WOutpoint::Extern(outpoint(vout)),
            secondary: TxoSealExt::strict_dumb(),
        }
    }

    #[test]
    fn runtime_revision_is_full_hash() {
        assert_eq!(RGB_RUNTIME_REV.len(), 40);
        assert!(runtime_type_name().contains("MemUtxos"));
    }

    #[test]
    fn issuer_is_deterministic() {
        assert_eq!(demo_issuer().codex_id(), demo_issuer().codex_id());
        assert_eq!(demo_issuer().codex_name().as_str(), "O2AIdentityDemo");
    }

    #[test]
    fn genesis_and_rotation_have_one_owned_seal() {
        let genesis = genesis_params(GenesisInput {
            root_xonly: [1; 32],
            entity_id: [2; 32],
            controller_xonly: [3; 32],
            policy_hash: [4; 32],
            state_commitment: [5; 32],
            seal: outpoint(0),
        });
        assert_eq!(genesis.owned.len(), 1);
        assert_eq!(genesis.global.len(), 5);

        let rotation = controller_rotation(RotationInput {
            previous_cell: CellAddr::strict_dumb(),
            controller_xonly: [6; 32],
            state_commitment: [7; 32],
            next_seal: seal(1),
        });
        assert_eq!(rotation.params.using.len(), 1);
        assert_eq!(rotation.params.core.global.len(), 1);
        assert_eq!(rotation.params.core.owned.len(), 1);
        assert_eq!(rotation.seals.len(), 1);
    }
}
