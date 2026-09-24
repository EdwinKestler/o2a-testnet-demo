//! Deterministic, network-free O2A encoding and verification boundary.

/// Spec commit whose canonical encoding and cryptographic profile govern this
/// disposable demo.
pub const SPEC_COMMIT: &str = "3ca98ea9b60256f271e71fa89caa09448e804e87";

/// Human-readable references to the only canonical and signing rules used by
/// the demo.
pub const CANONICAL_RULES: [&str; 2] = [
    "../o2a-protocol/specs/canonical-encoding.md",
    "../o2a-protocol/specs/cryptographic-profile.md",
];

/// Confirms that this crate is deliberately network-free.
pub fn evaluation_boundary() -> &'static str {
    "explicit evidence in; deterministic three-layer result out; no network fetch"
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_is_pinned() {
        assert_eq!(SPEC_COMMIT.len(), 40);
        assert_eq!(CANONICAL_RULES.len(), 2);
    }
}
