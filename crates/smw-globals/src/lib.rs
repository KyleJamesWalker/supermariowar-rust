//! `Ptr`, `Global` and `Aliased`, and the macros that model C++ inheritance (see docs/ARCHITECTURE.md).

mod pointers;
pub use pointers::{Aliased, Global, Ptr};

/// `impl_base!(Derived => field: Base)` makes `Derived` deref to its embedded C++ base class.
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

/// `static_cast<Enum>(uint8_t)` for a `#[repr(u8)]` enum with `$count` variants numbered from 0.
/// Out-of-range values (only from corrupt files) become the default variant.
#[macro_export]
macro_rules! enum_from_u8 {
    ($t:ty, $count:expr) => {
        impl $t {
            pub fn from_u8(v: u8) -> Self {
                if (v as usize) < $count {
                    unsafe { ::std::mem::transmute::<u8, $t>(v) }
                } else {
                    <$t>::default()
                }
            }
        }
    };
}
