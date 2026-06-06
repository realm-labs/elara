use core::ptr::NonNull;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::{GcArena, GcCollectionStats, GcColor, GcHeader, GcKind, GcObject, GcRef, GcStats};

#[derive(Debug)]
struct TestObject {
    header: GcHeader,
}

impl TestObject {
    fn new() -> Self {
        Self {
            header: GcHeader::new(GcKind::Table),
        }
    }
}

impl GcObject for TestObject {
    fn header(&self) -> &GcHeader {
        &self.header
    }
}

struct DropObject {
    header: GcHeader,
    drops: Arc<AtomicUsize>,
}

impl DropObject {
    fn new(drops: Arc<AtomicUsize>) -> Self {
        Self {
            header: GcHeader::new(GcKind::UserData),
            drops,
        }
    }
}

impl Drop for DropObject {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

impl GcObject for DropObject {
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

#[test]
fn gc_alloc_arena_allocates_objects_and_tracks_stats() {
    let mut arena = GcArena::new();

    assert!(arena.is_empty());
    assert_eq!(arena.stats(), GcStats::default());

    let reference = arena.allocate(TestObject::new());

    assert_eq!(arena.len(), 1);
    assert_eq!(
        arena.stats(),
        GcStats {
            live_objects: 1,
            total_allocations: 1,
            roots: 0,
        }
    );

    // SAFETY: The arena owns the allocated object and is still alive.
    let object = unsafe { reference.as_ref() };
    assert_eq!(object.kind(), GcKind::Table);
}

#[test]
fn gc_alloc_root_placeholder_tracks_registered_roots() {
    let mut arena = GcArena::new();
    let reference = arena.allocate(TestObject::new());

    let root = arena.add_root(reference);

    assert_eq!(root.id(), 0);
    assert_eq!(arena.root_count(), 1);
    assert!(arena.contains_root(root));
    assert!(arena.contains_root_for(root, reference));
    assert_eq!(arena.stats().roots, 1);

    assert!(arena.remove_root(root));
    assert!(!arena.contains_root(root));
    assert_eq!(arena.root_count(), 0);
    assert!(!arena.remove_root(root));
}

#[test]
fn gc_alloc_arena_drops_owned_objects() {
    let drops = Arc::new(AtomicUsize::new(0));

    {
        let mut arena = GcArena::new();
        arena.allocate(DropObject::new(Arc::clone(&drops)));
        arena.allocate(DropObject::new(Arc::clone(&drops)));

        assert_eq!(drops.load(Ordering::SeqCst), 0);
        assert_eq!(arena.stats().live_objects, 2);
    }

    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn gc_mark_sweep_collects_unrooted_objects() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut arena = GcArena::new();

    arena.allocate(DropObject::new(Arc::clone(&drops)));
    arena.allocate(DropObject::new(Arc::clone(&drops)));

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 0,
            swept: 2,
            live_objects: 0,
        }
    );
    assert_eq!(arena.len(), 0);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn gc_mark_sweep_keeps_rooted_objects() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut arena = GcArena::new();

    let rooted = arena.allocate(DropObject::new(Arc::clone(&drops)));
    arena.allocate(DropObject::new(Arc::clone(&drops)));
    let root = arena.add_root(rooted);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 1,
            swept: 1,
            live_objects: 1,
        }
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(arena.contains_root_for(root, rooted));
    assert_eq!(arena.stats().live_objects, 1);
    assert_eq!(arena.stats().roots, 1);

    assert!(arena.remove_root(root));
    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 0,
            swept: 1,
            live_objects: 0,
        }
    );
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}
