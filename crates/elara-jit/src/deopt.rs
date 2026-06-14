//! Deoptimization metadata and interpreter handoff helpers.

use std::{error::Error, fmt};

use elara_bytecode::Proto;
use elara_core::{LuaThread, Value};
use elara_interp::execute_proto;

/// Stable identifier for a deoptimization point.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeoptPointId(u32);

impl DeoptPointId {
    /// Creates a deoptimization point id.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the dense id value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// One VM register that must be materialized when deoptimizing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LiveRegister {
    register: u16,
}

impl LiveRegister {
    /// Creates live-register metadata.
    #[must_use]
    pub const fn new(register: u16) -> Self {
        Self { register }
    }

    /// VM register index.
    #[must_use]
    pub const fn register(self) -> u16 {
        self.register
    }
}

/// Runtime value for one live register at a deoptimization point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveRegisterValue {
    register: u16,
    value: Value,
}

impl LiveRegisterValue {
    /// Creates a live-register value.
    #[must_use]
    pub const fn new(register: u16, value: Value) -> Self {
        Self { register, value }
    }

    /// VM register index.
    #[must_use]
    pub const fn register(self) -> u16 {
        self.register
    }

    /// Runtime value to materialize.
    #[must_use]
    pub const fn value(self) -> Value {
        self.value
    }
}

/// Metadata required to leave generated code and resume in the interpreter.
#[derive(Clone, Debug, PartialEq)]
pub struct DeoptPoint {
    id: DeoptPointId,
    pc: usize,
    stack_slots: u16,
    live_registers: Box<[LiveRegister]>,
}

impl DeoptPoint {
    /// Creates deoptimization metadata.
    #[must_use]
    pub fn new(
        id: DeoptPointId,
        pc: usize,
        stack_slots: u16,
        live_registers: impl Into<Box<[LiveRegister]>>,
    ) -> Self {
        Self {
            id,
            pc,
            stack_slots,
            live_registers: live_registers.into(),
        }
    }

    /// Deoptimization point id.
    #[must_use]
    pub const fn id(&self) -> DeoptPointId {
        self.id
    }

    /// Interpreter program counter to resume from.
    #[must_use]
    pub const fn pc(&self) -> usize {
        self.pc
    }

    /// Stack slots required by this deoptimization point.
    #[must_use]
    pub const fn stack_slots(&self) -> u16 {
        self.stack_slots
    }

    /// Live register metadata.
    #[must_use]
    pub fn live_registers(&self) -> &[LiveRegister] {
        &self.live_registers
    }

    /// Synchronizes live values into a VM thread stack.
    pub fn sync_to_thread(
        &self,
        thread: &mut LuaThread,
        values: &[LiveRegisterValue],
    ) -> Result<(), DeoptError> {
        if values.len() != self.live_registers.len() {
            return Err(DeoptError::LiveValueCountMismatch {
                expected: self.live_registers.len(),
                actual: values.len(),
            });
        }

        thread.resize_stack_with_nil(usize::from(self.stack_slots));
        for (metadata, value) in self.live_registers.iter().zip(values.iter().copied()) {
            if metadata.register() != value.register() {
                return Err(DeoptError::LiveRegisterMismatch {
                    expected: metadata.register(),
                    actual: value.register(),
                });
            }
            if value.register() >= self.stack_slots {
                return Err(DeoptError::RegisterOutOfBounds {
                    register: value.register(),
                    stack_slots: self.stack_slots,
                });
            }
            let wrote = thread.set_stack_value(usize::from(value.register()), value.value());
            debug_assert!(wrote, "stack was resized before live register sync");
        }
        Ok(())
    }
}

/// Synchronizes live registers and falls back to interpreter execution.
///
/// The current interpreter entry point executes a whole Proto. Later M17 work
/// will use `DeoptPoint::pc` to resume at a precise bytecode offset.
pub fn deopt_to_interpreter(
    proto: &Proto,
    point: &DeoptPoint,
    values: &[LiveRegisterValue],
) -> Result<Vec<Value>, DeoptError> {
    let mut thread = LuaThread::new();
    point.sync_to_thread(&mut thread, values)?;
    execute_proto(proto).map_err(|error| DeoptError::Interpreter(error.to_string()))
}

/// Deoptimization metadata or handoff error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeoptError {
    /// Live value count does not match deopt metadata.
    LiveValueCountMismatch {
        /// Expected value count.
        expected: usize,
        /// Actual value count.
        actual: usize,
    },
    /// Live value register does not match deopt metadata.
    LiveRegisterMismatch {
        /// Expected register.
        expected: u16,
        /// Actual register.
        actual: u16,
    },
    /// Live register does not fit in the target stack.
    RegisterOutOfBounds {
        /// Register index.
        register: u16,
        /// Available stack slots.
        stack_slots: u16,
    },
    /// Interpreter fallback returned an error.
    Interpreter(String),
}

impl fmt::Display for DeoptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LiveValueCountMismatch { expected, actual } => {
                write!(
                    f,
                    "deopt live value count mismatch: expected {expected}, got {actual}"
                )
            }
            Self::LiveRegisterMismatch { expected, actual } => {
                write!(
                    f,
                    "deopt live register mismatch: expected r{expected}, got r{actual}"
                )
            }
            Self::RegisterOutOfBounds {
                register,
                stack_slots,
            } => {
                write!(
                    f,
                    "deopt register r{register} is outside {stack_slots} stack slots"
                )
            }
            Self::Interpreter(error) => f.write_str(error),
        }
    }
}

impl Error for DeoptError {}

#[cfg(test)]
mod tests {
    use elara_bytecode::{Op, ProtoBuilder};
    use elara_core::{LuaThread, Value};
    use elara_interp::execute_proto;

    use super::{
        DeoptError, DeoptPoint, DeoptPointId, LiveRegister, LiveRegisterValue, deopt_to_interpreter,
    };

    #[test]
    fn deopt_syncs_live_registers_to_vm_stack() {
        let point = DeoptPoint::new(
            DeoptPointId::new(3),
            7,
            3,
            [LiveRegister::new(0), LiveRegister::new(2)],
        );
        let mut thread = LuaThread::new();

        point
            .sync_to_thread(
                &mut thread,
                &[
                    LiveRegisterValue::new(0, Value::integer(42)),
                    LiveRegisterValue::new(2, Value::boolean(true)),
                ],
            )
            .expect("live registers should sync");

        assert_eq!(thread.stack_len(), 3);
        assert_eq!(thread.stack_value(0), Some(Value::integer(42)));
        assert_eq!(thread.stack_value(1), Some(Value::nil()));
        assert_eq!(thread.stack_value(2), Some(Value::boolean(true)));
        assert_eq!(point.id().get(), 3);
        assert_eq!(point.pc(), 7);
    }

    #[test]
    fn deopt_rejects_mismatched_live_registers() {
        let point = DeoptPoint::new(DeoptPointId::new(0), 0, 2, [LiveRegister::new(1)]);
        let mut thread = LuaThread::new();

        assert_eq!(
            point.sync_to_thread(&mut thread, &[LiveRegisterValue::new(0, Value::integer(1))]),
            Err(DeoptError::LiveRegisterMismatch {
                expected: 1,
                actual: 0,
            })
        );
    }

    #[test]
    fn deopt_rejects_out_of_bounds_live_registers() {
        let point = DeoptPoint::new(DeoptPointId::new(0), 0, 1, [LiveRegister::new(1)]);
        let mut thread = LuaThread::new();

        assert_eq!(
            point.sync_to_thread(&mut thread, &[LiveRegisterValue::new(1, Value::integer(1))]),
            Err(DeoptError::RegisterOutOfBounds {
                register: 1,
                stack_slots: 1,
            })
        );
    }

    #[test]
    fn deopt_to_interpreter_falls_back_to_proto_execution() {
        let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
        let constant = builder.add_constant(Value::integer(42));
        builder.emit_abx(Op::LoadK, 0, u64::from(constant));
        builder.emit_abc(Op::Return, 0, 1, 0);
        let proto = builder.finish();
        let point = DeoptPoint::new(DeoptPointId::new(0), 0, 1, [LiveRegister::new(0)]);
        let values = [LiveRegisterValue::new(0, Value::nil())];

        assert_eq!(
            deopt_to_interpreter(&proto, &point, &values),
            execute_proto(&proto).map_err(|error| DeoptError::Interpreter(error.to_string()))
        );
    }
}
