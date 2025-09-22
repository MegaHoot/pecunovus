//! Common runtime types: ExecutionContext, errors.

use crate::state::account_db::AccountKey;

#[derive(Debug)]
pub enum RuntimeError {
    InvalidProgram,
    ExecutionFailed(String),
}

#[derive(Debug)]
pub struct ExecutionContext {
    pub caller: AccountKey,
    pub program_id: AccountKey,
    pub params: Vec<u8>,
    pub result: Option<Vec<u8>>,
}

impl ExecutionContext {
    pub fn new(caller: AccountKey, program_id: AccountKey, params: Vec<u8>) -> Self {
        Self { caller, program_id, params, result: None }
    }
}
