use std::collections::HashSet;

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub full_path: String,
}

pub struct ExtractProgress {
    pub current: usize,
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}

pub struct PckView {
    pub pck_path: String,
    pub entries: Vec<PckEntryView>,
    #[allow(dead_code)]
    pub parent_dir: String,
    pub selected_entries: HashSet<u64>,
}

pub struct PckEntryView {
    pub file_id: u64,
    pub entry_type: String,
    pub size: usize,
}
