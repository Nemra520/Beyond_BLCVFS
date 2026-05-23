use super::types::{ManifestJson, ManifestVFS, BundleInfo, AssetInfo};
use std::path::Path;

pub struct ManifestParser;

impl ManifestParser {
    /// 从 JSON 字符串解析 manifest
    pub fn parse_json(json_str: &str) -> Result<ManifestJson, String> {
        serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse manifest JSON: {}", e))
    }

    /// 从 JSON 文件路径解析 manifest
    pub fn parse_from_file<P: AsRef<Path>>(path: P) -> Result<ManifestJson, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest file: {}", e))?;
        Self::parse_json(&content)
    }

    /// 从 hgmmap 二进制数据解析 manifest
    pub fn parse_hgmmap(data: &[u8]) -> Result<ManifestJson, String> {
        // 使用 HgmmapParser 解析二进制数据
        let scheme = crate::VFS::FileType::hgmmap::HgmmapParser::parse(data)
            .map_err(|e| format!("Failed to parse hgmmap: {}", e))?;

        // 将 ManifestScheme 转换为 ManifestJson
        let bundles: Vec<BundleInfo> = scheme.bundles.into_iter().map(|b| BundleInfo {
            bundle_flags: b.bundle_flags,
            bundle_index: b.bundle_index as i64,
            category: b.category,
            dependencies: b.dependencies.into_iter().map(|d| d as i64).collect(),
            direct_dependencies: b.direct_dependencies.into_iter().map(|d| d as i64).collect(),
            direct_reverse_dependencies: b.direct_reverse_dependencies.into_iter().map(|d| d as i64).collect(),
            hash_name: b.hash_name,
            hash_version: b.hash_version,
            name: b.name,
        }).collect();

        let assets: Vec<AssetInfo> = scheme.assets.into_iter().map(|a| AssetInfo {
            asset_size: a.asset_size as i64,
            bundle_index: a.bundle_index as i64,
            path: a.path,
            path_hash_head: a.path_hash_head,
        }).collect();

        Ok(ManifestJson {
            assets,
            bundles,
            hash: Some(scheme.hash),
            version: Some(scheme.version),
            perforce_cl: Some(scheme.perforce_cl),
            m_asset_info_address: Some(scheme.m_asset_info_address),
            m_bundle_address: Some(scheme.m_bundle_address),
            m_bundle_array_address: Some(scheme.m_bundle_array_address),
            m_data_address: Some(scheme.m_data_address),
        })
    }

    /// 从 hgmmap 文件路径解析 manifest
    pub fn parse_hgmmap_from_file<P: AsRef<Path>>(path: P) -> Result<ManifestJson, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read hgmmap file: {}", e))?;
        Self::parse_hgmmap(&data)
    }

    /// 从 JSON 字符串创建 ManifestVFS
    pub fn create_vfs(json_str: &str) -> Result<ManifestVFS, String> {
        let manifest = Self::parse_json(json_str)?;
        Ok(ManifestVFS::new(manifest))
    }

    /// 从 JSON 文件路径创建 ManifestVFS
    pub fn create_vfs_from_file<P: AsRef<Path>>(path: P) -> Result<ManifestVFS, String> {
        let manifest = Self::parse_from_file(path)?;
        Ok(ManifestVFS::new(manifest))
    }

    /// 从 hgmmap 数据创建 ManifestVFS
    pub fn create_vfs_from_hgmmap(data: &[u8]) -> Result<ManifestVFS, String> {
        let manifest = Self::parse_hgmmap(data)?;
        Ok(ManifestVFS::new(manifest))
    }

    /// 从 hgmmap 文件路径创建 ManifestVFS
    pub fn create_vfs_from_hgmmap_file<P: AsRef<Path>>(path: P) -> Result<ManifestVFS, String> {
        let manifest = Self::parse_hgmmap_from_file(path)?;
        Ok(ManifestVFS::new(manifest))
    }

    /// 检查文件是否是 manifest.json 或 manifest.hgmmap
    pub fn is_manifest_file(path: &str) -> bool {
        let lower = path.to_lowercase();
        lower.ends_with("manifest.json") || lower.ends_with("manifest.hgmmap")
    }

    /// 检查文件是否是 hgmmap 格式的 manifest
    pub fn is_hgmmap_file(path: &str) -> bool {
        path.to_lowercase().ends_with("manifest.hgmmap")
    }

    /// 检查文件是否是 JSON 格式的 manifest
    pub fn is_json_manifest_file(path: &str) -> bool {
        path.to_lowercase().ends_with("manifest.json")
    }

    /// 获取 manifest 文件对应的 hgmmap 目录路径
    /// manifest.hgmmap 通常在 .../Bundles/Windows/ 目录下
    /// 对应的 AB 文件在 .../hgmmap/ 目录下
    pub fn get_hgmmap_dir(manifest_path: &str) -> Option<String> {
        let path = std::path::Path::new(manifest_path);

        // 获取 manifest 的父目录 (Windows)
        let parent = path.parent()?;

        // 获取 Bundles 目录
        let bundles_dir = parent.parent()?;

        // 构建 hgmmap 目录路径: .../Data/hgmmap/
        let data_dir = bundles_dir.parent()?;
        let hgmmap_dir = data_dir.join("hgmmap");

        hgmmap_dir.to_str().map(|s| s.to_string())
    }

    /// 根据 manifest 文件路径自动选择解析方式并创建 VFS
    pub fn create_vfs_auto<P: AsRef<Path>>(path: P) -> Result<ManifestVFS, String> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        if Self::is_hgmmap_file(&path_str) {
            Self::create_vfs_from_hgmmap_file(path)
        } else if Self::is_json_manifest_file(&path_str) {
            Self::create_vfs_from_file(path)
        } else {
            Err(format!("Unknown manifest file format: {}", path_str))
        }
    }
}
