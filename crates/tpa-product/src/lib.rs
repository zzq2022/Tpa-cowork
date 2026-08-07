//! Product identity constants generated from `tpa/product.toml`.

include!(concat!(env!("OUT_DIR"), "/product.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_identity_matches_product_contract() {
        assert_eq!(DISPLAY_NAME, "TPA CoWork");
        assert_eq!(AGENT_NAME, "TPA-Agent");
        assert_eq!(DATA_DIR, ".tpa-cowork");
        assert_eq!(ENV_PREFIX, "TPA_COWORK_");
        assert_eq!(BINARY_NAME, "tpa-cowork");
        assert_eq!(SERVICE_NAME, "tpa-cowork");
        assert_eq!(NATIVE_HOST_ID, "com.tpacowork.chrome");
    }
}
