//! Garbage collector object headers and typed references.

use core::{
    any::type_name,
    cell::Cell,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    panic::AssertUnwindSafe,
    ptr::NonNull,
};
use std::panic::catch_unwind;

use crate::Value;

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

/// Collection mode for a GC arena.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GcMode {
    /// Complete collection in one stop-the-world pass.
    #[default]
    StopTheWorld,
    /// Incremental collection mode with write barriers.
    Incremental,
}

/// High-level incremental collection phase.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GcPhase {
    /// No incremental cycle is active.
    #[default]
    Pause,
    /// Mark/propagation work is active.
    Propagate,
    /// Sweep work is active.
    Sweep,
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

    /// Applies a write barrier for a stored Lua value.
    pub fn write_barrier_value(&self, value: Value) {
        if let Some(reference) = value.as_short_string() {
            self.write_barrier_ref(reference);
        } else if let Some(reference) = value.as_long_string() {
            self.write_barrier_ref(reference);
        }
    }

    /// Applies a write barrier for a stored GC reference.
    pub fn write_barrier_ref<T>(&self, reference: GcRef<T>)
    where
        T: GcObject,
    {
        if self.color() != GcColor::Black {
            return;
        }
        // SAFETY: The caller is storing a GC reference into this object, so the
        // referenced object is expected to still be arena-owned and valid.
        let child_color = unsafe { reference.header() }.color();
        if child_color == GcColor::White {
            self.set_color(GcColor::Gray);
        }
    }
}

/// Trait implemented by GC-managed object payloads.
pub trait GcObject {
    /// Embedded GC header.
    fn header(&self) -> &GcHeader;

    /// Traces child GC references owned by this object.
    fn trace(&self, _tracer: &mut GcTracer<'_>) {}

    /// Removes weak references to objects that are about to be swept.
    fn remove_dead_weak_references(&mut self, _sweeper: &GcWeakSweeper<'_>) {}

    /// Returns true when this object should run a finalizer before sweep.
    fn needs_finalizer(&self) -> bool {
        false
    }

    /// Runs this object's finalizer.
    fn finalize(&mut self) -> Result<(), GcFinalizeError> {
        Ok(())
    }

    /// Runtime object kind from the embedded header.
    fn kind(&self) -> GcKind {
        self.header().kind()
    }
}

/// Error returned by a GC finalizer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GcFinalizeError {
    message: Box<str>,
}

impl GcFinalizeError {
    /// Creates a finalizer error with a stable message.
    #[must_use]
    pub fn new(message: impl Into<Box<str>>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GcFinalizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GcFinalizeError {}

/// Tracing context used during a stop-the-world mark phase.
pub struct GcTracer<'arena> {
    arena: &'arena GcArena,
    marked: usize,
    ephemerons: Vec<(Value, Value)>,
}

impl GcTracer<'_> {
    /// Marks a typed GC reference and recursively traces its children.
    pub fn mark_ref<T>(&mut self, reference: GcRef<T>) -> bool
    where
        T: GcObject,
    {
        self.mark_ptr(reference.ptr.cast())
    }

    /// Marks any GC-managed reference contained in a Lua value.
    pub fn mark_value(&mut self, value: Value) -> bool {
        if let Some(reference) = value.as_short_string() {
            self.mark_ref(reference)
        } else if let Some(reference) = value.as_long_string() {
            self.mark_ref(reference)
        } else {
            false
        }
    }

    /// Defers ephemeron value tracing until all roots and strong edges are marked.
    pub fn mark_ephemeron(&mut self, key: Value, value: Value) {
        self.ephemerons.push((key, value));
    }

    /// Returns true when the GC-managed reference in `value` is already marked.
    #[must_use]
    pub fn is_value_marked(&self, value: Value) -> bool {
        if let Some(reference) = value.as_short_string() {
            self.is_ptr_marked(reference.ptr.cast())
        } else if let Some(reference) = value.as_long_string() {
            self.is_ptr_marked(reference.ptr.cast())
        } else {
            true
        }
    }

    fn mark_ptr(&mut self, ptr: NonNull<()>) -> bool {
        let Some(allocation) = self
            .arena
            .objects
            .iter()
            .find(|allocation| allocation.ptr == ptr)
        else {
            return false;
        };

        let header = allocation.header() as *const GcHeader;
        if unsafe { (*header).color() } != GcColor::White {
            return false;
        }

        let object = allocation.object.as_ref() as *const dyn GcObject;
        // SAFETY: The tracer only reads arena allocations during the mark phase.
        // No sweep or mutation of `arena.objects` can run until tracing returns,
        // so these raw pointers remain valid for the recursive trace.
        unsafe {
            (*header).set_color(GcColor::Gray);
            self.marked += 1;
            (*object).trace(self);
            (*header).set_color(GcColor::Black);
        }
        true
    }

    fn is_ptr_marked(&self, ptr: NonNull<()>) -> bool {
        self.arena
            .objects
            .iter()
            .find(|allocation| allocation.ptr == ptr)
            .is_some_and(|allocation| allocation.header().color() != GcColor::White)
    }

    fn mark_ephemerons_to_fixpoint(&mut self) {
        loop {
            let mut changed = false;
            for (key, value) in self.ephemerons.clone() {
                if self.is_value_marked(key) {
                    changed |= self.mark_value(value);
                }
            }
            if !changed {
                break;
            }
        }
    }
}

/// Weak-reference sweep context used just before unreachable objects are removed.
pub struct GcWeakSweeper<'live> {
    live_ptrs: &'live [NonNull<()>],
}

impl GcWeakSweeper<'_> {
    /// Returns true when a typed GC reference points at a marked live object.
    #[must_use]
    pub fn is_ref_live<T>(&self, reference: GcRef<T>) -> bool {
        self.live_ptrs.contains(&reference.ptr.cast())
    }

    /// Returns true when `value` is not collectable or points at a live object.
    #[must_use]
    pub fn is_value_live(&self, value: Value) -> bool {
        if let Some(reference) = value.as_short_string() {
            self.is_ref_live(reference)
        } else if let Some(reference) = value.as_long_string() {
            self.is_ref_live(reference)
        } else {
            true
        }
    }
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

/// Opaque root handle returned by the GC arena.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GcRoot {
    id: u64,
}

impl GcRoot {
    /// Stable root identifier.
    #[must_use]
    pub const fn id(self) -> u64 {
        self.id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RootEntry {
    root: GcRoot,
    ptr: NonNull<()>,
}

struct GcAllocation {
    ptr: NonNull<()>,
    object: Box<dyn GcObject>,
}

impl GcAllocation {
    fn header(&self) -> &GcHeader {
        self.object.header()
    }
}

/// Allocation statistics for a GC arena.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GcStats {
    /// Number of currently allocated objects.
    pub live_objects: usize,
    /// Number of allocations made by this arena since creation.
    pub total_allocations: usize,
    /// Number of placeholder roots currently registered.
    pub roots: usize,
}

/// Result of one stop-the-world collection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GcCollectionStats {
    /// Objects reached from roots.
    pub marked: usize,
    /// Unreachable finalizers run before sweep.
    pub finalized: usize,
    /// Finalizers that returned an error or panicked.
    pub finalizer_errors: usize,
    /// Objects reclaimed by sweep.
    pub swept: usize,
    /// Objects remaining after collection.
    pub live_objects: usize,
}

/// Runtime-owned list of GC allocations.
#[derive(Default)]
pub struct GcArena {
    objects: Vec<GcAllocation>,
    roots: Vec<RootEntry>,
    total_allocations: usize,
    next_root_id: u64,
    mode: GcMode,
    phase: GcPhase,
}

impl GcArena {
    /// Creates an empty GC arena.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            objects: Vec::new(),
            roots: Vec::new(),
            total_allocations: 0,
            next_root_id: 0,
            mode: GcMode::StopTheWorld,
            phase: GcPhase::Pause,
        }
    }

    /// Current collection mode.
    #[must_use]
    pub const fn mode(&self) -> GcMode {
        self.mode
    }

    /// Updates the collection mode.
    pub fn set_mode(&mut self, mode: GcMode) {
        self.mode = mode;
    }

    /// Current incremental collection phase.
    #[must_use]
    pub const fn phase(&self) -> GcPhase {
        self.phase
    }

    /// Allocates a GC object and returns an unrooted typed reference to it.
    pub fn allocate<T>(&mut self, object: T) -> GcRef<T>
    where
        T: GcObject + 'static,
    {
        let boxed = Box::new(object);
        let ptr = NonNull::from(boxed.as_ref());

        self.objects.push(GcAllocation {
            ptr: ptr.cast(),
            object: boxed,
        });
        self.total_allocations += 1;

        // SAFETY: `ptr` points into a Box now owned by `self.objects`. The Box
        // allocation is stable until this arena drops or later sweep logic
        // removes it. The returned reference is intentionally unrooted.
        unsafe { GcRef::from_non_null(ptr) }
    }

    /// Adds a placeholder root for a GC reference.
    pub fn add_root<T>(&mut self, reference: GcRef<T>) -> GcRoot {
        let root = GcRoot {
            id: self.next_root_id,
        };
        self.next_root_id += 1;
        self.roots.push(RootEntry {
            root,
            ptr: reference.ptr.cast(),
        });
        root
    }

    /// Removes a placeholder root.
    pub fn remove_root(&mut self, root: GcRoot) -> bool {
        if let Some(index) = self.roots.iter().position(|entry| entry.root == root) {
            self.roots.swap_remove(index);
            true
        } else {
            false
        }
    }

    /// Returns true if the root handle is currently registered.
    #[must_use]
    pub fn contains_root(&self, root: GcRoot) -> bool {
        self.roots.iter().any(|entry| entry.root == root)
    }

    /// Returns true if the root handle currently points to `reference`.
    #[must_use]
    pub fn contains_root_for<T>(&self, root: GcRoot, reference: GcRef<T>) -> bool {
        self.roots
            .iter()
            .any(|entry| entry.root == root && entry.ptr == reference.ptr.cast())
    }

    /// Number of currently allocated objects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Returns true if no objects are allocated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Number of placeholder roots.
    #[must_use]
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    /// Current allocation statistics.
    #[must_use]
    pub fn stats(&self) -> GcStats {
        GcStats {
            live_objects: self.objects.len(),
            total_allocations: self.total_allocations,
            roots: self.roots.len(),
        }
    }

    /// Runs one stop-the-world mark-sweep collection.
    pub fn collect_garbage(&mut self) -> GcCollectionStats {
        self.reset_marks();
        let marked = self.mark_roots();
        self.remove_dead_weak_references();
        let finalizer_report = self.run_finalizers();
        let swept = self.sweep_unmarked();
        self.prune_dead_roots();
        self.reset_marks();

        GcCollectionStats {
            marked,
            finalized: finalizer_report.finalized,
            finalizer_errors: finalizer_report.errors,
            swept,
            live_objects: self.objects.len(),
        }
    }

    /// Runs one incremental collection step.
    pub fn incremental_step(&mut self) -> GcCollectionStats {
        if self.mode != GcMode::Incremental {
            self.mode = GcMode::Incremental;
        }
        self.phase = GcPhase::Propagate;
        let stats = self.collect_garbage();
        self.phase = GcPhase::Pause;
        stats
    }

    fn reset_marks(&self) {
        for allocation in &self.objects {
            allocation.header().set_color(GcColor::White);
        }
    }

    fn mark_roots(&self) -> usize {
        let mut tracer = GcTracer {
            arena: self,
            marked: 0,
            ephemerons: Vec::new(),
        };
        for root in &self.roots {
            tracer.mark_ptr(root.ptr);
        }
        tracer.mark_ephemerons_to_fixpoint();
        tracer.marked
    }

    fn remove_dead_weak_references(&mut self) {
        let live_ptrs = self
            .objects
            .iter()
            .filter(|allocation| allocation.header().color() == GcColor::Black)
            .map(|allocation| allocation.ptr)
            .collect::<Vec<_>>();
        let sweeper = GcWeakSweeper {
            live_ptrs: &live_ptrs,
        };

        for allocation in &mut self.objects {
            allocation.object.remove_dead_weak_references(&sweeper);
        }
    }

    fn run_finalizers(&mut self) -> FinalizerReport {
        let mut report = FinalizerReport::default();
        let finalizer_queue = self
            .objects
            .iter()
            .filter(|allocation| {
                allocation.header().color() == GcColor::White && allocation.object.needs_finalizer()
            })
            .map(|allocation| allocation.ptr)
            .collect::<Vec<_>>();

        for ptr in finalizer_queue {
            let Some(allocation) = self
                .objects
                .iter_mut()
                .find(|allocation| allocation.ptr == ptr)
            else {
                continue;
            };
            report.finalized += 1;
            let result = catch_unwind(AssertUnwindSafe(|| allocation.object.finalize()));
            if !matches!(result, Ok(Ok(()))) {
                report.errors += 1;
            }
        }
        report
    }

    fn sweep_unmarked(&mut self) -> usize {
        let before = self.objects.len();
        self.objects
            .retain(|allocation| allocation.header().color() == GcColor::Black);
        before - self.objects.len()
    }

    fn prune_dead_roots(&mut self) {
        let live_ptrs = self
            .objects
            .iter()
            .map(|allocation| allocation.ptr)
            .collect::<Vec<_>>();
        self.roots.retain(|root| live_ptrs.contains(&root.ptr));
    }
}

#[derive(Default)]
struct FinalizerReport {
    finalized: usize,
    errors: usize,
}

#[cfg(test)]
mod tests;
