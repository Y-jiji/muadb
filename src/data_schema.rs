use bytemuck::NoUninit;
use muadb_util::Bytes;

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

pub struct DSchema(Bytes);
impl DSchema {
    pub fn empty() -> DSchema {
        DSchema(Bytes::new())
    }
    pub fn from_primitive<T: Primitive + NumOf>() -> DSchema {
        let mut schema = Bytes::new();
        schema.push(T::NUM);
        DSchema(schema)
    }
    pub fn as_ref(&self) -> DSchemaRef<'_> {
        DSchemaRef(self.0.slice(..))
    }
    pub fn join(&mut self, schema: DSchemaRef) {
        self.0.extend(schema.0)
    }
    pub fn put<T: NoUninit>(&mut self, v: T) {
        self.0.pad(std::mem::size_of::<T>());
        self.0.extend(bytemuck::bytes_of(&v));
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DSchemaRef<'a>(&'a [u8]);
pub enum DSchemaEnum<'a> {
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    Nil, F32, F64, Str, 
    List(DSchemaRef<'a>),
    Enum(u32, &'a [u16], DSchemaRef<'a>),
    Pair(ScalarLayout, DSchemaRef<'a>, DSchemaRef<'a>),
}

impl<'a> std::fmt::Debug for DSchemaRef<'a> {
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

impl<'a> DSchemaRef<'a> {
    pub fn decode(self) -> DSchemaEnum<'a> {
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
        DSchemaRef(&self.0[..self.0.len()-l])
    }
    pub fn slice(self, range: std::ops::Range<usize>) -> DSchemaRef<'a> {
        DSchemaRef(&self.0[range])
    }
}

/// [`DSchemaParser`] decode schema to an enumeration
pub trait DSchemaParser<const TAG: u8> {
    fn decode<'a>(schema: DSchemaRef<'a>) -> DSchemaEnum<'a> { 
        todo!("DSchemaParser::decode for {:?}", Tag::from(TAG));
    }
    fn encode<'a>(children: &[DSchemaRef]) -> DSchema { 
        todo!("DSchemaParser::encode for {:?}", Tag::from(TAG));
    }
    fn scalar_layout<'a>(schema: DSchemaRef) -> ScalarLayout {
        todo!("DSchemaParser::scalar_layout for {:?}", Tag::from(TAG));
    }
    fn dbg(schema: DSchemaRef<'_>, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!("DSchemaParser::dbg for {:?}", Tag::from(TAG));
    }
    fn num_columns<'a>(schema: DSchemaRef) -> usize {
        todo!("DSchemaParser::num_columns for {:?}", Tag::from(TAG));
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