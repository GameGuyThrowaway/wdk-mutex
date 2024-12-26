//! Error types used by wdk-mutex

#[derive(Debug, PartialEq, Eq)]
pub enum DriverMutexError {
    IrqlTooHigh,
    IrqlNotAPCLevel,
    PagedPoolAllocFailed,
}