use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let identity_path = manifest_dir.join("../../tpa/product.toml");
    println!("cargo:rerun-if-changed={}", identity_path.display());

    let source = fs::read_to_string(&identity_path).expect("read tpa/product.toml");
    let values = source
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((
                key.trim().to_owned(),
                value.trim().trim_matches('"').to_owned(),
            ))
        })
        .collect::<BTreeMap<_, _>>();

    let mappings = [
        ("DISPLAY_NAME", "display_name"),
        ("SLUG", "slug"),
        ("COMPACT_NAME", "compact_name"),
        ("AGENT_NAME", "agent_name"),
        ("DATA_DIR", "data_dir"),
        ("ENV_PREFIX", "env_prefix"),
        ("BINARY_NAME", "binary_name"),
        ("SERVICE_NAME", "service_name"),
        ("USER_AGENT", "user_agent"),
        ("NATIVE_HOST_ID", "native_host_id"),
    ];
    let mut generated = String::from("// @generated from tpa/product.toml; do not edit.\n");
    for (constant, key) in mappings {
        let value = values
            .get(key)
            .unwrap_or_else(|| panic!("missing product identity key: {key}"));
        generated.push_str(&format!("pub const {constant}: &str = {value:?};\n"));
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("product.rs");
    fs::write(out, generated).expect("write generated product identity");
}
