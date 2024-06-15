use crate::data_schema::*;
use crate::data_buffer::*;

//                            //
// Implementation of str type //
//                            //

impl Primitive for Str {}

impl DSchemaParser<{Tag::Str as u8}> for Str {
    fn decode<'a>(_: DSchemaRef<'a>) -> DSchemaEnum<'a> {
        DSchemaEnum::Str
    }
    fn encode<'a>(children: &[DSchemaRef<'a>]) -> DSchema {
        assert!(children.len() == 0);
        return DSchema::from_primitive::<Str>()
    }
    fn scalar_layout<'a>(_: DSchemaRef<'a>) -> ScalarLayout {
        let m = std::alloc::Layout::new::<&str>();
        ScalarLayout {
            size: m.size(),
            align: m.align()
        }
    }
    fn num_columns<'a>(_: DSchemaRef<'a>) -> usize {
        2
    }
}

impl BufferParser<{Tag::Str as u8}> for Str {
    type ScalarRef<'a> = &'a str;
    type VectorRef<'a> = FlatStr<'a>;
    fn vector_cast<'a>(buffer: VBufRef<'a>) -> Self::VectorRef<'a> {
        assert!(buffer.buffer.len() == 2);
        let buffer = buffer.buffer;
        FlatStr {
            offset: bytemuck::cast_slice(buffer[1].slice(..)),
            buffer: unsafe { std::str::from_utf8_unchecked(buffer[0].slice(..)) },
        }
    }
    fn vector_push<'a>(buffer: VBufMut<'a>, elem: &str) {
        assert!(buffer.buffer.len() == 2);
        buffer.buffer[0].extend(elem.as_bytes());
        buffer.buffer[1].extend(&(buffer.buffer[0].len() as u64).to_ne_bytes());
    }
}

pub struct FlatStr<'a> {
    offset: &'a [u64],
    buffer: &'a str,
}

