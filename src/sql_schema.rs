use bumpalo::collections::Vec as BVec;
use ciborium::*;
use serde::*;
use serde::de::DeserializeSeed;

// SQLSchema
#[derive(Debug, Clone, Serialize)]
#[repr(u8)]
pub enum SQLSchema<'a> {
    NamedTuple {
        name:  BVec<'a, &'a str>,
        tuple: BVec<'a, SQLSchema<'a>>,
    },
    Tuple {
        tuple: BVec<'a, SQLSchema<'a>>
    },
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    Nil, F32, F64, Str, 
}

