/// Mapping entry: hash -> string offset
pub(super) struct MappingEntry {
    pub hash: i64,
    pub offset: i32,
}

/// Binary header information
pub(super) struct StringPathHeader {
    pub string_pool_offset: i32,
    pub capacity: i32,
}
