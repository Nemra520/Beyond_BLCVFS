use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct EnumDef {
    #[allow(dead_code)]
    name: String,
    members: HashMap<i32, String>,
}

#[derive(Clone, Debug)]
struct FieldDef {
    name: String,
    f_type: u8,
    inner_type: Option<u8>,
    inner_hash: Option<[u8; 4]>,
    key_type: Option<u8>,
    val_type: Option<u8>,
    val_hash: Option<[u8; 4]>,
}

#[derive(Clone, Debug)]
struct RawSchema {
    name: String,
    fields: Vec<FieldDef>,
}

#[derive(Clone, Debug)]
struct ComputedField {
    name: String,
    f_type: u8,
    offset: usize,
    inner_type: Option<u8>,
    inner_hash: Option<[u8; 4]>,
    key_type: Option<u8>,
    val_type: Option<u8>,
    val_hash: Option<[u8; 4]>,
}

#[derive(Clone, Debug)]
struct ComputedSchema {
    #[allow(dead_code)]
    name: String,
    fields: Vec<ComputedField>,
    #[allow(dead_code)]
    size: usize,
    #[allow(dead_code)]
    align: usize,
}

struct BytesParser {
    raw_schemas: HashMap<[u8; 4], RawSchema>,
    enums: HashMap<[u8; 4], EnumDef>,
    schemas: HashMap<[u8; 4], ComputedSchema>,
}

impl BytesParser {
    fn new() -> Self {
        Self {
            raw_schemas: HashMap::new(),
            enums: HashMap::new(),
            schemas: HashMap::new(),
        }
    }

    fn parse_schemas_from_bytes(&mut self, data: &[u8]) {
        if data.len() < 16 {
            return;
        }

        let type_count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        let mut offset = 16;

        for _ in 0..type_count {
            if offset >= data.len() {
                break;
            }
            let meta_type = data[offset];
            offset += 1;
            offset = align4(offset);
            if offset + 4 > data.len() {
                break;
            }
            let type_hash: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
            offset += 4;
            let name = read_cstring(data, offset);
            offset += name.len() + 1;
            offset = align4(offset);
            if offset + 4 > data.len() {
                break;
            }
            let count = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;

            if meta_type == 6 {
                let mut members = HashMap::new();
                for _ in 0..count {
                    let m_name = read_cstring(data, offset);
                    offset += m_name.len() + 1;
                    offset = align4(offset);
                    if offset + 4 > data.len() {
                        break;
                    }
                    let m_val = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
                    offset += 4;
                    members.insert(m_val, m_name);
                }
                self.enums.entry(type_hash).or_insert(EnumDef {
                    name,
                    members,
                });
            } else if meta_type == 8 {
                let mut fields = Vec::new();
                for _ in 0..count {
                    let f_name = read_cstring(data, offset);
                    offset += f_name.len() + 1;
                    if offset >= data.len() {
                        break;
                    }
                    let f_type = data[offset];
                    offset += 1;

                    let mut inner_type: Option<u8> = None;
                    let mut inner_hash: Option<[u8; 4]> = None;
                    let mut key_type: Option<u8> = None;
                    let mut val_type: Option<u8> = None;
                    let mut val_hash: Option<[u8; 4]> = None;

                    if f_type == 9 {
                        if offset >= data.len() {
                            break;
                        }
                        inner_type = Some(data[offset]);
                        offset += 1;
                        if inner_type == Some(6) || inner_type == Some(8) {
                            offset = align4(offset);
                            if offset + 4 > data.len() {
                                break;
                            }
                            inner_hash = Some(data[offset..offset + 4].try_into().unwrap());
                            offset += 4;
                        }
                    } else if f_type == 6 || f_type == 8 {
                        offset = align4(offset);
                        if offset + 4 > data.len() {
                            break;
                        }
                        inner_hash = Some(data[offset..offset + 4].try_into().unwrap());
                        offset += 4;
                    } else if f_type == 10 {
                        if offset + 2 > data.len() {
                            break;
                        }
                        key_type = Some(data[offset]);
                        offset += 1;
                        val_type = Some(data[offset]);
                        offset += 1;
                        if val_type == Some(6) || val_type == Some(8) {
                            offset = align4(offset);
                            if offset + 4 > data.len() {
                                break;
                            }
                            val_hash = Some(data[offset..offset + 4].try_into().unwrap());
                            offset += 4;
                        }
                    }

                    fields.push(FieldDef {
                        name: f_name,
                        f_type,
                        inner_type,
                        inner_hash,
                        key_type,
                        val_type,
                        val_hash,
                    });
                }
                self.raw_schemas.entry(type_hash).or_insert(RawSchema {
                    name,
                    fields,
                });
            }
        }
    }

    fn compute_all_schemas(&mut self) {
        let type_hashes: Vec<[u8; 4]> = self.raw_schemas.keys().cloned().collect();
        for th in type_hashes {
            self.compute_schema(&th);
        }
    }

    fn compute_schema(&mut self, type_hash: &[u8; 4]) -> Option<ComputedSchema> {
        if self.schemas.contains_key(type_hash) {
            return self.schemas.get(type_hash).cloned();
        }
        let raw = self.raw_schemas.get(type_hash)?.clone();
        let mut computed_fields = Vec::new();
        let mut row_size: usize = 0;
        let mut max_align: usize = 1;

        for f in &raw.fields {
            let (size, align) = type_size_align(f.f_type);
            if row_size % align != 0 {
                row_size += align - (row_size % align);
            }
            computed_fields.push(ComputedField {
                name: f.name.clone(),
                f_type: f.f_type,
                offset: row_size,
                inner_type: f.inner_type,
                inner_hash: f.inner_hash,
                key_type: f.key_type,
                val_type: f.val_type,
                val_hash: f.val_hash,
            });
            row_size += size;
            max_align = max_align.max(align);
        }

        if row_size % max_align != 0 {
            row_size += max_align - (row_size % max_align);
        }

        let schema = ComputedSchema {
            name: raw.name,
            fields: computed_fields,
            size: row_size,
            align: max_align,
        };
        self.schemas.insert(*type_hash, schema.clone());
        Some(schema)
    }

    fn parse_bytes_file(data: &[u8]) -> Value {
        let mut parser = Self::new();
        parser.parse_schemas_from_bytes(data);
        parser.compute_all_schemas();

        if data.len() < 12 {
            return Value::Null;
        }

        let table_def_offset = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        let data_payload_offset = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;

        if table_def_offset >= data.len() {
            return Value::Null;
        }

        let table_type_marker = data[table_def_offset];
        let is_table = table_type_marker == 0x0A;
        let is_const = table_type_marker == 0x08;

        if is_table {
            parser.read_table(data, table_def_offset, data_payload_offset)
        } else if is_const {
            parser.read_const(data, table_def_offset, data_payload_offset)
        } else {
            Value::Null
        }
    }

    fn read_table(&self, data: &[u8], table_def_offset: usize, data_payload_offset: usize) -> Value {
        let mut offset = table_def_offset + 1;
        let name = read_cstring(data, offset);
        offset += name.len() + 1;
        if offset >= data.len() {
            return Value::Null;
        }
        let key_type = data[offset];
        offset += 1;
        if offset >= data.len() {
            return Value::Null;
        }
        let val_type = data[offset];
        offset += 1;

        let mut val_hash: Option<[u8; 4]> = None;
        if val_type == 6 || val_type == 8 {
            offset = align4(offset);
            if offset + 4 <= data.len() {
                val_hash = Some(data[offset..offset + 4].try_into().unwrap());
                offset += 4;
            }
        }
        let _ = offset;

        let mut result = Map::new();
        let b_offset = data_payload_offset;
        if b_offset + 4 > data.len() {
            return Value::Object(result);
        }
        let bucket_count = u32::from_le_bytes(data[b_offset..b_offset + 4].try_into().unwrap()) as usize;
        let mut b_off = b_offset + 4;

        for _ in 0..bucket_count {
            if b_off + 8 > data.len() {
                break;
            }
            let bucket_ptr = u32::from_le_bytes(data[b_off..b_off + 4].try_into().unwrap()) as usize;
            let bucket_size = u32::from_le_bytes(data[b_off + 4..b_off + 8].try_into().unwrap()) as usize;
            b_off += 8;

            if bucket_size > 0 && bucket_ptr > 0 {
                let mut ptr = bucket_ptr;
                for _ in 0..bucket_size {
                    let (key_val, key_size) = self.read_key(data, ptr, key_type);
                    ptr += key_size;

                    let (val_val, val_size) = self.read_table_value(data, ptr, val_type, val_hash);
                    ptr += val_size;

                    result.insert(key_val, val_val);
                }
            }
        }

        Value::Object(result)
    }

    fn read_key(&self, data: &[u8], offset: usize, key_type: u8) -> (String, usize) {
        match key_type {
            3 => {
                let k = if offset + 12 <= data.len() {
                    i64::from_le_bytes(data[offset + 4..offset + 12].try_into().unwrap()).to_string()
                } else {
                    "0".to_string()
                };
                (k, 12)
            }
            2 => {
                let k = if offset + 4 <= data.len() {
                    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()).to_string()
                } else {
                    "0".to_string()
                };
                (k, 4)
            }
            7 => {
                let k = if offset + 4 <= data.len() {
                    let str_ptr = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    read_cstring(data, str_ptr)
                } else {
                    String::new()
                };
                (k, 4)
            }
            _ => {
                let k = if offset + 4 <= data.len() {
                    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()).to_string()
                } else {
                    "0".to_string()
                };
                (k, 4)
            }
        }
    }

    fn read_table_value(&self, data: &[u8], offset: usize, val_type: u8, val_hash: Option<[u8; 4]>) -> (Value, usize) {
        match val_type {
            8 => {
                let struct_ptr = if offset + 4 <= data.len() {
                    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize
                } else {
                    0
                };
                let v = if struct_ptr == 0 || struct_ptr == 0xFFFFFFFF {
                    Value::Null
                } else if let Some(h) = val_hash {
                    self.read_struct(data, struct_ptr, &h)
                } else {
                    Value::Null
                };
                (v, 4)
            }
            7 => {
                let str_ptr = if offset + 4 <= data.len() {
                    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize
                } else {
                    0
                };
                let v = Value::String(read_cstring(data, str_ptr));
                (v, 4)
            }
            6 => {
                let enum_val = if offset + 4 <= data.len() {
                    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
                } else {
                    0
                };
                let v = if let Some(h) = val_hash {
                    if let Some(enum_def) = self.enums.get(&h) {
                        if let Some(name) = enum_def.members.get(&enum_val) {
                            Value::String(name.clone())
                        } else {
                            Value::Number(enum_val.into())
                        }
                    } else {
                        Value::Number(enum_val.into())
                    }
                } else {
                    Value::Number(enum_val.into())
                };
                (v, 4)
            }
            _ => {
                let (v, _) = self.read_value(data, offset, val_type, None, None, None, None, None);
                (v, 4)
            }
        }
    }

    fn read_const(&self, data: &[u8], table_def_offset: usize, data_payload_offset: usize) -> Value {
        let mut offset = table_def_offset + 1;
        let name = read_cstring(data, offset);
        offset += name.len() + 1;
        offset = align4(offset);
        if offset + 4 > data.len() {
            return Value::Null;
        }
        let val_hash: [u8; 4] = data[offset..offset + 4].try_into().unwrap();

        let mut result = Map::new();
        if self.schemas.contains_key(&val_hash) {
            let struct_val = self.read_struct(data, data_payload_offset, &val_hash);
            if let Value::Object(map) = struct_val {
                for (k, v) in map {
                    result.insert(k, v);
                }
            } else {
                result.insert(name, struct_val);
            }
        } else if self.enums.contains_key(&val_hash) {
            let enum_val = if data_payload_offset + 4 <= data.len() {
                i32::from_le_bytes(data[data_payload_offset..data_payload_offset + 4].try_into().unwrap())
            } else {
                0
            };
            if let Some(enum_def) = self.enums.get(&val_hash) {
                if let Some(member_name) = enum_def.members.get(&enum_val) {
                    result.insert(name, Value::String(member_name.clone()));
                } else {
                    result.insert(name, Value::Number(enum_val.into()));
                }
            } else {
                result.insert(name, Value::Number(enum_val.into()));
            }
        } else {
            let struct_val = self.read_struct(data, data_payload_offset, &val_hash);
            result.insert(name, struct_val);
        }

        Value::Object(result)
    }

    fn read_struct(&self, data: &[u8], offset: usize, type_hash: &[u8; 4]) -> Value {
        let schema = match self.schemas.get(type_hash) {
            Some(s) => s,
            None => return Value::String(format!("<Schema unregistered, Hash:{}>", hex_encode(type_hash))),
        };

        let mut result = Map::new();
        for field in &schema.fields {
            let f_size = field_type_size(field.f_type);
            if offset + field.offset + f_size > data.len() {
                result.insert(field.name.clone(), Value::Null);
                continue;
            }
            let val = self.read_value(
                data,
                offset + field.offset,
                field.f_type,
                field.inner_type,
                field.inner_hash,
                field.key_type,
                field.val_type,
                field.val_hash,
            );
            result.insert(field.name.clone(), val.0);
        }
        Value::Object(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn read_value(
        &self,
        data: &[u8],
        offset: usize,
        val_type: u8,
        inner_type: Option<u8>,
        inner_hash: Option<[u8; 4]>,
        key_type: Option<u8>,
        val_type2: Option<u8>,
        val_hash: Option<[u8; 4]>,
    ) -> (Value, usize) {
        match val_type {
            0 | 1 => {
                if offset < data.len() {
                    (Value::Bool(data[offset] != 0), 1)
                } else {
                    (Value::Null, 0)
                }
            }
            2 => {
                if offset + 4 <= data.len() {
                    (Value::Number(i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()).into()), 4)
                } else {
                    (Value::Null, 0)
                }
            }
            3 => {
                if offset + 8 <= data.len() {
                    (Value::Number(i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap()).into()), 8)
                } else {
                    (Value::Null, 0)
                }
            }
            4 => {
                if offset + 4 <= data.len() {
                    let f = f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
                    (serde_json::Number::from_f64(f as f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null), 4)
                } else {
                    (Value::Null, 0)
                }
            }
            5 => {
                if offset + 8 <= data.len() {
                    let d = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                    (serde_json::Number::from_f64(d)
                        .map(Value::Number)
                        .unwrap_or(Value::Null), 8)
                } else {
                    (Value::Null, 0)
                }
            }
            6 => {
                if offset + 4 <= data.len() {
                    let enum_val = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
                    if let Some(h) = inner_hash {
                        if let Some(enum_def) = self.enums.get(&h) {
                            if let Some(name) = enum_def.members.get(&enum_val) {
                                return (Value::String(name.clone()), 4);
                            }
                        }
                    }
                    (Value::Number(enum_val.into()), 4)
                } else {
                    (Value::Null, 0)
                }
            }
            7 => {
                if offset + 4 <= data.len() {
                    let str_ptr = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    (Value::String(read_cstring(data, str_ptr)), 4)
                } else {
                    (Value::String(String::new()), 4)
                }
            }
            8 => {
                if offset + 4 <= data.len() {
                    let struct_ptr = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    if struct_ptr == 0 || struct_ptr == 0xFFFFFFFF {
                        (Value::Null, 4)
                    } else if let Some(h) = inner_hash {
                        (self.read_struct(data, struct_ptr, &h), 4)
                    } else {
                        (Value::Null, 4)
                    }
                } else {
                    (Value::Null, 0)
                }
            }
            9 => {
                if offset + 4 > data.len() {
                    return (Value::Null, 4);
                }
                let arr_ptr = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                if arr_ptr == 0 || arr_ptr == 0xFFFFFFFF {
                    return (Value::Null, 4);
                }
                if arr_ptr + 4 > data.len() {
                    return (Value::Null, 4);
                }
                let count = u32::from_le_bytes(data[arr_ptr..arr_ptr + 4].try_into().unwrap()) as usize;

                let it = inner_type.unwrap_or(0);
                let mut result = Vec::new();

                if it == 7 {
                    for i in 0..count {
                        if arr_ptr + 4 + i * 4 + 4 > data.len() {
                            break;
                        }
                        let s_ptr = u32::from_le_bytes(data[arr_ptr + 4 + i * 4..arr_ptr + 8 + i * 4].try_into().unwrap()) as usize;
                        result.push(Value::String(read_cstring(data, s_ptr)));
                    }
                } else if it == 8 {
                    let h = inner_hash.unwrap_or([0; 4]);
                    for i in 0..count {
                        if arr_ptr + 4 + i * 4 + 4 > data.len() {
                            break;
                        }
                        let ptr_val = u32::from_le_bytes(data[arr_ptr + 4 + i * 4..arr_ptr + 8 + i * 4].try_into().unwrap()) as usize;
                        result.push(self.read_struct(data, ptr_val, &h));
                    }
                } else {
                    let elem_size = if it == 5 || it == 3 { 8usize } else { 4 };
                    let mut data_offset = arr_ptr + 4;
                    if elem_size == 8 && data_offset % 8 != 0 {
                        data_offset += 8 - (data_offset % 8);
                    }
                    for i in 0..count {
                        let off = data_offset + i * elem_size;
                        let (val, _) = self.read_value(data, off, it, None, None, None, None, None);
                        result.push(val);
                    }
                }

                (Value::Array(result), 4)
            }
            10 => {
                if offset + 4 > data.len() {
                    return (Value::Null, 4);
                }
                let map_ptr = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                if map_ptr == 0 || map_ptr == 0xFFFFFFFF {
                    return (Value::Null, 4);
                }
                if map_ptr + 4 > data.len() {
                    return (Value::Null, 4);
                }
                let bucket_count = u32::from_le_bytes(data[map_ptr..map_ptr + 4].try_into().unwrap()) as usize;

                let mut result = Map::new();
                let mut b_off = map_ptr + 4;

                let kt = key_type.unwrap_or(2);
                let vt = val_type2.unwrap_or(2);
                let vh = val_hash;

                for _ in 0..bucket_count {
                    if b_off + 8 > data.len() {
                        break;
                    }
                    let b_ptr = u32::from_le_bytes(data[b_off..b_off + 4].try_into().unwrap()) as usize;
                    let b_size = u32::from_le_bytes(data[b_off + 4..b_off + 8].try_into().unwrap()) as usize;
                    b_off += 8;

                    if b_size > 0 && b_ptr > 0 {
                        let mut ptr = b_ptr;
                        if ptr % 4 != 0 {
                            ptr += 4 - (ptr % 4);
                        }
                        for _ in 0..b_size {
                            let (k, key_consumed) = self.read_map_key(data, ptr, kt);
                            let v_off = ptr + key_consumed;
                            let v = self.read_map_value(data, v_off, vt, vh);
                            let entry_size = if kt == 3 { 16 } else { 8 };
                            ptr += entry_size;
                            result.insert(k, v);
                        }
                    }
                }

                (Value::Object(result), 4)
            }
            _ => (Value::Null, 0),
        }
    }

    fn read_map_key(&self, data: &[u8], offset: usize, key_type: u8) -> (String, usize) {
        match key_type {
            3 => {
                let k = if offset + 12 <= data.len() {
                    i64::from_le_bytes(data[offset + 4..offset + 12].try_into().unwrap()).to_string()
                } else {
                    "0".to_string()
                };
                (k, 8)
            }
            2 => {
                let k = if offset + 4 <= data.len() {
                    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()).to_string()
                } else {
                    "0".to_string()
                };
                (k, 4)
            }
            7 => {
                let k = if offset + 4 <= data.len() {
                    let k_ptr = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    read_cstring(data, k_ptr)
                } else {
                    String::new()
                };
                (k, 4)
            }
            _ => {
                let k = if offset + 4 <= data.len() {
                    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()).to_string()
                } else {
                    "0".to_string()
                };
                (k, 4)
            }
        }
    }

    fn read_map_value(&self, data: &[u8], offset: usize, val_type: u8, val_hash: Option<[u8; 4]>) -> Value {
        match val_type {
            8 => {
                if offset + 4 <= data.len() {
                    let v_ptr = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    if let Some(h) = val_hash {
                        self.read_struct(data, v_ptr, &h)
                    } else {
                        Value::Null
                    }
                } else {
                    Value::Null
                }
            }
            7 => {
                if offset + 4 <= data.len() {
                    let v_ptr = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    Value::String(read_cstring(data, v_ptr))
                } else {
                    Value::String(String::new())
                }
            }
            0 => {
                if offset + 4 <= data.len() {
                    Value::Bool(u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) != 0)
                } else if offset < data.len() {
                    Value::Bool(data[offset] != 0)
                } else {
                    Value::Null
                }
            }
            1 => {
                if offset + 4 <= data.len() {
                    Value::Number((u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) & 0xFF).into())
                } else {
                    Value::Null
                }
            }
            _ => {
                self.read_value(data, offset, val_type, None, val_hash, None, None, None).0
            }
        }
    }
}

fn align4(offset: usize) -> usize {
    if offset % 4 != 0 {
        offset + 4 - (offset % 4)
    } else {
        offset
    }
}

fn read_cstring(data: &[u8], offset: usize) -> String {
    if offset == 0 || offset >= data.len() {
        return String::new();
    }
    let end = data[offset..].iter().position(|&b| b == 0).unwrap_or(data.len() - offset);
    String::from_utf8_lossy(&data[offset..offset + end]).to_string()
}

fn type_size_align(f_type: u8) -> (usize, usize) {
    match f_type {
        0 => (1, 1),
        1 => (1, 1),
        3 => (8, 8),
        5 => (8, 8),
        8 => (4, 4),
        10 => (4, 4),
        _ => (4, 4),
    }
}

fn field_type_size(f_type: u8) -> usize {
    match f_type {
        0 | 1 => 1,
        3 | 5 => 8,
        _ => 4,
    }
}

fn hex_encode(bytes: &[u8; 4]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// SparkBytes .bytes 文件解析器
pub struct SparkBytesParser;

impl SparkBytesParser {
    /// 解析 .bytes 文件数据并返回格式化的 JSON 字符串
    pub fn parse_to_json(data: &[u8]) -> String {
        let value = BytesParser::parse_bytes_file(data);
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
    }
}
