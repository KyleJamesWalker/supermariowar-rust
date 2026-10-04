//! Global-state infrastructure (see ARCHITECTURE.md) and re-exports of every C++ global.

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

pub use crate::common::global::*;
pub use crate::smw::main::{blitdest, screen, x_shake, y_shake};

/// Runs the constructors of C++ globals that have static storage, in C++ definition order.
pub fn init_globals() {
    crate::common::global::init_globals();
    crate::smw::world::init_globals();
    crate::smw::net::init_globals();
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Base {
        x: i32,
        _alias: Aliased,
    }
    struct Derived {
        base: Base,
        y: i32,
    }
    crate::impl_base!(Derived => base: Base);

    #[test]
    fn ptr_and_base() {
        let mut d = Box::new(Derived { base: Base { x: 1, _alias: Aliased::new() }, y: 2 });
        let mut p = Ptr::from_mut(&mut *d);
        p.x = 5;
        p.y += 1;
        assert_eq!(d.x, 5);
        assert_eq!(d.y, 3);
        assert!(Ptr::<Derived>::null().is_null());
        assert_eq!(p, Ptr::from_mut(&mut *d));
        assert_eq!(std::mem::size_of::<Ptr<Derived>>(), std::mem::size_of::<usize>());
        assert_eq!(std::mem::size_of::<Aliased>(), 0);
    }
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
