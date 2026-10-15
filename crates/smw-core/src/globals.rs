//! Global-state infrastructure (see docs/ARCHITECTURE.md) and re-exports of every C++ global.
use crate::game::Game;

pub use smw_globals::{Aliased, Global, Ptr};

pub use crate::common::global::*;
pub use crate::game::game;

/// Runs the constructors of C++ globals that have static storage, in C++ definition order.
pub fn init_globals(g: &mut Game) {
    crate::common::global::init_globals(g);
    crate::smw::world::init_globals(g);
    crate::smw::net::init_globals(g);
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
