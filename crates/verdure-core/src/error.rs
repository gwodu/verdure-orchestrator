//! One error type for the whole SDK. Variants are *categories a vendor's code
//! can act on* (retry the download, free memory, show a licence screen), each
//! carrying the plain detail. Engine jargon stays inside `Engine(_)` where the
//! integrator can log it; it is never the only thing a user would see.

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum VError {
    #[error("licence problem: {0}")]
    License(String),
    #[error("the policy forbids this: {0}")]
    PolicyDenied(String),
    #[error("network problem: {0}")]
    Network(String),
    #[error("downloaded file failed verification (expected {expected}, got {actual})")]
    Integrity { expected: String, actual: String },
    #[error("storage problem: {0}")]
    Storage(String),
    #[error("no bundle fits: {0}")]
    NoBundle(String),
    #[error("not enough memory: needs {needed_mb} MB, {available_mb} MB available")]
    NotEnoughMemory { needed_mb: u64, available_mb: u64 },
    #[error("not enough free storage: needs {needed_mb} MB, {free_mb} MB free")]
    NotEnoughDisk { needed_mb: u64, free_mb: u64 },
    #[error("no engine is available for this: {0}")]
    EngineUnavailable(String),
    #[error("the engine failed: {0}")]
    Engine(String),
    #[error("the schema could not be used: {0}")]
    InvalidSchema(String),
    #[error("the output did not match the schema: {}", violations.join("; "))]
    SchemaInvalid { violations: Vec<String> },
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("cancelled")]
    Cancelled,
}
