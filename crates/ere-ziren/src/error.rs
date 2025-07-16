use std::{io, path::PathBuf, process::ExitStatus};
use thiserror::Error;
use zkvm_interface::zkVMError;

impl From<ZirenError> for zkVMError {
    fn from(value: ZirenError) -> Self {
        zkVMError::Other(Box::new(value))
    }
}

#[derive(Debug, Error)]
pub enum ZirenError {
    /// Errors that can be encountered while compiling a Ziren program
    #[error(transparent)]
    ExecuteError(#[from] ExecuteError),

    /// Errors that can be encountered while compiling a Ziren program
    #[error(transparent)]
    Prove(#[from] ProveError),

    /// Errors that can be encountered while compiling a Ziren program
    #[error(transparent)]
    Verify(#[from] VerifyError),

    /// Guest program directory does not exist.
    #[error("guest program directory not found: {0}")]
    PathNotFound(PathBuf),

    /// Failed to spawn or run `cargo pico build`.
    #[error("failed to run `cargo pico build`: {0}")]
    Spawn(#[from] io::Error),

    /// `cargo pico build` exited with a non-zero status.
    #[error("`cargo pico build` failed with status {status:?}")]
    CargoFailed { status: ExitStatus },

    /// Expected ELF file was not produced.
    #[error("ELF file not found at {0}")]
    ElfNotFound(PathBuf),

    /// Reading the ELF file failed.
    #[error("failed to read ELF file at {path}: {source}")]
    ReadElf {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}


#[derive(Debug, Error)]
pub enum ExecuteError {
    #[error("SP1 execution failed: {0}")]
    Client(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[derive(Debug, Error)]
pub enum ProveError {
    #[error("SP1 SDK proving failed: {0}")]
    Client(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("Serialising proof with `bincode` failed: {0}")]
    Bincode(#[from] bincode::Error),
}

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("Deserialising proof failed: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("SP1 SDK verification failed: {0}")]
    Client(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}