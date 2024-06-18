/// This macro declares all types (and type functions) in this database system. 
/// In other programming languages like c++, this technique is often refered to as XMacro. 
#[macro_export]
macro_rules! TyXMacro {
    ($Macro: ident { <Here> }) => {
        $Macro! {
            // Typical Primitive Types
            I64, I32, I16, I8,
            U64, U32, U16, U8,
            F32, F64, 
            // Special Primitive Types
            Nil, Str,
            // Tuple & Struct
            Struct, Tuple,
            // Enumeration Type
            Enum,
            // Array Type
            Array, 
            // Default Type
            Default, 
        }
    };
}

/// This macro declares all primitive types in this database system. 
#[macro_export]
macro_rules! PrimitiveTyXMacro {
    ($Macro: ident { <Here> }) => {
        $Macro! {
            // Typical Primitive Types
            I64, I32, I16, I8,
            U64, U32, U16, U8,
            F32, F64, 
            // Special Primitive Types
            Nil, Str,
        }
    };
}

/// The following code declares a NUM for each Ty. 
/// You can use $Ty::NUM => $Ty to implement functions that only applies to a specific enum. 
pub trait NumOf { const NUM: u8; }
macro_rules! DeclareTy {
    ($($Ty: ident, )*) => {
        DeclareTy! {<Progression Mode> $($Ty, )* }
        #[derive(Clone, Copy, Debug)]
        #[repr(u8)]
        pub enum Ty { $($Ty = $Ty::NUM, )* }
        impl From<u8> for Ty {
            fn from(value: u8) -> Ty {
                match value {
                    $($Ty::NUM => Ty::$Ty, )*
                    _ => unreachable!()
                }
            }
        }
    };
    (<Progression Mode>) => {};
    (<Progression Mode> $X: ident, $Y: ident, $($Ty: ident, )*) => {
        #[derive(Debug)]
        pub struct $X;
        impl NumOf for $X { const NUM: u8 = $Y::NUM + 1; }
        impl Into<u8> for $X { fn into(self) -> u8 { return $X::NUM; } }
        DeclareTy!{<Progression Mode> $Y, $($Ty, )* }
    };
    (<Progression Mode> $X: ident, ) => {
        #[derive(Debug)]
        pub struct $X;
        impl Into<u8> for $X { fn into(self) -> u8 { return $X::NUM; } }
        impl NumOf for $X { const NUM: u8 = 0; }
    }
}
crate::TyXMacro!{DeclareTy{<Here>}}