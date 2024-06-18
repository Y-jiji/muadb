use std::fmt::Debug;

use crate::*;

#[derive(Clone)]
pub struct Schema(Bytes);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SchemaRef<'a>(&'a [u8]);

/// representing a list of (name, schema)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NamedSlice<'a>(&'a [u16], &'a [u8]);

/// representing a list of schema (anony -> anonymous)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnonySlice<'a>(&'a [u16], &'a [u8]);

/// Ultimately, we want to read our schema in a zero-allocation style. 
/// However, we cannot implement tree-like structures without allocation!
/// Therefore, we decode it one layer at a time. 
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SchemaEnum<'a, Named, Tuple> where 
    Named: IntoIterator<Item = (&'a str, SchemaRef<'a>)>,
    Tuple: IntoIterator<Item = SchemaRef<'a>>,
{
    // Typical Primitive Types
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    F32, F64, 
    // Special Primitive Types
    Nil, Str,
    // Tuple & Struct
    Struct(Named), 
    Tuple(Tuple),
    // Enumeration Type
    Enum(Named),
    // Array Type
    Array(SchemaRef<'a>), 
    // Default Type
    Default, 
}

/// encode a schema enum into Self
impl Schema {
    pub fn encode<'a, N, T>(schema: SchemaEnum<'a, N, T>) -> Self where
        N: IntoIterator<Item = (&'a str, SchemaRef<'a>)> + Clone,
        T: IntoIterator<Item = SchemaRef<'a>> + Clone
    {
        let mut owner = Schema(Bytes::new());
        macro_rules! Match {($($X: ident, )*) => {
            match schema {$(
                SchemaEnum::$X{..} => $X::encode(schema, &mut owner),
            )*}
        };}
        crate::TyXMacro!{Match{<Here>}}; owner
    }
    pub fn as_ref(&self) -> SchemaRef<'_> {
        SchemaRef(self.0.slice(..))
    }
    fn join<'a>(&mut self, schema: SchemaRef<'a>) {
        self.0.extend(schema.0)
    }
    fn join_str(&mut self, s: &str) {
        self.0.extend(s.as_bytes())
    }
    fn put<T: bytemuck::NoUninit>(&mut self, v: T) {
        self.0.pad(std::mem::size_of::<T>(), Default::NUM);
        self.0.extend(bytemuck::bytes_of(&v));
    }
}

impl Debug for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

/// Implement schema encoding & decoding by dispatching tags. 
impl<'a> SchemaRef<'a> {
    /// Decode a schema for one layer
    /// See [`Dispatch`] for implementation
    pub fn decode(self) -> SchemaEnum<'a, NamedSlice<'a>, AnonySlice<'a>> {
        assert!(self.len() != 0, "empty schema cannot be a valid schema");
        // declare a branch for each variant
        macro_rules! Match {($($X: ident, )*) => {
            match self.u8() {
                $($X::NUM => {$X::decode(self.cut(1))})*
                _ => unreachable!()
            }
        };}
        crate::TyXMacro!{Match{<Here>}}
    }
    /// Internal bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    fn len(self) -> usize {
        return self.0.len()
    }
    fn u8(self) -> u8 {
        *self.0.last().unwrap()
    }
    fn u16(self) -> u16 {
        u16::from_ne_bytes(unsafe{self.0[self.0.len()-2..].try_into().unwrap_unchecked()})
    }
    fn cut(self, l: usize) -> Self {
        SchemaRef(&self.0[..self.0.len()-l])
    }
    fn slice(self, range: std::ops::Range<usize>) -> SchemaRef<'a> {
        SchemaRef(&self.0[range])
    }
}

impl<'a> std::fmt::Debug for SchemaRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.decode())
    }
}

impl<'a> Iterator for NamedSlice<'a> {
    type Item = (&'a str, SchemaRef<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.len() < 3 { None? }
        let x = self.0[0] as usize;
        let y = self.0[1] as usize;
        let z = self.0[2] as usize;
        self.0 = &self.0[2..];
        Some((std::str::from_utf8(&self.1[x..y]).unwrap(), SchemaRef(&self.1[y..z])))
    }
}

impl<'a> std::fmt::Debug for NamedSlice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.into_iter()).finish()
    }
}

impl<'a> Iterator for AnonySlice<'a> {
    type Item = SchemaRef<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.len() < 2 { None? }
        let x = self.0[0] as usize;
        let y = self.0[1] as usize;
        self.0 = &self.0[1..];
        Some(SchemaRef(&self.1[x..y]))
    }
}

impl<'a> std::fmt::Debug for AnonySlice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.into_iter()).finish()
    }
}

/// Dispatch a schema
trait Dispatch {
    /// Decode a schema by one layer
    fn decode<'a>(schema: SchemaRef<'a>) -> SchemaEnum<'a, NamedSlice<'a>, AnonySlice<'a>>;
    /// Encode a schema enum into schema
    fn encode<'a, N, T>(option: SchemaEnum<'a, N, T>, schema: &mut Schema) where
        N: IntoIterator<Item = (&'a str, SchemaRef<'a>)> + Clone,
        T: IntoIterator<Item = SchemaRef<'a>> + Clone;
}

/// Implement Primitive Types
macro_rules! PrimitiveDispatch {($($X: ident, )*) => {$(
    impl Dispatch for $X {
        /// Primitive don't contain children types. 
        /// Getting dispatched into this branch means we already know the type. 
        /// Therefore we just return the type tag. 
        fn decode<'a>(_: SchemaRef<'a>) -> SchemaEnum<'a, NamedSlice<'a>, AnonySlice<'a>> {
            SchemaEnum::$X
        }
        /// Encode a schema by one layer. 
        fn encode<'a, N, T>(_: SchemaEnum<'a, N, T>, schema: &mut Schema) where
            N: IntoIterator<Item = (&'a str, SchemaRef<'a>)>,
            T: IntoIterator<Item = SchemaRef<'a>>
        {
            schema.0.push($X::NUM);
        }
    }
    impl From<$X> for SchemaRef<'static> {
        /// Declare a static object for each schema type
        fn from(_: $X) -> Self {
            SchemaRef(&[$X::NUM])
        }
    }
)*};}

PrimitiveTyXMacro!{PrimitiveDispatch{<Here>}}

/// Implement [`NamedSlice`] Types
macro_rules! NamedSliceDispatch {($($T: ident, )*) => {$(
    impl Dispatch for $T {
        fn decode<'a>(schema: SchemaRef<'a>) -> SchemaEnum<'a, NamedSlice<'a>, AnonySlice<'a>> {
            let len = schema.u16() as usize;
            let schema = schema.cut(2);
            let cuts = schema.slice(schema.len()-len*4-2..schema.len());
            let data = schema.cut(len*4+2);
            SchemaEnum::$T(NamedSlice(bytemuck::cast_slice(cuts.0), data.0))
        }
        fn encode<'a, N, T>(schema: SchemaEnum<'a, N, T>, owner: &mut Schema) where
            N: IntoIterator<Item = (&'a str, SchemaRef<'a>)> + Clone,
            T: IntoIterator<Item = SchemaRef<'a>> + Clone
        {
            let SchemaEnum::$T(named) = schema else { unreachable!() };
            let mut start = owner.0.len();
            for (name, ty) in named.clone() {
                owner.join_str(name);
                owner.join(ty);
            }
            let mut count: u16 = 0;
            owner.put(0u16);
            for (name, ty) in named {
                let x = start + name.len(); start = x;
                let y = start + ty.0.len(); start = y;
                // Safety: some nuts may actually construct super long names. 
                owner.put(x as u16);
                owner.put(y as u16);
                count += 1;
            }
            owner.0.extend(&count.to_ne_bytes());
            owner.0.push($T::NUM);
            owner.0.pad(2, Default::NUM);
        }
    }
)*};}

NamedSliceDispatch!{Enum, Struct, }

/// Implement [`AnonySlice`] Types
impl Dispatch for Tuple {
    fn decode<'a>(schema: SchemaRef<'a>) -> SchemaEnum<'a, NamedSlice<'a>, AnonySlice<'a>> {
        let len = schema.u16() as usize;
        let schema = schema.cut(2);
        let cuts = schema.slice(schema.len()-2*len-2..schema.len());
        let data = schema.cut(2*len+2);
        SchemaEnum::Tuple(AnonySlice(bytemuck::cast_slice(cuts.0), data.0))
    }
    fn encode<'a, N, T>(schema: SchemaEnum<'a, N, T>, owner: &mut Schema) where
        N: IntoIterator<Item = (&'a str, SchemaRef<'a>)> + Clone,
        T: IntoIterator<Item = SchemaRef<'a>> + Clone,
    {
        let SchemaEnum::Tuple(tuple) = schema else { unreachable!() };
        let mut start = owner.0.len();
        for ty in tuple.clone() {
            owner.join(ty);
        }
        let mut count: u16 = 0;
        owner.put(0 as u16);
        for ty in tuple {
            start = start + ty.0.len();
            owner.put(start as u16);
            count += 1;
        }
        owner.put(count as u16);
        owner.0.push(Tuple::NUM);
        owner.0.pad(2, Default::NUM);
    }
}

impl Dispatch for Array {
    fn decode<'a>(schema: SchemaRef<'a>) -> SchemaEnum<'a, NamedSlice<'a>, AnonySlice<'a>> {
        SchemaEnum::Array(schema)
    }
    fn encode<'a, N, T>(_: SchemaEnum<'a, N, T>, owner: &mut Schema) where
        N: IntoIterator<Item = (&'a str, SchemaRef<'a>)>,
        T: IntoIterator<Item = SchemaRef<'a>>
    {
        owner.0.push(Array::NUM);
    }
}

impl Dispatch for Default {
    fn decode<'a>(schema: SchemaRef<'a>) -> SchemaEnum<'a, NamedSlice<'a>, AnonySlice<'a>> {
        schema.decode()
    }
    fn encode<'a, N, T>(_: SchemaEnum<'a, N, T>, _: &mut Schema) where
        N: IntoIterator<Item = (&'a str, SchemaRef<'a>)>,
        T: IntoIterator<Item = SchemaRef<'a>>
    {
        panic!("You cannot really to this. Because `Default` is used for padding. ")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tuple_serde() {
        // This case will fail if children schema is not aligned. 
        // That is the purpose of that last line of Tuple::encode. 
        let schema = SchemaEnum::<NamedSlice, _>::Tuple([
            SchemaRef::from(I16), 
            SchemaRef::from(I32), 
            SchemaRef::from(F32),
        ]);
        let schema = Schema::encode(schema);
        let schema = SchemaEnum::<NamedSlice, _>::Tuple([
            SchemaRef::from(I16), 
            SchemaRef::from(I32), 
            schema.as_ref(),
            schema.as_ref()
        ]);
        let schema = Schema::encode(schema);
        println!("{schema:?}");
    }

    #[test]
    fn struct_serde() {
        let schema = SchemaEnum::<_, AnonySlice>::Struct([
            ("i16", SchemaRef::from(I16)), 
            ("i32", SchemaRef::from(I32)), 
        ]);
        let schema = Schema::encode(schema);
        assert!(format!("{schema:?}") == r#"Struct({"i16": I16, "i32": I32})"#);
    }

}