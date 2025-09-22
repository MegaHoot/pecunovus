//! Minimal BPF VM placeholder.
//!
//! In production, integrate with rbpf (pecunovus’s BPF interpreter).

use crate::runtime::runtime_types::{ExecutionContext, RuntimeError};
use crate::runtime::program_loader::LoadedProgram;

pub struct BpfVm;

impl BpfVm {
    pub fn execute(ctx: &mut ExecutionContext, program: &LoadedProgram) -> Result<(), RuntimeError> {
        match program {
            LoadedProgram::Bpf(_bytes) => {
                // TODO: run program using rbpf
                Ok(())
            }
            _ => Err(RuntimeError::InvalidProgram),
        }
    }
}
