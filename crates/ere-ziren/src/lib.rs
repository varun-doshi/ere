use std::{process::Command, time::Instant};
use zkm_sdk::{ProverClient, ProverClientBuilder, ZKMProofWithPublicValues, ZKMStdin, include_elf};
use zkvm_interface::{
    Compiler, Input, InputItem, ProgramExecutionReport, ProgramProvingReport, ProverResourceType,
    zkVM, zkVMError,
};

include!(concat!(env!("OUT_DIR"), "/name_and_sdk_version.rs"));
mod error;
use error::{ExecuteError, ProveError, VerifyError, ZirenError};

#[allow(non_camel_case_types)]
pub struct ZIREN_TARGET;

impl Compiler for ZIREN_TARGET {
    type Error = ZirenError;

    type Program = Vec<u8>;

    fn compile(path: &std::path::Path) -> Result<Self::Program, Self::Error> {
        // 1. Check guest path
        if !path.exists() {
            return Err(ZirenError::PathNotFound(path.to_path_buf()));
        }

        // 2. Run `cargo build --release`
        let status = Command::new("cargo")
            .current_dir(path)
            .env("RUST_LOG", "info")
            .args(["build", "--release"])
            .status()?; // From<io::Error> → Spawn

        if !status.success() {
            return Err(ZirenError::CargoFailed { status });
        }

        // 3. Locate the ELF file
        let elf_path = path.join("elf/mipsel-zkm-zkvm-elf");

        if !elf_path.exists() {
            return Err(ZirenError::ElfNotFound(elf_path));
        }

        // 4. Read the ELF file
        let elf_bytes = std::fs::read(&elf_path).map_err(|e| ZirenError::ReadElf {
            path: elf_path,
            source: e,
        })?;

        Ok(elf_bytes)
    }
}

pub struct EreZiren {
    program: <ZIREN_TARGET as Compiler>::Program,
}

impl EreZiren {
    pub fn new(
        program_bytes: <ZIREN_TARGET as Compiler>::Program,
        _resource_type: ProverResourceType,
    ) -> Self {
        EreZiren {
            program: program_bytes,
        }
    }
}
impl zkVM for EreZiren {
    fn execute(&self, inputs: &Input) -> Result<ProgramExecutionReport, zkVMError> {
        let client = ProverClient::new();

        let mut stdin = ZKMStdin::new();
        for input in inputs.iter() {
            match input {
                InputItem::Object(serialize) => stdin.write(serialize),
                InputItem::Bytes(items) => stdin.write_slice(items),
            }
        }

        let start = Instant::now();
        let execute_result = client
            .execute(&self.program, stdin)
            .run()
            .map_err(|err| ZirenError::ExecuteError(ExecuteError::Client(err.into())))?;

        Ok(ProgramExecutionReport {
            total_num_cycles: execute_result.1.total_instruction_count(),
            execution_duration: start.elapsed(),
            ..Default::default()
        })
    }

    fn prove(
        &self,
        inputs: &Input,
    ) -> Result<(Vec<u8>, zkvm_interface::ProgramProvingReport), zkVMError> {
        let client = ProverClient::new();

        let mut stdin = ZKMStdin::new();
        for input in inputs.iter() {
            match input {
                InputItem::Object(serialize) => stdin.write(serialize),
                InputItem::Bytes(items) => stdin.write_slice(items),
            }
        }

        // Setup the program.
        let (pk, _) = client.setup(&self.program);

        let now = std::time::Instant::now();
        let proof = client
            .prove(&pk, stdin)
            .compressed()
            .run()
            .map_err(|e| ZirenError::Prove(error::ProveError::Client(e.into())))?;
        let elapsed = now.elapsed();

        let bytes = bincode::serialize(&proof)
            .map_err(|err| ZirenError::Prove(ProveError::Bincode(err)))?;

        Ok((bytes, ProgramProvingReport::new(elapsed)))
    }

    fn verify(&self, proof: &[u8]) -> Result<(), zkVMError> {
        let client = ProverClient::new();
        let (_, vk) = client.setup(&self.program);

        let proof: ZKMProofWithPublicValues = bincode::deserialize(proof)
            .map_err(|err| ZirenError::Verify(VerifyError::Bincode(err)))?;

        client
            .verify(&proof, &vk)
            .map_err(|err| ZirenError::Verify(VerifyError::Client(err.into())).into())
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn sdk_version(&self) -> &'static str {
        SDK_VERSION
    }
}

#[cfg(test)]
mod execute_tests {
    use std::path::PathBuf;

    use super::*;
    use zkvm_interface::Input;

    fn get_compile_test_guest_program_path() -> PathBuf {
        let workspace_dir = env!("CARGO_WORKSPACE_DIR");
        let path = PathBuf::from(workspace_dir)
            .join("tests")
            .join("ziren")
            .join("compile")
            .join("basic")
            .join("app");

        println!(
            "Attempting to find test guest program at: {}",
            path.display()
        );
        println!("Workspace dir is: {}", workspace_dir);

        path.canonicalize()
            .expect("Failed to find or canonicalize test guest program at <CARGO_WORKSPACE_DIR>/tests/pico/compile/basic/app")
    }

    #[test]
    fn test_compile_trait() {
        let test_guest_path = get_compile_test_guest_program_path();
        println!("Using test guest path: {}", test_guest_path.display());

        match ZIREN_TARGET::compile(&test_guest_path) {
            Ok(elf_bytes) => {
                assert!(!elf_bytes.is_empty(), "ELF bytes should not be empty.");
            }
            Err(e) => {
                panic!(
                    "compile_ziren_program direct call failed for dedicated guest: {:?}",
                    e
                );
            }
        }
    }
}
