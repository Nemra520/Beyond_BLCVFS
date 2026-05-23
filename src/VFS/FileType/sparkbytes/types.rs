use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct EnumDef {
    #[allow(dead_code)]
    pub name: String,
    pub members: HashMap<i32, String>,
}

#[derive(Clone, Debug)]
pub struct FieldDef {
    pub name: String,
    pub f_type: u8,
    pub inner_type: Option<u8>,
    pub inner_hash: Option<[u8; 4]>,
    pub key_type: Option<u8>,
    pub val_type: Option<u8>,
    pub val_hash: Option<[u8; 4]>,
}

#[derive(Clone, Debug)]
pub struct RawSchema {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Clone, Debug)]
pub struct ComputedField {
    pub name: String,
    pub f_type: u8,
    pub offset: usize,
    pub inner_type: Option<u8>,
    pub inner_hash: Option<[u8; 4]>,
    pub key_type: Option<u8>,
    pub val_type: Option<u8>,
    pub val_hash: Option<[u8; 4]>,
}

#[derive(Clone, Debug)]
pub struct ComputedSchema {
    #[allow(dead_code)]
    pub name: String,
    pub fields: Vec<ComputedField>,
    #[allow(dead_code)]
    pub size: usize,
    #[allow(dead_code)]
    pub align: usize,
}
