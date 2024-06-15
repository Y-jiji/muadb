// SQLSchema
pub enum SQLSchema<'a> {
    NamedTuple {
        name:  &'a[&'a str],
        tuple: &'a[SQLSchema<'a>]
    },
    Tuple {
        tuple: &'a[SQLSchema<'a>]
    },
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    Nil, F32, F64, Str, 
}

