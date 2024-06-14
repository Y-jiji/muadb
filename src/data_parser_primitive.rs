use crate::data_schema::*;
use crate::data_buffer::*;

/*                                       */
/* Implementation of constant-sized type */
/*                                       */

macro_rules! ImplPrimitiveDataParser {($($X: ident: $Y: ty, )*) => {$(
    impl Primitive for $X {}
    impl SchemaParser<{Tag::$X as u8}> for $X {
        #[inline(always)]
        fn decode<'a>(_: SchemaRef<'a>) -> SchemaEnum<'a> {
            SchemaEnum::$X
        }
        fn encode<'a>(children: &[SchemaRef<'a>]) -> Schema {
            assert!(children.len() == 0);
            return Schema::from_primitive::<$X>();
        }
        fn scalar_layout<'a>(_: SchemaRef<'a>) -> ScalarLayout {
            let m = std::alloc::Layout::new::<$Y>();
            ScalarLayout {
                size: m.size(),
                align: m.align()
            }
        }
        fn dbg(_: SchemaRef<'_>, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(fmt, "{:?}", Tag::$X)
        }
        fn num_columns<'a>(_: SchemaRef<'a>) -> usize {
            1
        }
    }
    impl BufferParser<{Tag::$X as u8}> for $X {
        type ScalarRef<'a> = &'a $Y;
        type VectorRef<'a> = &'a [$Y];
        fn vector_cast<'a>(buffer: VBufRef<'a>) -> Self::VectorRef<'a> {
            bytemuck::cast_slice(buffer.buffer[buffer.buffer.len()-1].slice(..))
        }
        fn vector_push<'a>(buffer: VBufMut<'a>, elem: Self::ScalarRef<'a>) {
            buffer.buffer[buffer.buffer.len()-1].extend(&elem.to_ne_bytes());
        }
    }
)*};}

ImplPrimitiveDataParser! {
    I64: i64, I32: i32, I16: i16, I8: i8,
    U64: u64, U32: u32, U16: u16, U8: u8,
    F64: f64, F32: f32,
}

/*                                   */
/* Implementation of zero-sized type */
/*                                   */

impl Primitive for Nil {}
impl SchemaParser<{Tag::Nil as u8}> for Nil {
    #[inline(always)]
    fn decode<'a>(schema: SchemaRef<'a>) -> SchemaEnum<'a> {
        SchemaEnum::Nil
    }
    fn encode<'a>(children: &[SchemaRef<'a>]) -> Schema {
        assert!(children.len() == 0);
        return Schema::from_primitive::<Nil>()
    }
    fn scalar_layout<'a>(schema: SchemaRef) -> ScalarLayout {
        ScalarLayout{size: 0, align: 0}
    }
    fn dbg(_: SchemaRef<'_>, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{:?}", Tag::Nil)
    }
    fn num_columns<'a>(schema: SchemaRef) -> usize {
        1
    }
}

pub struct FlatNilRef(u64);
impl BufferParser<{Tag::Nil as u8}> for Nil {
    type ScalarRef<'a> = ();
    type VectorRef<'a> = FlatNilRef;
    fn vector_cast<'a>(buffer: VBufRef<'a>) -> Self::VectorRef<'a> {
        FlatNilRef(buffer.buffer[0].as_u64() as u64)
    }
    fn vector_push<'a>(buffer: VBufMut<'a>, _: Self::ScalarRef<'a>) {
        buffer.buffer[0].add(1);
    }
}

/*                            */
/* Implementation of pad type */
/*                            */

impl BufferParser<{Tag::Pad as u8}> for Pad {
    type ScalarRef<'a> = ();
    type VectorRef<'a> = FlatNilRef;
}

impl SchemaParser<{Tag::Pad as u8}> for Pad {
    fn decode<'a>(schema: SchemaRef<'a>) -> SchemaEnum<'a> {
        schema.decode()
    }
    fn encode<'a>(children: &[SchemaRef<'a>]) -> Schema {
        panic!("you should never construct a schema with type 'pad'");
    }
    fn scalar_layout<'a>(schema: SchemaRef<'a>) -> ScalarLayout {
        panic!("you should never construct a schema with type 'pad', so there function will never be called");
    }
    fn dbg(_: SchemaRef<'_>, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        panic!("you should never construct a schema with type 'pad', so there function will never be called");
    }
    fn num_columns<'a>(schema: SchemaRef) -> usize {
        panic!("you should never construct a schema with type 'pad'");
    }
}