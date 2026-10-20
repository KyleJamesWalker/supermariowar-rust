//! `Ptr`, `Global`, `Owned` and `Aliased`, and the macros for embedded base structs and `u8` enums (see
//! docs/guides/CODE_CONVENTIONS.md).

mod pointers;
pub use pointers::{Aliased, Global, Owned, Ptr};

/// `impl_base!(Derived => field: Base)` makes `Derived` deref, shared and mutable, to its embedded `Base` field.
#[macro_export]
macro_rules! impl_base {
    ($derived:ty => $field:ident : $base:ty) => {
        impl ::std::ops::Deref for $derived {
            type Target = $base;
            #[inline(always)]
            fn deref(&self) -> &$base {
                &self.$field
            }
        }
        impl ::std::ops::DerefMut for $derived {
            #[inline(always)]
            fn deref_mut(&mut self) -> &mut $base {
                &mut self.$field
            }
        }
    };
}

/// `from_u8` for a `#[repr(u8)]` enum with `$count` variants numbered from 0 and a `Default`.
/// Out-of-range values (only from corrupt files) become the default variant.
#[macro_export]
macro_rules! enum_from_u8 {
    ($t:ty, $count:expr) => {
        impl $t {
            pub fn from_u8(v: u8) -> Self {
                if (v as usize) < $count {
                    // SAFETY: `v` is below `$count`, so it names a variant of the `#[repr(u8)]` enum.
                    unsafe { ::std::mem::transmute::<u8, $t>(v) }
                } else {
                    <$t>::default()
                }
            }
        }
    };
}
