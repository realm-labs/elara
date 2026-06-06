//! Garbage collector object headers and typed references.

use core::{
    any::type_name,
    cell::Cell,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ptr::NonNull,
};

/// Mark color used by the garbage collector.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GcColor {
    /// Candidate for collection or not yet visited.
    White,
    /// Reached but children still need tracing.
    Gray,
    /// Reached and fully traced.
    Black,
}

/// Runtime object kind stored in every GC header.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GcKind {
    /// Interned short string.
    ShortString,
    /// Long string object.
    LongString,
    /// Table object.
    Table,
    /// Closure object.
    Closure,
    /// Thread or coroutine object.
    Thread,
    /// Full userdata object.
    UserData,
    /// Compiled function prototype.
    Proto,
    /// Closed upvalue object.
    Upvalue,
}

/// Header embedded in every GC-managed object.
#[derive(Debug)]
pub struct GcHeader {
    kind: GcKind,
    color: Cell<GcColor>,
}

impl GcHeader {
    /// Creates a GC header with initial white mark color.
    #[must_use]
    pub const fn new(kind: GcKind) -> Self {
        Self {
            kind,
            color: Cell::new(GcColor::White),
        }
    }

    /// Object kind.
    #[must_use]
    pub const fn kind(&self) -> GcKind {
        self.kind
    }

    /// Current mark color.
    #[must_use]
    pub fn color(&self) -> GcColor {
        self.color.get()
    }

    /// Updates the current mark color.
    pub fn set_color(&self, color: GcColor) {
        self.color.set(color);
    }
}

/// Trait implemented by GC-managed object payloads.
pub trait GcObject {
    /// Static object kind for this payload type.
    const KIND: GcKind;

    /// Embedded GC header.
    fn header(&self) -> &GcHeader;
}

/// Typed reference to a GC-managed object.
///
/// `GcRef` is copyable, but it does not root the referenced object. Safe public
/// APIs must wrap runtime objects in handles that preserve the required root
/// lifetime.
pub struct GcRef<T> {
    ptr: NonNull<T>,
    marker: PhantomData<T>,
}

impl<T> GcRef<T> {
    /// Creates a GC reference from a non-null object pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a valid GC-managed object of type `T`. The object
    /// must remain allocated and correctly typed for every use of the returned
    /// reference. The returned reference does not root the object.
    #[allow(dead_code)]
    pub(crate) const unsafe fn from_non_null(ptr: NonNull<T>) -> Self {
        Self {
            ptr,
            marker: PhantomData,
        }
    }

    /// Returns true when both references point at the same object.
    #[must_use]
    pub fn ptr_eq(self, other: Self) -> bool {
        core::ptr::addr_eq(self.ptr.as_ptr(), other.ptr.as_ptr())
    }

    /// Borrows the referenced object.
    ///
    /// # Safety
    ///
    /// The caller must ensure the referenced object is still allocated, has not
    /// been moved, and is valid for the returned borrow lifetime.
    #[must_use]
    pub unsafe fn as_ref<'a>(self) -> &'a T {
        // SAFETY: The caller upholds allocation, type, and lifetime validity for
        // the wrapped non-null pointer.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: GcObject> GcRef<T> {
    /// Borrows the referenced object's GC header.
    ///
    /// # Safety
    ///
    /// The caller must ensure the referenced object is still allocated, has not
    /// been moved, and is valid for the returned borrow lifetime.
    #[must_use]
    pub unsafe fn header<'a>(self) -> &'a GcHeader
    where
        T: 'a,
    {
        // SAFETY: This forwards the same validity requirements to `as_ref`.
        unsafe { self.as_ref() }.header()
    }
}

impl<T> Clone for GcRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for GcRef<T> {}

impl<T> fmt::Debug for GcRef<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GcRef")
            .field("type", &type_name::<T>())
            .finish_non_exhaustive()
    }
}

impl<T> PartialEq for GcRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(*other)
    }
}

impl<T> Eq for GcRef<T> {}

impl<T> Hash for GcRef<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use core::ptr::NonNull;

    use super::{GcColor, GcHeader, GcKind, GcObject, GcRef};

    #[derive(Debug)]
    struct TestObject {
        header: GcHeader,
    }

    impl TestObject {
        fn new() -> Self {
            Self {
                header: GcHeader::new(Self::KIND),
            }
        }
    }

    impl GcObject for TestObject {
        const KIND: GcKind = GcKind::Table;

        fn header(&self) -> &GcHeader {
            &self.header
        }
    }

    #[test]
    fn gc_header_tracks_kind_and_color() {
        let header = GcHeader::new(GcKind::Closure);

        assert_eq!(header.kind(), GcKind::Closure);
        assert_eq!(header.color(), GcColor::White);

        header.set_color(GcColor::Gray);
        assert_eq!(header.color(), GcColor::Gray);

        header.set_color(GcColor::Black);
        assert_eq!(header.color(), GcColor::Black);
    }

    #[test]
    fn gc_ref_compares_object_identity() {
        let object = TestObject::new();
        let other = TestObject::new();

        // SAFETY: The test objects are stack allocated and live for the entire
        // duration of the references used in this test.
        let reference = unsafe { GcRef::from_non_null(NonNull::from(&object)) };
        // SAFETY: Same object and lifetime as `reference`.
        let same = unsafe { GcRef::from_non_null(NonNull::from(&object)) };
        // SAFETY: `other` is stack allocated and lives for this test.
        let different = unsafe { GcRef::from_non_null(NonNull::from(&other)) };

        assert_eq!(reference, same);
        assert!(reference.ptr_eq(same));
        assert_ne!(reference, different);
    }

    #[test]
    fn gc_ref_borrows_object_and_header_through_unsafe_helpers() {
        let object = TestObject::new();
        // SAFETY: The test object is stack allocated and lives for the entire
        // duration of the reference used in this test.
        let reference = unsafe { GcRef::from_non_null(NonNull::from(&object)) };

        // SAFETY: `object` is still alive and has not moved.
        let borrowed = unsafe { reference.as_ref() };
        assert_eq!(borrowed.header().kind(), GcKind::Table);

        // SAFETY: `object` is still alive and has not moved.
        let header = unsafe { reference.header() };
        assert_eq!(header.kind(), GcKind::Table);
    }
}
