//! `Aliased`, `Global<T>` and `Ptr<T>`, the global-state infrastructure (see ARCHITECTURE.md).

use std::cell::UnsafeCell;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

/// Zero-sized marker that makes the containing type `!Freeze + !Unpin`, so rustc emits no
/// `noalias` for `&T` / `&mut T` and C++-style mutation through other pointers stays sound in practice.
#[derive(Default)]
pub struct Aliased {
    _cell: UnsafeCell<()>,
    _pin: PhantomPinned,
}

impl Aliased {
    pub const fn new() -> Self {
        Aliased { _cell: UnsafeCell::new(()), _pin: PhantomPinned }
    }
}

impl Clone for Aliased {
    fn clone(&self) -> Self {
        Aliased::new()
    }
}

impl std::fmt::Debug for Aliased {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Aliased")
    }
}

impl PartialEq for Aliased {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Aliased {}

impl std::hash::Hash for Aliased {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// A C++ global object constructed at startup. Derefs to `T`.
pub struct Global<T> {
    value: MaybeUninit<T>,
    initialized: bool,
}

impl<T> Global<T> {
    pub const fn uninit() -> Self {
        Global { value: MaybeUninit::uninit(), initialized: false }
    }

    pub const fn new(v: T) -> Self {
        Global { value: MaybeUninit::new(v), initialized: true }
    }

    pub fn init(&mut self, v: T) {
        if self.initialized {
            unsafe { self.value.assume_init_drop() };
        }
        self.value.write(v);
        self.initialized = true;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn as_ptr(&mut self) -> Ptr<T> {
        Ptr::from_mut(&mut **self)
    }
}

impl<T> Deref for Global<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        debug_assert!(self.initialized, "global used before init");
        unsafe { self.value.assume_init_ref() }
    }
}

impl<T> DerefMut for Global<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        debug_assert!(self.initialized, "global used before init");
        unsafe { self.value.assume_init_mut() }
    }
}

/// A nullable, non-owning C++ pointer (`T*`). Copy, compares by address, derefs to `T`.
pub struct Ptr<T: ?Sized>(Option<NonNull<T>>);

impl<T: ?Sized> Ptr<T> {
    pub const fn null() -> Self {
        Ptr(None)
    }

    #[inline(always)]
    pub fn from_mut(r: &mut T) -> Self {
        Ptr(Some(NonNull::from(r)))
    }

    #[inline(always)]
    pub fn from_raw(p: *mut T) -> Self {
        Ptr(NonNull::new(p))
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.0.is_none()
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *mut T {
        match self.0 {
            Some(p) => p.as_ptr(),
            None => panic!("null Ptr::as_ptr"),
        }
    }

    /// The address only, for comparisons and hashing.
    #[inline(always)]
    pub fn addr(&self) -> usize {
        match self.0 {
            Some(p) => p.as_ptr() as *mut u8 as usize,
            None => 0,
        }
    }

    /// `&mut *p` with an unbounded lifetime, like dereferencing a C++ pointer.
    #[inline(always)]
    pub fn get<'a>(self) -> &'a mut T {
        match self.0 {
            Some(p) => unsafe { &mut *p.as_ptr() },
            None => panic!("null Ptr dereference"),
        }
    }

    /// C++ `delete p`.
    pub fn delete(self) {
        if let Some(p) = self.0 {
            drop(unsafe { Box::from_raw(p.as_ptr()) });
        }
    }

    pub fn from_box(b: Box<T>) -> Self {
        Ptr(Some(unsafe { NonNull::new_unchecked(Box::into_raw(b)) }))
    }
}

impl<T> Ptr<T> {
    /// C++ `new T(...)`.
    pub fn new_box(v: T) -> Self {
        Ptr::from_box(Box::new(v))
    }
}

impl<T: ?Sized> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ?Sized> Copy for Ptr<T> {}

impl<T: ?Sized> Default for Ptr<T> {
    fn default() -> Self {
        Ptr::null()
    }
}

impl<T: ?Sized, U: ?Sized> PartialEq<Ptr<U>> for Ptr<T> {
    fn eq(&self, other: &Ptr<U>) -> bool {
        self.addr() == other.addr()
    }
}
impl<T: ?Sized> Eq for Ptr<T> {}

impl<T: ?Sized> std::fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ptr({:#x})", self.addr())
    }
}

impl<T: ?Sized> Deref for Ptr<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        match self.0 {
            Some(p) => unsafe { &*p.as_ptr() },
            None => panic!("null Ptr dereference"),
        }
    }
}

impl<T: ?Sized> DerefMut for Ptr<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        match self.0 {
            Some(p) => unsafe { &mut *p.as_ptr() },
            None => panic!("null Ptr dereference"),
        }
    }
}
