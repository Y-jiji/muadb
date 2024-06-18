use crate::*;
use casey::lower;

/// size and alignment of a type
pub struct Layout{pub size: u16, pub align: u16}

/// Declare & Seal Primitive Type Mark
mod seal {pub trait Seal {}}
pub trait Primitive: seal::Seal {
    type Me: ?Sized;
    fn layout() -> Layout;
}

macro_rules! DeclarePrimitive {($($Ty: ident, )*) => {
    #[allow(non_camel_case_types)]
    type nil = ();
    trait Relay {
        fn lay() -> Layout;
    }
    trait Lay {
        fn lay() -> Layout;
    }
    impl<T> Lay for T where T: Sized {
        fn lay() -> Layout {
            let lay = std::alloc::Layout::new::<T>();
            Layout{size: lay.size() as u16, align: lay.align() as u16}
        }
    }
    /// we need simply because [`str`] is not sized
    /// so we have to manually implement it
    impl Relay for str {
        fn lay() -> Layout {
            let lay = std::alloc::Layout::new::<&str>();
            Layout{size: lay.size() as u16, align: lay.align() as u16}
        }
    }
    $(
        impl seal::Seal for $Ty {}
        impl Primitive for $Ty {
            type Me = lower!($Ty);
            fn layout() -> Layout {
                <lower!($Ty)>::lay()
            }
        }
    )*
};}
PrimitiveTyXMacro!{DeclarePrimitive{<Here>}}