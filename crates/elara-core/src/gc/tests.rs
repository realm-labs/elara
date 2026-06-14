use core::ptr::NonNull;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use super::{
    GcArena, GcCollectionStats, GcColor, GcFinalizeError, GcHeader, GcKind, GcMode, GcObject,
    GcPhase, GcRef, GcStats, GcTracer,
};
use crate::{LongString, LuaThread, ShortString, Table, Value, WeakMode};

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

struct CapturedValueObject {
    header: GcHeader,
    captured: Value,
}

impl CapturedValueObject {
    fn new(captured: Value) -> Self {
        Self {
            header: GcHeader::new(GcKind::Upvalue),
            captured,
        }
    }
}

impl GcObject for CapturedValueObject {
    fn header(&self) -> &GcHeader {
        &self.header
    }

    fn trace(&self, tracer: &mut GcTracer<'_>) {
        tracer.mark_value(self.captured);
    }
}

#[derive(Clone, Copy)]
enum FinalizerBehavior {
    Ok,
    Error,
}

struct FinalizableUserData {
    header: GcHeader,
    events: Arc<Mutex<Vec<&'static str>>>,
    behavior: FinalizerBehavior,
}

impl FinalizableUserData {
    fn new(events: Arc<Mutex<Vec<&'static str>>>, behavior: FinalizerBehavior) -> Self {
        Self {
            header: GcHeader::new(GcKind::UserData),
            events,
            behavior,
        }
    }
}

impl Drop for FinalizableUserData {
    fn drop(&mut self) {
        self.events
            .lock()
            .expect("event log lock must not be poisoned")
            .push("drop");
    }
}

impl GcObject for FinalizableUserData {
    fn header(&self) -> &GcHeader {
        &self.header
    }

    fn needs_finalizer(&self) -> bool {
        true
    }

    fn finalize(&mut self) -> Result<(), GcFinalizeError> {
        self.events
            .lock()
            .expect("event log lock must not be poisoned")
            .push("finalize");
        match self.behavior {
            FinalizerBehavior::Ok => Ok(()),
            FinalizerBehavior::Error => Err(GcFinalizeError::new("finalizer failed")),
        }
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
fn finalizer_queue_runs_userdata_finalizer_before_drop() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut arena = GcArena::new();
    arena.allocate(FinalizableUserData::new(
        Arc::clone(&events),
        FinalizerBehavior::Ok,
    ));

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 0,
            finalized: 1,
            finalizer_errors: 0,
            swept: 1,
            live_objects: 0,
        }
    );
    assert_eq!(
        events
            .lock()
            .expect("event log lock must not be poisoned")
            .as_slice(),
        ["finalize", "drop"]
    );
}

#[test]
fn finalizer_errors_are_recorded_and_do_not_block_sweep() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut arena = GcArena::new();
    arena.allocate(FinalizableUserData::new(
        Arc::clone(&events),
        FinalizerBehavior::Error,
    ));

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 0,
            finalized: 1,
            finalizer_errors: 1,
            swept: 1,
            live_objects: 0,
        }
    );
    assert_eq!(
        events
            .lock()
            .expect("event log lock must not be poisoned")
            .as_slice(),
        ["finalize", "drop"]
    );
}

#[test]
fn finalizer_queue_skips_reachable_userdata_until_unrooted() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut arena = GcArena::new();
    let userdata = arena.allocate(FinalizableUserData::new(
        Arc::clone(&events),
        FinalizerBehavior::Ok,
    ));
    let root = arena.add_root(userdata);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 1,
            finalized: 0,
            finalizer_errors: 0,
            swept: 0,
            live_objects: 1,
        }
    );
    assert!(
        events
            .lock()
            .expect("event log lock must not be poisoned")
            .is_empty()
    );

    assert!(arena.remove_root(root));
    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 0,
            finalized: 1,
            finalizer_errors: 0,
            swept: 1,
            live_objects: 0,
        }
    );
    assert_eq!(
        events
            .lock()
            .expect("event log lock must not be poisoned")
            .as_slice(),
        ["finalize", "drop"]
    );
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
            finalized: 0,
            finalizer_errors: 0,
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
            finalized: 0,
            finalizer_errors: 0,
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
            finalized: 0,
            finalizer_errors: 0,
            swept: 1,
            live_objects: 0,
        }
    );
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn gc_trace_table_marks_values_keys_and_metatable() {
    let mut arena = GcArena::new();

    let metatable = arena.allocate(Table::new());
    let key = arena.allocate(ShortString::new("field").expect("short string fits"));
    let value = arena.allocate(LongString::new("value"));
    let mut table = Table::new();
    table.set_metatable(Some(metatable));
    assert!(table.raw_set_value(Value::short_string(key), Value::integer(1)));
    assert!(table.raw_set_integer(1, Value::long_string(value)));
    let table = arena.allocate(table);
    arena.add_root(table);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 4,
            finalized: 0,
            finalizer_errors: 0,
            swept: 0,
            live_objects: 4,
        }
    );
    assert_eq!(arena.len(), 4);
}

#[test]
fn gc_trace_thread_stack_marks_value_references() {
    let mut arena = GcArena::new();

    let value = arena.allocate(LongString::new("stack"));
    let mut thread = LuaThread::new();
    thread.push_value(Value::long_string(value));
    let thread = arena.allocate(thread);
    arena.add_root(thread);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 2,
            finalized: 0,
            finalizer_errors: 0,
            swept: 0,
            live_objects: 2,
        }
    );
}

#[test]
fn gc_trace_upvalue_marks_captured_value() {
    let mut arena = GcArena::new();

    let value = arena.allocate(LongString::new("captured"));
    let upvalue = arena.allocate(CapturedValueObject::new(Value::long_string(value)));
    arena.add_root(upvalue);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 2,
            finalized: 0,
            finalizer_errors: 0,
            swept: 0,
            live_objects: 2,
        }
    );
}

#[test]
fn gc_trace_registry_roots_mark_objects_once() {
    let mut arena = GcArena::new();

    let value = arena.allocate(LongString::new("registered"));
    arena.add_root(value);
    arena.add_root(value);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 1,
            finalized: 0,
            finalizer_errors: 0,
            swept: 0,
            live_objects: 1,
        }
    );
}

#[test]
fn weak_table_values_do_not_keep_entries_alive() {
    let mut arena = GcArena::new();

    let key = arena.allocate(ShortString::new("key").expect("short string fits"));
    let value = arena.allocate(LongString::new("weak value"));
    let mut table = Table::new();
    table.set_weak_mode(WeakMode::Values);
    assert!(table.raw_set_value(Value::short_string(key), Value::long_string(value)));
    let table = arena.allocate(table);
    arena.add_root(table);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 2,
            finalized: 0,
            finalizer_errors: 0,
            swept: 1,
            live_objects: 2,
        }
    );
    // SAFETY: The rooted table survived collection.
    let table = unsafe { table.as_ref() };
    assert_eq!(table.raw_get_value(Value::short_string(key)), Value::nil());
}

#[test]
fn weak_table_keys_do_not_keep_ephemeron_values_alive_when_key_is_dead() {
    let mut arena = GcArena::new();

    let key = arena.allocate(ShortString::new("key").expect("short string fits"));
    let value = arena.allocate(LongString::new("ephemeron value"));
    let mut table = Table::new();
    table.set_weak_mode(WeakMode::Keys);
    assert!(table.raw_set_value(Value::short_string(key), Value::long_string(value)));
    let table = arena.allocate(table);
    arena.add_root(table);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 1,
            finalized: 0,
            finalizer_errors: 0,
            swept: 2,
            live_objects: 1,
        }
    );
    // SAFETY: The rooted table survived collection.
    let table = unsafe { table.as_ref() };
    assert_eq!(table.raw_get_value(Value::short_string(key)), Value::nil());
}

#[test]
fn weak_table_keys_mark_ephemeron_values_when_key_is_live() {
    let mut arena = GcArena::new();

    let key = arena.allocate(ShortString::new("key").expect("short string fits"));
    let value = arena.allocate(LongString::new("ephemeron value"));
    let mut table = Table::new();
    table.set_weak_mode(WeakMode::Keys);
    assert!(table.raw_set_value(Value::short_string(key), Value::long_string(value)));
    let table = arena.allocate(table);
    arena.add_root(table);
    arena.add_root(key);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 3,
            finalized: 0,
            finalizer_errors: 0,
            swept: 0,
            live_objects: 3,
        }
    );
    // SAFETY: The rooted table survived collection.
    let table = unsafe { table.as_ref() };
    assert_eq!(
        table.raw_get_value(Value::short_string(key)),
        Value::long_string(value)
    );
}

#[test]
fn weak_table_keys_and_values_drop_collectable_entries() {
    let mut arena = GcArena::new();

    let key = arena.allocate(ShortString::new("key").expect("short string fits"));
    let value = arena.allocate(LongString::new("weak value"));
    let mut table = Table::new();
    table.set_weak_mode(WeakMode::KeysAndValues);
    assert!(table.raw_set_value(Value::short_string(key), Value::long_string(value)));
    let table = arena.allocate(table);
    arena.add_root(table);

    let collection = arena.collect_garbage();

    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 1,
            finalized: 0,
            finalizer_errors: 0,
            swept: 2,
            live_objects: 1,
        }
    );
    // SAFETY: The rooted table survived collection.
    let table = unsafe { table.as_ref() };
    assert_eq!(table.raw_get_value(Value::short_string(key)), Value::nil());
}

#[test]
fn incremental_gc_step_runs_in_incremental_mode_and_returns_to_pause() {
    let mut arena = GcArena::new();
    arena.set_mode(GcMode::Incremental);
    arena.allocate(DropObject::new(Arc::new(AtomicUsize::new(0))));

    let collection = arena.incremental_step();

    assert_eq!(arena.mode(), GcMode::Incremental);
    assert_eq!(arena.phase(), GcPhase::Pause);
    assert_eq!(
        collection,
        GcCollectionStats {
            marked: 0,
            finalized: 0,
            finalizer_errors: 0,
            swept: 1,
            live_objects: 0,
        }
    );
}

#[test]
fn incremental_gc_table_write_barrier_grays_black_table_for_value() {
    let mut arena = GcArena::new();
    let value = arena.allocate(LongString::new("child"));
    let mut table = Table::new();
    table.header().set_color(GcColor::Black);

    assert!(table.raw_set_integer(1, Value::long_string(value)));

    assert_eq!(table.header().color(), GcColor::Gray);
}

#[test]
fn incremental_gc_table_write_barrier_grays_black_table_for_key() {
    let mut arena = GcArena::new();
    let key = arena.allocate(ShortString::new("key").expect("short string fits"));
    let mut table = Table::new();
    table.header().set_color(GcColor::Black);

    assert!(table.raw_set_value(Value::short_string(key), Value::integer(1)));

    assert_eq!(table.header().color(), GcColor::Gray);
}

#[test]
fn incremental_gc_table_write_barrier_grays_black_table_for_metatable() {
    let mut arena = GcArena::new();
    let metatable = arena.allocate(Table::new());
    let mut table = Table::new();
    table.header().set_color(GcColor::Black);

    table.set_metatable(Some(metatable));

    assert_eq!(table.header().color(), GcColor::Gray);
}

#[test]
fn incremental_gc_thread_stack_write_barrier_grays_black_thread() {
    let mut arena = GcArena::new();
    let value = arena.allocate(LongString::new("stack"));
    let mut thread = LuaThread::new();
    thread.header().set_color(GcColor::Black);

    thread.push_value(Value::long_string(value));

    assert_eq!(thread.header().color(), GcColor::Gray);
}
