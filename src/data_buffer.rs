use crate::{
    util_bytes::Bytes, 
    data_schema::*,
};

/*                                 */
/* Implementation of scalar buffer */
/*                                 */

/// Generic Scalar Buffer
pub struct SBuf {
    pub buffer: Bytes,
    pub schema: DSchema,
}

impl SBuf {
    pub fn new(schema: DSchema) -> Self {
        macro_rules! Match {($($X: ident,)*) => {{
            let schema_ref = schema.as_ref();
            match Tag::from(schema_ref.tag()) {
                $(Tag::$X => $X::scalar_layout(schema_ref.cut(1)), )*
            }
        }};}
        let size = crate::Fill!{Match{<Here>}};
        let mut buffer = Bytes::new();
        todo!()
    }
}

/// Immutable Reference of Generic Scalar Buffer
pub struct SBufRef<'a> {
    pub buffer: &'a [u8],
    pub schema: DSchemaRef<'a>,
}

/// 
pub trait SBufParser<const TAG: u8> {
    type ScalarRef<'a>;
    fn scalar_cast<'a>(buf: SBufRef<'a>) -> Self::ScalarRef<'a>;
}

/*                                 */
/* Implementation of vector buffer */
/*                                 */

/// Generic Vector Buffer
pub struct VBuf {
    pub buffer: Vec<Bytes>,
    pub schema: DSchema,
}

impl VBuf {
    pub fn new(schema: DSchema) -> VBuf {
        macro_rules! Match {($($X: ident,)*) => {{
            let schema_ref = schema.as_ref();
            match Tag::from(schema_ref.tag()) {
                $(Tag::$X => $X::num_columns(schema_ref.cut(1)), )*
            }
        }};}
        let column = crate::Fill!{Match{<Here>}};
        let buffer = (0..column).map(|_| Bytes::new()).collect();
        VBuf {schema, buffer}
    }
}

/// Mutable Reference of Generic Vector Buffer
pub struct VBufMut<'a> {
    pub buffer: &'a mut [Bytes],
    pub schema: DSchemaRef<'a>,
}

/// Immutable Reference of Generic Vector Buffer
#[derive(Clone, Copy)]
pub struct VBufRef<'a> {
    pub buffer: &'a [Bytes],
    pub schema: DSchemaRef<'a>,
}

pub trait BufferParser<const TAG: u8> {
    type VectorRef<'a>;
    type ScalarRef<'a>;
    fn vector_cast<'a>(buffer: VBufRef<'a>) -> Self::VectorRef<'a> { todo!() }
    fn vector_push<'a>(buffer: VBufMut<'a>, elem: Self::ScalarRef<'a>) { todo!() }
    fn vector_tail<'a>(buffer: VBufMut<'a>, elem: SBuf) -> SBuf { todo!() }
}