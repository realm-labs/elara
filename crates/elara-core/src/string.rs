//! Immutable Lua strings and short-string interning.

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use crate::{GcArena, GcHeader, GcKind, GcObject, GcRef, GcRoot};

/// Maximum byte length for interned short strings.
pub const SHORT_STRING_MAX_BYTES: usize = 40;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Deterministic hash for Lua string bytes.
#[must_use]
pub fn hash_string_bytes(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Interned short Lua string.
#[derive(Debug)]
pub struct ShortString {
    header: GcHeader,
    bytes: Box<[u8]>,
    hash: u64,
}

impl ShortString {
    /// Creates a short string when `bytes` fits the short-string limit.
    #[must_use]
    pub fn new(bytes: impl AsRef<[u8]>) -> Option<Self> {
        let bytes = bytes.as_ref();
        if bytes.len() > SHORT_STRING_MAX_BYTES {
            return None;
        }

        Some(Self {
            header: GcHeader::new(GcKind::ShortString),
            bytes: bytes.into(),
            hash: hash_string_bytes(bytes),
        })
    }

    /// String bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// String length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns true for an empty string.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Cached string hash.
    #[must_use]
    pub const fn hash(&self) -> u64 {
        self.hash
    }
}

impl GcObject for ShortString {
    fn header(&self) -> &GcHeader {
        &self.header
    }
}

impl PartialEq for ShortString {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.bytes == other.bytes
    }
}

impl Eq for ShortString {}

impl Hash for ShortString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

/// Non-interned long Lua string.
#[derive(Debug)]
pub struct LongString {
    header: GcHeader,
    bytes: Box<[u8]>,
    hash: u64,
}

impl LongString {
    /// Creates a long string.
    #[must_use]
    pub fn new(bytes: impl AsRef<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        Self {
            header: GcHeader::new(GcKind::LongString),
            bytes: bytes.into(),
            hash: hash_string_bytes(bytes),
        }
    }

    /// String bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// String length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns true for an empty string.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Cached string hash.
    #[must_use]
    pub const fn hash(&self) -> u64 {
        self.hash
    }
}

impl GcObject for LongString {
    fn header(&self) -> &GcHeader {
        &self.header
    }
}

impl PartialEq for LongString {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.bytes == other.bytes
    }
}

impl Eq for LongString {}

impl Hash for LongString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

#[derive(Clone, Copy)]
struct InternedShortString {
    reference: GcRef<ShortString>,
    _root: GcRoot,
}

/// Intern table for short strings.
#[derive(Default)]
pub struct StringInterner {
    short_strings: HashMap<Box<[u8]>, InternedShortString>,
}

impl StringInterner {
    /// Creates an empty string interner.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns a short string, allocating it in `arena` when missing.
    ///
    /// # Panics
    ///
    /// Panics when `bytes` exceeds `SHORT_STRING_MAX_BYTES`.
    pub fn intern_short(
        &mut self,
        arena: &mut GcArena,
        bytes: impl AsRef<[u8]>,
    ) -> GcRef<ShortString> {
        let bytes = bytes.as_ref();
        assert!(
            bytes.len() <= SHORT_STRING_MAX_BYTES,
            "short string length exceeds limit"
        );

        if let Some(interned) = self.short_strings.get(bytes) {
            return interned.reference;
        }

        let string = ShortString::new(bytes).expect("short string length was checked");
        let reference = arena.allocate(string);
        let root = arena.add_root(reference);

        self.short_strings.insert(
            bytes.into(),
            InternedShortString {
                reference,
                _root: root,
            },
        );

        reference
    }

    /// Returns an interned short string without allocating.
    #[must_use]
    pub fn get_short(&self, bytes: impl AsRef<[u8]>) -> Option<GcRef<ShortString>> {
        self.short_strings
            .get(bytes.as_ref())
            .map(|interned| interned.reference)
    }

    /// Number of interned short strings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.short_strings.len()
    }

    /// Returns true if no strings are interned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.short_strings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use super::{
        LongString, SHORT_STRING_MAX_BYTES, ShortString, StringInterner, hash_string_bytes,
    };
    use crate::{GcArena, GcKind, GcObject};

    #[test]
    fn string_short_string_accepts_limited_byte_strings() {
        let string = ShortString::new(b"hello\0lua").expect("short string should fit");

        assert_eq!(string.header().kind(), GcKind::ShortString);
        assert_eq!(string.as_bytes(), b"hello\0lua");
        assert_eq!(string.len(), 9);
        assert!(!string.is_empty());
        assert_eq!(string.hash(), hash_string_bytes(b"hello\0lua"));
    }

    #[test]
    fn string_short_string_rejects_long_inputs() {
        let bytes = vec![b'x'; SHORT_STRING_MAX_BYTES + 1];

        assert!(ShortString::new(bytes).is_none());
    }

    #[test]
    fn string_long_string_stores_arbitrary_bytes() {
        let bytes = vec![b'a'; SHORT_STRING_MAX_BYTES + 8];
        let string = LongString::new(&bytes);

        assert_eq!(string.header().kind(), GcKind::LongString);
        assert_eq!(string.as_bytes(), bytes.as_slice());
        assert_eq!(string.len(), SHORT_STRING_MAX_BYTES + 8);
        assert_eq!(string.hash(), hash_string_bytes(&bytes));
    }

    #[test]
    fn string_equality_and_hashing_use_bytes() {
        let left = ShortString::new(b"same").expect("short string should fit");
        let right = ShortString::new(b"same").expect("short string should fit");
        let other = ShortString::new(b"other").expect("short string should fit");

        assert_eq!(left, right);
        assert_ne!(left, other);

        let mut left_hasher = DefaultHasher::new();
        Hash::hash(&left, &mut left_hasher);
        let mut right_hasher = DefaultHasher::new();
        Hash::hash(&right, &mut right_hasher);

        assert_eq!(left_hasher.finish(), right_hasher.finish());
    }

    #[test]
    fn string_interner_reuses_short_string_references() {
        let mut arena = GcArena::new();
        let mut interner = StringInterner::new();

        let first = interner.intern_short(&mut arena, "answer");
        let second = interner.intern_short(&mut arena, b"answer");

        assert_eq!(first, second);
        assert!(first.ptr_eq(second));
        assert_eq!(interner.len(), 1);
        assert_eq!(arena.len(), 1);
        assert_eq!(arena.root_count(), 1);
        assert_eq!(interner.get_short("answer"), Some(first));
    }

    #[test]
    fn string_interner_roots_short_strings_for_collection() {
        let mut arena = GcArena::new();
        let mut interner = StringInterner::new();

        let interned = interner.intern_short(&mut arena, "rooted");
        let collection = arena.collect_garbage();

        assert_eq!(collection.marked, 1);
        assert_eq!(collection.swept, 0);
        assert_eq!(arena.len(), 1);
        assert_eq!(arena.root_count(), 1);
        assert_eq!(interner.get_short("rooted"), Some(interned));
    }

    #[test]
    fn string_long_strings_are_not_interned_by_identity() {
        let mut arena = GcArena::new();
        let bytes = vec![b'z'; SHORT_STRING_MAX_BYTES + 1];

        let left = arena.allocate(LongString::new(&bytes));
        let right = arena.allocate(LongString::new(&bytes));

        assert_ne!(left, right);
        assert!(!left.ptr_eq(right));

        // SAFETY: The arena owns both allocated strings and is still alive.
        let left_string = unsafe { left.as_ref() };
        // SAFETY: The arena owns both allocated strings and is still alive.
        let right_string = unsafe { right.as_ref() };
        assert_eq!(left_string, right_string);
    }
}
