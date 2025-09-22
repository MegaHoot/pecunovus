//! Program loader: loads BPF or WASM bytecode into memory.
//!
//! In pecunovus, programs are deployed and cached. Here we simulate that.

use anyhow::Result;

#[derive(Debug, Clone)]
pub enum LoadedProgram {
    Bpf(Vec<u8>),
    Wasm(Vec<u8>),
}

pub struct ProgramLoader;

impl ProgramLoader {
    pub fn load_bpf(bytes: Vec<u8>) -> Result<LoadedProgram> {
        // TODO: verify ELF format, sandbox checks
        Ok(LoadedProgram::Bpf(bytes))
    }

    pub fn load_wasm(bytes: Vec<u8>) -> Result<LoadedProgram> {
        // TODO: verify WASM format, instrumentation
        Ok(LoadedProgram::Wasm(bytes))
    }
}
