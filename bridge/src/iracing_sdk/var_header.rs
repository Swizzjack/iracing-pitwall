//! irsdk_varHeader: Descriptor für jede Telemetrie-Variable.
//!
//! Layout: 144 Bytes pro Eintrag, numVars Einträge ab header.var_header_offset.

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

/// Name → Descriptor Lookup. Wird einmal nach Connect aufgebaut.
pub type VarIndex = HashMap<String, VarDescriptor>;

/// Parst das varHeader-Array aus dem MMF-Slice.
///
/// # Arguments
/// * `raw` - Slice beginnend bei `header.var_header_offset`, Länge ≥ `num_vars * 144`.
/// * `num_vars` - Anzahl Einträge (aus Top-Level-Header).
pub fn parse_var_index(_raw: &[u8], _num_vars: usize) -> VarIndex {
    // TODO: Für jeden Eintrag 144 Bytes lesen, null-terminated Strings extrahieren,
    // VarDescriptor in HashMap mit name als Key einfügen.
    todo!("parse var_header array")
}
