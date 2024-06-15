use crate::{data_parser_primitive::FlatNilRef, data_schema::*, data_buffer::*};

/*                             */
/* Implementation of pair type */
/*                             */

impl DSchemaParser<{Tag::Pair as u8}> for Pair {
    #[inline(always)]
    fn decode<'a>(schema: DSchemaRef<'a>) -> DSchemaEnum<'a> {
        // (a, b): the offset of types
        let a = schema.u16() as usize;
        let schema = schema.cut(2);
        let b = schema.u16() as usize;
        let schema = schema.cut(2);
        // sum of children type sizes
        let m = schema.u64();
        let m = ScalarLayout{size: m as usize / 256, align: m as usize % 256};
        let schema = schema.cut(8);
        // the rest of schema
        DSchemaEnum::Pair(m, schema.slice(a..b), schema.slice(0..a))
    }
    fn encode<'a>(children: &[DSchemaRef]) -> DSchema {
        assert!(children.len() == 2);
        let a = children[0].len();
        let b = children[1].len();
        let l0 = children[0].scalar_layout();
        let l1 = children[1].scalar_layout();
        let m = ScalarLayout{
            size: (l0.size + (1 << l1.align) - 1) & !((1 << l1.align) - 1) + l1.size, 
            align: l0.align.max(l1.align)
        };
        let m = m.size * 256 + m.align;
        let mut schema = DSchema::empty();
        schema.join(children[0]);
        schema.join(children[1]);
        schema.put(m as u64);
        schema.put(b as u16 + a as u16);
        schema.put(a as u16);
        schema.put(Tag::Pair as u8);
        schema
    }
    fn scalar_layout<'a>(schema: DSchemaRef) -> ScalarLayout {
        let m = schema.cut(8).u64();
        ScalarLayout{size: m as usize / 256, align: m as usize % 256}
    }
    fn dbg(schema: DSchemaRef<'_>, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let DSchemaEnum::Pair(_, a, b) = Self::decode(schema) else { unreachable!() };
        write!(fmt, "Pair({b:?}, {a:?})")
    }
    fn num_columns<'a>(schema: DSchemaRef) -> usize {
        let DSchemaEnum::Pair(_, a, b) = Self::decode(schema) else { unreachable!() };
        a.num_columns() + b.num_columns()
    }
}

// TODO: columnar & scalar implementation (with SmallVec?)
pub struct FlatPairRef<'a>(pub VBufRef<'a>, pub VBufRef<'a>);

impl BufferParser<{Tag::Pair as u8}> for Pair {
    type ScalarRef<'a> = ();
    type VectorRef<'a> = FlatNilRef;
}