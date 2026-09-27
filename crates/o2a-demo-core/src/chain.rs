//! Pure Bitcoin inclusion checks and the lineage result.
//!
//! Callers pass explicit proofs. This module does not open a socket.

use bitcoin_hashes::{sha256d, Hash};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InclusionProof {
    pub txid: [u8; 32],
    pub index: u32,
    pub siblings: Vec<[u8; 32]>,
    pub header: [u8; 80],
    pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealFact {
    pub expected_script: Vec<u8>,
    pub observed_script: Vec<u8>,
    pub creation: InclusionProof,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentSealView {
    pub unspent: bool,
    pub spend: Option<InclusionProof>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageEvidence {
    pub seals: Vec<SealFact>,
    pub anchor: Option<InclusionProof>,
    pub observation: Option<CurrentSealView>,
    pub o2a_ok: bool,
    pub best_height: u32,
    pub required_depth: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageReport {
    pub identity_history_state: &'static str,
    pub bitcoin: &'static str,
    pub rgb: &'static str,
    pub o2a: &'static str,
    pub header_trust: &'static str,
}

pub fn header_merkle_root(header: &[u8; 80]) -> [u8; 32] {
    let mut root = [0u8; 32];
    root.copy_from_slice(&header[36..68]);
    root
}

fn hash256_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(left);
    data[32..].copy_from_slice(right);
    sha256d::Hash::hash(&data).to_byte_array()
}

pub fn merkle_root(txid: [u8; 32], mut index: u32, siblings: &[[u8; 32]]) -> [u8; 32] {
    let mut current = txid;
    for sibling in siblings {
        let (left, right) = if index % 2 == 0 {
            (current, *sibling)
        } else {
            (*sibling, current)
        };
        current = hash256_pair(&left, &right);
        index /= 2;
    }
    current
}

pub fn inclusion_matches(proof: &InclusionProof) -> bool {
    merkle_root(proof.txid, proof.index, &proof.siblings) == header_merkle_root(&proof.header)
}

fn at_depth(best_height: u32, height: u32, required_depth: u32) -> bool {
    best_height.saturating_add(1) >= height.saturating_add(required_depth)
}

/// Deterministic lineage result from explicit proofs.
///
/// `INVALID` is a script that does not match the policy.
/// `INCOMPLETE` is missing observation, a proof that does not meet the header,
/// or an anchor that is not on the named best chain at the required depth.
pub fn evaluate_lineage(evidence: &LineageEvidence, rgb: &'static str) -> LineageReport {
    let header_trust =
        "headers come from one electrs instance; this is a trust assumption, not a light client";
    let script_mismatch = evidence
        .seals
        .iter()
        .any(|seal| seal.expected_script != seal.observed_script);
    if script_mismatch {
        return LineageReport {
            identity_history_state: "INVALID",
            bitcoin: "script does not match the policy",
            rgb,
            o2a: if evidence.o2a_ok {
                "objects accepted"
            } else {
                "objects rejected"
            },
            header_trust,
        };
    }
    let creation_ok = evidence.seals.iter().all(|seal| {
        inclusion_matches(&seal.creation)
            && at_depth(
                evidence.best_height,
                seal.creation.height,
                evidence.required_depth,
            )
    });
    let anchor_ok = match &evidence.anchor {
        None => true,
        Some(anchor) => {
            inclusion_matches(anchor)
                && at_depth(evidence.best_height, anchor.height, evidence.required_depth)
        }
    };
    if !creation_ok || !anchor_ok {
        return LineageReport {
            identity_history_state: "INCOMPLETE",
            bitcoin: "anchor or seal creation is not on the named best chain at the required depth",
            rgb,
            o2a: if evidence.o2a_ok {
                "objects accepted"
            } else {
                "objects rejected"
            },
            header_trust,
        };
    }
    let Some(observation) = &evidence.observation else {
        return LineageReport {
            identity_history_state: "INCOMPLETE",
            bitcoin: "current seal was not observed",
            rgb,
            o2a: if evidence.o2a_ok {
                "objects accepted"
            } else {
                "objects rejected"
            },
            header_trust,
        };
    };
    if observation.unspent {
        return LineageReport {
            identity_history_state: "CURRENT",
            bitcoin: "current seal is unspent and the supplied proofs meet the header",
            rgb,
            o2a: if evidence.o2a_ok {
                "objects accepted"
            } else {
                "objects rejected"
            },
            header_trust,
        };
    }
    let Some(spend) = &observation.spend else {
        return LineageReport {
            identity_history_state: "INCOMPLETE",
            bitcoin: "spent seal has no inclusion proof",
            rgb,
            o2a: if evidence.o2a_ok {
                "objects accepted"
            } else {
                "objects rejected"
            },
            header_trust,
        };
    };
    if !inclusion_matches(spend)
        || !at_depth(evidence.best_height, spend.height, evidence.required_depth)
    {
        return LineageReport {
            identity_history_state: "INCOMPLETE",
            bitcoin: "seal spend is not on the named best chain at the required depth",
            rgb,
            o2a: if evidence.o2a_ok {
                "objects accepted"
            } else {
                "objects rejected"
            },
            header_trust,
        };
    }
    if evidence.o2a_ok {
        LineageReport {
            identity_history_state: "CURRENT",
            bitcoin: "seal spend is included at the required depth",
            rgb,
            o2a: "objects accepted",
            header_trust,
        }
    } else {
        LineageReport {
            identity_history_state: "SEAL_CLOSED_WITHOUT_VALID_TRANSITION",
            bitcoin: "seal spend is included at the required depth",
            rgb,
            o2a: "no valid transition closes the seal",
            header_trust,
        }
    }
}

pub fn format_lineage_report(report: &LineageReport) -> String {
    format!(
        "identity_history_state={}\nbitcoin={}\nrgb={}\no2a={}\nheader_trust={}\n",
        report.identity_history_state, report.bitcoin, report.rgb, report.o2a, report.header_trust
    )
}
