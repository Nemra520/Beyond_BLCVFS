#[derive(Debug, Clone)]
pub struct Bundle {
    pub bundle_index: i32,
    pub name: String,
    pub dependencies: Vec<i32>,
    pub direct_reverse_dependencies: Vec<i32>,
    pub direct_dependencies: Vec<i32>,
    pub bundle_flags: i32,
    pub hash_name: i64,
    pub hash_version: i64,
    pub category: i32,
}

#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub path_hash_head: i64,
    pub path: String,
    pub bundle_index: i32,
    pub asset_size: i32,
}

#[derive(Debug, Clone)]
pub struct ManifestScheme {
    pub version: String,
    pub hash: String,
    pub perforce_cl: String,
    pub m_asset_info_address: i64,
    pub m_bundle_address: i64,
    pub m_bundle_array_address: i64,
    pub m_data_address: i64,
    pub bundles: Vec<Bundle>,
    pub assets: Vec<AssetInfo>,
}
