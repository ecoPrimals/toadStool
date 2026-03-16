// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{ExecutionRequest, RuntimeConfig},
    workload::ExecutableSource,
};

pub(crate) fn resolve_executable(
    _config: &RuntimeConfig,
    source: &ExecutableSource,
) -> ToadStoolResult<PathBuf> {
    match source {
        ExecutableSource::File { path } => {
            if !path.exists() {
                return Err(ToadStoolError::not_found(format!(
                    "Executable not found: {}",
                    path.display()
                )));
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(path)
                    .map_err(|e| ToadStoolError::io(format!("Failed to read metadata: {e}")))?;
                let permissions = metadata.permissions();
                if permissions.mode() & 0o111 == 0 {
                    return Err(ToadStoolError::permission_denied(format!(
                        "File is not executable: {}",
                        path.display()
                    )));
                }
            }

            Ok(path.clone())
        }
        ExecutableSource::Url { url: _ } => Err(ToadStoolError::not_supported(
            "URL-based executables not yet supported",
        )),
        ExecutableSource::Bytes { data: _ } => Err(ToadStoolError::not_supported(
            "Byte-based executables not yet supported",
        )),
    }
}

pub(crate) fn validate_resource_requirements(request: &ExecutionRequest) -> ToadStoolResult<()> {
    let requirements = &request.resources;

    if requirements.cpu.min_cores > 32.0 {
        return Err(ToadStoolError::resource(
            "Requested CPU cores exceed system limits",
        ));
    }

    if requirements.memory.min_bytes > 128 * 1024 * 1024 * 1024 {
        return Err(ToadStoolError::resource(
            "Requested memory exceeds system limits",
        ));
    }

    Ok(())
}
