use serde_json::{Map, Value};

use super::types::{ManifestScheme, Bundle, AssetInfo};

/// Convert ManifestScheme to JSON Value
pub fn scheme_to_json(scheme: &ManifestScheme) -> Value {
    let mut root = Map::new();

    root.insert("Version".to_string(), Value::String(scheme.version.clone()));
    root.insert("Hash".to_string(), Value::String(scheme.hash.clone()));
    root.insert("perforceCL".to_string(), Value::String(scheme.perforce_cl.clone()));
    root.insert("m_assetInfoAddress".to_string(), Value::Number(scheme.m_asset_info_address.into()));
    root.insert("m_bundleAddress".to_string(), Value::Number(scheme.m_bundle_address.into()));
    root.insert("m_bundleArrayAddress".to_string(), Value::Number(scheme.m_bundle_array_address.into()));
    root.insert("m_dataAddress".to_string(), Value::Number(scheme.m_data_address.into()));

    let bundles: Vec<Value> = scheme.bundles.iter().map(bundle_to_json).collect();
    root.insert("Bundles".to_string(), Value::Array(bundles));

    let assets: Vec<Value> = scheme.assets.iter().map(asset_to_json).collect();
    root.insert("Assets".to_string(), Value::Array(assets));

    Value::Object(root)
}

fn bundle_to_json(b: &Bundle) -> Value {
    let mut map = Map::new();
    map.insert("bundleIndex".to_string(), Value::Number(b.bundle_index.into()));
    map.insert("name".to_string(), Value::String(b.name.clone()));
    map.insert("dependencies".to_string(),
        Value::Array(b.dependencies.iter().map(|&v| Value::Number(v.into())).collect()));
    map.insert("directReverseDependencies".to_string(),
        Value::Array(b.direct_reverse_dependencies.iter().map(|&v| Value::Number(v.into())).collect()));
    map.insert("directDependencies".to_string(),
        Value::Array(b.direct_dependencies.iter().map(|&v| Value::Number(v.into())).collect()));
    map.insert("bundleFlags".to_string(), Value::Number(b.bundle_flags.into()));
    map.insert("hashName".to_string(), Value::Number(b.hash_name.into()));
    map.insert("hashVersion".to_string(), Value::Number(b.hash_version.into()));
    map.insert("category".to_string(), Value::Number(b.category.into()));
    Value::Object(map)
}

fn asset_to_json(a: &AssetInfo) -> Value {
    let mut map = Map::new();
    map.insert("pathHashHead".to_string(), Value::Number(a.path_hash_head.into()));
    map.insert("path".to_string(), Value::String(a.path.clone()));
    map.insert("bundleIndex".to_string(), Value::Number(a.bundle_index.into()));
    map.insert("assetSize".to_string(), Value::Number(a.asset_size.into()));
    Value::Object(map)
}
