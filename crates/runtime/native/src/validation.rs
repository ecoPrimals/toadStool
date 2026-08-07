// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{ExecutionRequest, RuntimeConfig},
    workload::ExecutableSource,
};

pub fn resolve_executable(
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
                match toadstool::common::platform::check_access(
                    path,
                    toadstool::common::platform::PlatformAccess::Executable,
                ) {
                    Ok(true) => {}
                    Ok(false) => {
                        return Err(ToadStoolError::permission_denied(format!(
                            "File is not executable: {}",
                            path.display()
                        )));
                    }
                    Err(e) => {
                        return Err(ToadStoolError::io(format!(
                            "Failed to check permissions: {e}"
                        )));
                    }
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

pub fn validate_resource_requirements(request: &ExecutionRequest) -> ToadStoolResult<()> {
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
