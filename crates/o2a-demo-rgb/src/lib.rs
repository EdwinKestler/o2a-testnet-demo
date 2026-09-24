//! RGB 0.12 RC3 adapter for the disposable O2A demo.

/// Exact RGB-WG runtime revision authorized for the demo lineage.
pub const RGB_RUNTIME_REV: &str = "a1e6b41524131f6d6f183b2235fdaacb5c1abb31";

/// Compile-time use of the pinned runtime without adding network behavior.
pub fn runtime_type_name() -> &'static str {
    core::any::type_name::<rgbp::MemUtxos>()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_revision_is_full_hash() {
        assert_eq!(RGB_RUNTIME_REV.len(), 40);
        assert!(runtime_type_name().contains("MemUtxos"));
    }
}
