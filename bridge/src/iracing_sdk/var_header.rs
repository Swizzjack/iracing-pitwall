//! irsdk_varHeader: descriptor for each telemetry variable.
//!
//! Layout: 144 bytes per entry, numVars entries starting at header.var_header_offset.

use std::collections::HashMap;

pub const IRSDK_MAX_STRING: usize = 32;
pub const IRSDK_MAX_DESC: usize = 64;
pub const VAR_HEADER_SIZE: usize = 144;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarType {
    Char = 0,
    Bool = 1,
    Int = 2,
    BitField = 3,
    Float = 4,
    Double = 5,
}

impl VarType {
    pub fn size_bytes(self) -> usize {
        match self {
            VarType::Char | VarType::Bool => 1,
            VarType::Int | VarType::BitField | VarType::Float => 4,
            VarType::Double => 8,
        }
    }

    pub fn from_i32(n: i32) -> Option<Self> {
        match n {
            0 => Some(VarType::Char),
            1 => Some(VarType::Bool),
            2 => Some(VarType::Int),
            3 => Some(VarType::BitField),
            4 => Some(VarType::Float),
            5 => Some(VarType::Double),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VarDescriptor {
    pub var_type: VarType,
    pub offset: usize,
    pub count: usize,
    pub name: String,
    pub desc: String,
    pub unit: String,
}

/// Name → descriptor lookup. Built once after connecting.
pub type VarIndex = HashMap<String, VarDescriptor>;

/// Helper: Extract a null-terminated C-style UTF-8 string from bytes.
fn cstr_from_bytes(b: &[u8]) -> crate::error::Result<String> {
    let len = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let prefix = &b[..len];
    std::str::from_utf8(prefix)
        .map(|s| s.to_owned())
        .map_err(|e| crate::error::BridgeError::SdkRead(format!("UTF-8: {e}")))
}

/// Parses the varHeader array from the MMF slice.
///
/// # Arguments
/// * `raw` - Slice starting at `header.var_header_offset`, length ≥ `num_vars * 144`.
/// * `num_vars` - Number of entries (from the top-level header).
pub fn parse_var_index(raw: &[u8], num_vars: usize) -> crate::error::Result<VarIndex> {
    let expected_len = num_vars.checked_mul(VAR_HEADER_SIZE).ok_or_else(|| {
        crate::error::BridgeError::SdkRead(format!(
            "var_header length overflow: {} * {}",
            num_vars, VAR_HEADER_SIZE
        ))
    })?;

    if raw.len() < expected_len {
        return Err(crate::error::BridgeError::SdkRead(format!(
            "var_header slice too short: {} < {}",
            raw.len(),
            expected_len
        )));
    }

    let mut var_index = HashMap::with_capacity(num_vars);

    for i in 0..num_vars {
        let record_start = i * VAR_HEADER_SIZE;
        let record = &raw[record_start..record_start + VAR_HEADER_SIZE];

        // Read i32 fields (little-endian, Windows x86_64)
        let type_bytes: [u8; 4] = [record[0], record[1], record[2], record[3]];
        let offset_bytes: [u8; 4] = [record[4], record[5], record[6], record[7]];
        let count_bytes: [u8; 4] = [record[8], record[9], record[10], record[11]];

        let var_type_val = i32::from_le_bytes(type_bytes);
        let var_offset = i32::from_le_bytes(offset_bytes);
        let var_count = i32::from_le_bytes(count_bytes);

        let var_type = VarType::from_i32(var_type_val).ok_or_else(|| {
            crate::error::BridgeError::SdkRead(format!(
                "invalid var_type {} at index {}",
                var_type_val, i
            ))
        })?;

        // Extract strings (trimmed at null bytes).
        // Layout after the 16-byte numeric block: name[32] desc[64] unit[32].
        let name = cstr_from_bytes(&record[16..16 + IRSDK_MAX_STRING])?;
        let desc = cstr_from_bytes(&record[48..48 + IRSDK_MAX_DESC])?;
        let unit = cstr_from_bytes(&record[112..112 + IRSDK_MAX_STRING])?;

        // Skip padding records (empty name)
        if name.is_empty() {
            continue;
        }

        let descriptor = VarDescriptor {
            var_type,
            offset: var_offset as usize,
            count: var_count as usize,
            name: name.clone(),
            desc,
            unit,
        };

        // Keep first occurrence, warn about duplicates.
        // Entry API avoids the HashMap::insert "always overwrites" semantics.
        match var_index.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => {
                log::warn!(
                    "Duplicate var name '{}' found, keeping first occurrence",
                    name
                );
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(descriptor);
            }
        }
    }

    Ok(var_index)
}
