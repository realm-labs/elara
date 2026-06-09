//! VM state, Lua thread stack, and call frames.

use crate::{GcArena, GcHeader, GcKind, GcObject, Value};

/// Stack index within one Lua thread.
pub type StackIndex = usize;

/// Runtime VM state.
#[derive(Default)]
pub struct Vm {
    gc: GcArena,
}

impl Vm {
    /// Creates an empty VM state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Immutable access to the GC arena.
    #[must_use]
    pub const fn gc(&self) -> &GcArena {
        &self.gc
    }

    /// Mutable access to the GC arena.
    pub fn gc_mut(&mut self) -> &mut GcArena {
        &mut self.gc
    }
}

/// Lua coroutine/thread state.
#[derive(Debug)]
pub struct LuaThread {
    header: GcHeader,
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    status: ThreadStatus,
}

impl LuaThread {
    /// Creates an empty runnable Lua thread.
    #[must_use]
    pub fn new() -> Self {
        Self {
            header: GcHeader::new(GcKind::Thread),
            stack: Vec::new(),
            frames: Vec::new(),
            status: ThreadStatus::Runnable,
        }
    }

    /// Current thread status.
    #[must_use]
    pub const fn status(&self) -> ThreadStatus {
        self.status
    }

    /// Updates the thread status.
    pub fn set_status(&mut self, status: ThreadStatus) {
        self.status = status;
    }

    /// Marks a runnable or suspended thread as currently running.
    pub fn enter_running(&mut self) -> bool {
        if matches!(
            self.status,
            ThreadStatus::Runnable | ThreadStatus::Suspended
        ) {
            self.status = ThreadStatus::Running;
            true
        } else {
            false
        }
    }

    /// Marks a running thread as suspended by a coroutine yield.
    pub fn suspend(&mut self) -> bool {
        if self.status == ThreadStatus::Running {
            self.status = ThreadStatus::Suspended;
            true
        } else {
            false
        }
    }

    /// Marks a thread as finished or errored.
    pub fn finish(&mut self) {
        self.status = ThreadStatus::Dead;
    }

    /// Number of values on the stack.
    #[must_use]
    pub fn stack_len(&self) -> usize {
        self.stack.len()
    }

    /// Returns true when the stack is empty.
    #[must_use]
    pub fn is_stack_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Pushes a value onto the stack.
    pub fn push_value(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Pops a value from the stack.
    pub fn pop_value(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    /// Peeks at the top stack value.
    #[must_use]
    pub fn peek_value(&self) -> Option<Value> {
        self.stack.last().copied()
    }

    /// Gets a stack slot.
    #[must_use]
    pub fn stack_value(&self, index: StackIndex) -> Option<Value> {
        self.stack.get(index).copied()
    }

    /// Writes an existing stack slot.
    pub fn set_stack_value(&mut self, index: StackIndex, value: Value) -> bool {
        if let Some(slot) = self.stack.get_mut(index) {
            *slot = value;
            true
        } else {
            false
        }
    }

    /// Truncates the value stack.
    pub fn truncate_stack(&mut self, len: usize) {
        self.stack.truncate(len);
    }

    /// Number of active call frames.
    #[must_use]
    pub fn frame_len(&self) -> usize {
        self.frames.len()
    }

    /// Pushes a call frame if its range is valid for the current stack.
    pub fn push_frame(&mut self, frame: CallFrame) -> bool {
        if frame.base > frame.top || frame.top > self.stack.len() {
            return false;
        }
        self.frames.push(frame);
        true
    }

    /// Pops the current call frame.
    pub fn pop_frame(&mut self) -> Option<CallFrame> {
        self.frames.pop()
    }

    /// Current call frame.
    #[must_use]
    pub fn current_frame(&self) -> Option<&CallFrame> {
        self.frames.last()
    }
}

impl Default for LuaThread {
    fn default() -> Self {
        Self::new()
    }
}

impl GcObject for LuaThread {
    fn header(&self) -> &GcHeader {
        &self.header
    }
}

/// Lua thread status.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ThreadStatus {
    /// Ready to run.
    Runnable,
    /// Currently running.
    Running,
    /// Suspended by yield.
    Suspended,
    /// Finished or errored terminal state.
    Dead,
}

/// Number of results requested by a call frame.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResultCount {
    /// A fixed result count.
    Fixed(u16),
    /// All results.
    Multiple,
}

/// Placeholder frame flags.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct FrameFlags {
    bits: u16,
}

impl FrameFlags {
    /// Empty frame flags.
    pub const EMPTY: Self = Self { bits: 0 };
    /// Frame is a protected-call boundary.
    pub const PROTECTED: Self = Self { bits: 1 << 0 };

    /// Raw flag bits.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.bits
    }

    /// Returns true when no flags are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Returns true when this frame is a protected-call boundary.
    #[must_use]
    pub const fn is_protected(self) -> bool {
        self.bits & Self::PROTECTED.bits != 0
    }

    /// Returns these flags with the protected-call marker enabled.
    #[must_use]
    pub const fn with_protected(mut self) -> Self {
        self.bits |= Self::PROTECTED.bits;
        self
    }
}

/// Basic call frame metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CallFrame {
    /// Program counter inside the active prototype.
    pub pc: u32,
    /// Base stack slot for the frame.
    pub base: StackIndex,
    /// Exclusive top stack slot for the frame.
    pub top: StackIndex,
    /// Requested result count.
    pub wanted_results: ResultCount,
    /// Frame flags placeholder.
    pub flags: FrameFlags,
}

impl CallFrame {
    /// Creates a call frame.
    #[must_use]
    pub const fn new(base: StackIndex, top: StackIndex, wanted_results: ResultCount) -> Self {
        Self {
            pc: 0,
            base,
            top,
            wanted_results,
            flags: FrameFlags::EMPTY,
        }
    }

    /// Updates the program counter.
    #[must_use]
    pub const fn with_pc(mut self, pc: u32) -> Self {
        self.pc = pc;
        self
    }

    /// Marks this frame as a protected-call boundary.
    #[must_use]
    pub const fn protected(mut self) -> Self {
        self.flags = self.flags.with_protected();
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{CallFrame, GcKind, GcObject, LuaThread, ResultCount, ThreadStatus, Value, Vm};

    #[test]
    fn thread_stack_new_thread_is_gc_object() {
        let thread = LuaThread::new();

        assert_eq!(thread.header().kind(), GcKind::Thread);
        assert_eq!(thread.status(), ThreadStatus::Runnable);
        assert!(thread.is_stack_empty());
        assert_eq!(thread.frame_len(), 0);
    }

    #[test]
    fn thread_stack_push_pop_and_set_values() {
        let mut thread = LuaThread::new();

        thread.push_value(Value::integer(1));
        thread.push_value(Value::boolean(true));

        assert_eq!(thread.stack_len(), 2);
        assert_eq!(thread.peek_value(), Some(Value::boolean(true)));
        assert_eq!(thread.stack_value(0), Some(Value::integer(1)));
        assert!(thread.set_stack_value(0, Value::integer(2)));
        assert_eq!(thread.stack_value(0), Some(Value::integer(2)));
        assert_eq!(thread.pop_value(), Some(Value::boolean(true)));
        assert_eq!(thread.pop_value(), Some(Value::integer(2)));
        assert_eq!(thread.pop_value(), None);
    }

    #[test]
    fn thread_stack_frame_stack_validates_bounds() {
        let mut thread = LuaThread::new();
        thread.push_value(Value::nil());
        thread.push_value(Value::nil());

        assert!(thread.push_frame(CallFrame::new(0, 2, ResultCount::Fixed(1)).with_pc(12)));
        assert_eq!(thread.frame_len(), 1);
        assert_eq!(thread.current_frame().unwrap().pc, 12);
        assert!(!thread.push_frame(CallFrame::new(2, 1, ResultCount::Fixed(0))));
        assert!(!thread.push_frame(CallFrame::new(0, 3, ResultCount::Multiple)));
        assert_eq!(thread.pop_frame().unwrap().top, 2);
        assert_eq!(thread.pop_frame(), None);
    }

    #[test]
    fn thread_stack_frame_flags_mark_protected_boundaries() {
        let frame = CallFrame::new(0, 0, ResultCount::Multiple).protected();

        assert!(frame.flags.is_protected());
        assert!(!frame.flags.is_empty());
    }

    #[test]
    fn thread_stack_status_transitions_for_coroutines() {
        let mut thread = LuaThread::new();

        assert!(thread.enter_running());
        assert_eq!(thread.status(), ThreadStatus::Running);
        assert!(thread.suspend());
        assert_eq!(thread.status(), ThreadStatus::Suspended);
        assert!(thread.enter_running());
        thread.finish();
        assert_eq!(thread.status(), ThreadStatus::Dead);

        assert!(!thread.enter_running());
        assert!(!thread.suspend());
    }

    #[test]
    fn thread_stack_vm_owns_gc_arena() {
        let mut vm = Vm::new();
        let thread = vm.gc_mut().allocate(LuaThread::new());
        let root = vm.gc_mut().add_root(thread);

        assert_eq!(vm.gc().len(), 1);
        assert!(vm.gc().contains_root_for(root, thread));
    }
}
