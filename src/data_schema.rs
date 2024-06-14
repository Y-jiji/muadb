use bytemuck::NoUninit;
use crate::util_bytes::Bytes;

#[macro_export]
/// This macro declares all types in this data base system. 
macro_rules! Fill {
    ($Macro: ident { <Here> }) => {
        $Macro! {
            I64, I32, I16, I8,
            U64, U32, U16, U8,
            F32, F64, Nil, Str,
            Pair, Pad, 
        }
        // Str, Pair, List, Union, Pad, 
    };
}

pub trait Primitive {}

pub struct Schema(Bytes);
impl Schema {
    pub fn empty() -> Schema {
        Schema(Bytes::new())
    }
    pub fn from_primitive<T: Primitive + NumOf>() -> Schema {
        let mut schema = Bytes::new();
        schema.push(T::NUM);
        Schema(schema)
    }
    pub fn as_ref(&self) -> SchemaRef<'_> {
        SchemaRef(self.0.slice(..))
    }
    pub fn join(&mut self, schema: SchemaRef) {
        self.0.extend(schema.0)
    }
    pub fn put<T: NoUninit>(&mut self, v: T) {
        self.0.pad(std::mem::size_of::<T>());
        self.0.extend(bytemuck::bytes_of(&v));
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SchemaRef<'a>(&'a [u8]);
pub enum SchemaEnum<'a> {
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    Nil, F32, F64, Str, 
    List(SchemaRef<'a>),
    Enum(u32, &'a [u16], SchemaRef<'a>),
    Pair(ScalarLayout, SchemaRef<'a>, SchemaRef<'a>),
}

impl<'a> std::fmt::Debug for SchemaRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        macro_rules! Match {($($X: ident, )*) => {
            match self.tag() {
                $($X::NUM => $X::dbg(self.cut(1), f), )*
                _ => unreachable!()
            }
        };}
        crate::Fill!{Match{<Here>}}
    }
}

pub struct ScalarLayout{pub size: usize, pub align: usize}

impl<'a> SchemaRef<'a> {
    pub fn decode(self) -> SchemaEnum<'a> {
        // declare a branch for each variant
        macro_rules! Match {($($X: ident, )*) => {
            match self.tag() {
                $($X::NUM => $X::decode(self.cut(1)), )*
                _ => unreachable!()
            }
        };}
        crate::Fill!{Match{<Here>}}
    }
    pub fn len(self) -> usize {
        return self.0.len()
    }
    pub fn num_columns(&self) -> usize {
        // declare a branch for each variant
        macro_rules! Match {($($X: ident, )*) => {
            match self.tag() {
                $($X::NUM => $X::num_columns(self.cut(1)), )*
                _ => unreachable!()
            }
        };}
        crate::Fill!{Match{<Here>}}
    }
    pub fn scalar_layout(self) -> ScalarLayout {
        // declare a branch for each variant
        macro_rules! Match {($($X: ident, )*) => {
            match self.tag() {
                $($X::NUM => $X::scalar_layout(self.cut(1)), )*
                _ => unreachable!()
            }
        };}
        crate::Fill!{Match{<Here>}}
    }
    pub fn tag(self) -> u8 {
        *self.0.last().unwrap()
    }
    pub fn u16(self) -> u16 {
        u16::from_ne_bytes(unsafe{self.0[self.0.len()-2..].try_into().unwrap_unchecked()})
    }
    pub fn u32(self) -> u32 {
        u32::from_ne_bytes(unsafe{self.0[self.0.len()-4..].try_into().unwrap_unchecked()})
    }
    pub fn u64(self) -> u64 {
        u64::from_ne_bytes(unsafe{self.0[self.0.len()-8..].try_into().unwrap_unchecked()})
    }
    pub fn cut(self, l: usize) -> Self {
        SchemaRef(&self.0[..self.0.len()-l])
    }
    pub fn slice(self, range: std::ops::Range<usize>) -> SchemaRef<'a> {
        SchemaRef(&self.0[range])
    }
}

/// [`SchemaParser`] decode schema to an enumeration
pub trait SchemaParser<const TAG: u8> {
    fn decode<'a>(schema: SchemaRef<'a>) -> SchemaEnum<'a> { 
        todo!("SchemaParser::decode for {:?}", Tag::from(TAG));
    }
    fn encode<'a>(children: &[SchemaRef]) -> Schema { 
        todo!("SchemaParser::encode for {:?}", Tag::from(TAG));
    }
    fn scalar_layout<'a>(schema: SchemaRef) -> ScalarLayout {
        todo!("SchemaParser::scalar_layout for {:?}", Tag::from(TAG));
    }
    fn dbg(schema: SchemaRef<'_>, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!("SchemaParser::dbg for {:?}", Tag::from(TAG));
    }
    fn num_columns<'a>(schema: SchemaRef) -> usize {
        todo!("SchemaParser::num_columns for {:?}", Tag::from(TAG));
    }
}

pub trait NumOf { const NUM: u8; }
macro_rules! DeclareTagEnum {
    ($($Tag: ident, )*) => {
        DeclareTagEnum! {<Progression Mode> $($Tag, )* }
        #[derive(Clone, Copy, Debug)]
        #[repr(u8)]
        pub enum Tag { $($Tag = $Tag::NUM, )* }
        impl From<u8> for Tag {
            fn from(value: u8) -> Tag {
                match value {
                    $($Tag::NUM => Tag::$Tag, )*
                    _ => unreachable!()
                }
            }
        }
    };
    (<Progression Mode>) => {};
    (<Progression Mode> $X: ident, $Y: ident, $($Tag: ident, )*) => {
        pub struct $X;
        impl NumOf for $X { const NUM: u8 = $Y::NUM + 1; }
        impl Into<u8> for $X { fn into(self) -> u8 { return $X::NUM; } }
        DeclareTagEnum!{<Progression Mode> $Y, $($Tag, )* }
    };
    (<Progression Mode> $X: ident, ) => {
        pub struct $X;
        impl Into<u8> for $X { fn into(self) -> u8 { return $X::NUM; } }
        impl NumOf for $X { const NUM: u8 = 0; }
    }
}
crate::Fill!{DeclareTagEnum{<Here>}}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pad_num() {
        assert!(Pad::NUM == 0);
    }

    #[test]
    fn encode_decode() {
        let i32 = I32::encode(&[]);
        let i64 = I64::encode(&[]);
        let f64 = F64::encode(&[]);
        let i32 = i32.as_ref();
        let i64 = i64.as_ref();
        let f64 = f64.as_ref();
        let pair = Pair::encode(&[Pair::encode(&[i32, i64]).as_ref(), f64]);
        assert!("Pair(Pair(I32, I64), F64)" == format!("{:?}", pair.as_ref()));
    }
}