//! Regtest seal adapter. It stores seal records, builds script-path spends,
//! and gathers explicit Bitcoin evidence. O2A evaluation stays in
//! `o2a-demo-core`.

mod address;
mod electrum;
mod spend;
mod store;

pub use address::{core_descriptor, prepare_state, regtest_address, PrepareText};
pub use electrum::{gather_report, inclusion_proof, Electrum, ObservedSeal, TrackedSeal};
pub use spend::{
    close_plain_tx, finish_spend, open_spend, script_path_tx, OpenSpend, SpendRequest, SpendStyle,
};
pub use store::{SealRecord, SealStore};
