//! Runtime module: executes smart contracts and native transactions.
//!
//! Exposes:
//! - Executor: orchestrates transaction execution (parallel, lock-based).
//! - ProgramLoader: loads BPF/WASM programs.
//! - BpfVm/WasmVm: interpreters for contract bytecode.
//! - runtime_types: common types used in execution.

pub mod executor;
pub mod program_loader;
pub mod bpf_vm;
pub mod wasm_vm;
pub mod runtime_types;

pub use executor::{Executor, Transaction, Receipt};
pub use program_loader::{ProgramLoader, LoadedProgram};
pub use bpf_vm::BpfVm;
pub use wasm_vm::WasmVm;
pub use runtime_types::{RuntimeError, ExecutionContext};
