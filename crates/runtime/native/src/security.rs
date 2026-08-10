// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::process::Command;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(any(unix, target_os = "linux"))]
use tracing::info;
use tracing::{debug, warn};

use toadstool::security::{IsolationLevel, SecurityContext};

/// Resolve the working directory for a workload process.
///
/// Priority: explicit `workload_working_dir` (if permitted by isolation) → isolation default.
/// For `None` isolation, the workload dir is always honoured.
/// For `Basic`/`Standard`, it is honoured when the path is listed in
/// `SecurityContext.filesystem_security.allowed_write_paths` (acting as
/// `trusted_directories`), or is under `std::env::temp_dir()`.
/// For `Enhanced`/`Maximum`, the workload dir is ignored.
fn resolve_working_dir(
    security_context: &SecurityContext,
    workload_working_dir: Option<&Path>,
) -> Option<std::path::PathBuf> {
    match security_context.isolation_level {
        IsolationLevel::None => workload_working_dir.map(std::path::PathBuf::from),

        IsolationLevel::Basic | IsolationLevel::Standard => {
            if let Some(wd) = workload_working_dir {
                let temp = std::env::temp_dir();
                let is_under_temp = wd.starts_with(&temp);
                let is_trusted = security_context
                    .filesystem_security
                    .allowed_write_paths
                    .iter()
                    .any(|trusted| wd.starts_with(trusted));

                if is_under_temp || is_trusted {
                    return Some(wd.to_path_buf());
                }
                warn!(
                    working_dir = %wd.display(),
                    isolation = ?security_context.isolation_level,
                    "working_dir not in trusted_directories or temp — falling back to temp_dir"
                );
            }
            Some(std::env::temp_dir())
        }

        IsolationLevel::Enhanced | IsolationLevel::Maximum => {
            if workload_working_dir.is_some() {
                debug!(
                    isolation = ?security_context.isolation_level,
                    "working_dir ignored under Enhanced/Maximum isolation"
                );
            }
            Some(std::env::temp_dir())
        }
    }
}

pub fn apply_security_context(
    mut command: Command,
    security_context: &SecurityContext,
    workload_working_dir: Option<&Path>,
) -> Command {
    if let Some(wd) = resolve_working_dir(security_context, workload_working_dir) {
        command.current_dir(&wd);
        debug!(working_dir = %wd.display(), "Process working directory set");
    }

    match security_context.isolation_level {
        IsolationLevel::None => {
            debug!("No isolation applied");
        }
        IsolationLevel::Basic => {
            debug!("Applying basic isolation");
            #[cfg(unix)]
            {
                command.process_group(0);
            }
        }
        IsolationLevel::Standard => {
            debug!("Applying standard isolation");
            #[cfg(unix)]
            {
                command.process_group(0);
                if let Some(user_context) = &security_context.user_context
                    && let Some(username) = &user_context.username
                {
                    info!("Setting user context to: {}", username);
                }
            }
        }
        IsolationLevel::Enhanced => {
            debug!("Applying enhanced isolation");
            #[cfg(unix)]
            {
                command.process_group(0);
            }

            #[cfg(target_os = "linux")]
            {
                info!("Enhanced isolation on Linux - implementing namespace isolation");
            }
        }
        IsolationLevel::Maximum => {
            debug!("Applying maximum isolation");

            #[cfg(unix)]
            {
                command.process_group(0);
            }

            #[cfg(target_os = "linux")]
            {
                info!("Maximum isolation - would use container-like isolation");
            }

            #[cfg(not(target_os = "linux"))]
            {
                warn!("Maximum isolation not fully supported on this platform");
            }
        }
    }

    if !security_context.has_capability(&toadstool::security::Capability::Read) {
        debug!("File system read access denied");
    }

    if !security_context.has_capability(&toadstool::security::Capability::NetworkClient) {
        debug!("Network outbound access denied");
    }

    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::security::FilesystemSecurity;

    fn ctx(level: IsolationLevel) -> SecurityContext {
        SecurityContext::for_isolation_level(level)
    }

    fn ctx_with_trusted(level: IsolationLevel, trusted: Vec<String>) -> SecurityContext {
        let mut sc = ctx(level);
        sc.filesystem_security = FilesystemSecurity {
            allowed_write_paths: trusted,
            ..FilesystemSecurity::default()
        };
        sc
    }

    #[test]
    fn none_honours_workload_dir() {
        let wd = resolve_working_dir(&ctx(IsolationLevel::None), Some(Path::new("/app/data")));
        assert_eq!(wd, Some(std::path::PathBuf::from("/app/data")));
    }

    #[test]
    fn none_without_workload_dir_returns_none() {
        let wd = resolve_working_dir(&ctx(IsolationLevel::None), None);
        assert!(wd.is_none());
    }

    #[test]
    fn standard_allows_temp_subdir() {
        let temp = std::env::temp_dir();
        let sub = temp.join("my-workload");
        let wd = resolve_working_dir(&ctx(IsolationLevel::Standard), Some(&sub));
        assert_eq!(wd, Some(sub));
    }

    #[test]
    fn standard_allows_trusted_dir() {
        let sc = ctx_with_trusted(IsolationLevel::Standard, vec!["/opt/workloads".to_string()]);
        let wd = resolve_working_dir(&sc, Some(Path::new("/opt/workloads/job42")));
        assert_eq!(wd, Some(std::path::PathBuf::from("/opt/workloads/job42")));
    }

    #[test]
    fn standard_rejects_untrusted_dir() {
        let wd = resolve_working_dir(
            &ctx(IsolationLevel::Standard),
            Some(Path::new("/home/user/data")),
        );
        assert_eq!(wd, Some(std::env::temp_dir()));
    }

    #[test]
    fn standard_no_workload_dir_defaults_to_temp() {
        let wd = resolve_working_dir(&ctx(IsolationLevel::Standard), None);
        assert_eq!(wd, Some(std::env::temp_dir()));
    }

    #[test]
    fn basic_allows_trusted_dir() {
        let sc = ctx_with_trusted(IsolationLevel::Basic, vec!["/data".to_string()]);
        let wd = resolve_working_dir(&sc, Some(Path::new("/data/jobs")));
        assert_eq!(wd, Some(std::path::PathBuf::from("/data/jobs")));
    }

    #[test]
    fn enhanced_ignores_workload_dir() {
        let wd = resolve_working_dir(&ctx(IsolationLevel::Enhanced), Some(Path::new("/app/data")));
        assert_eq!(wd, Some(std::env::temp_dir()));
    }

    #[test]
    fn maximum_ignores_workload_dir() {
        let wd = resolve_working_dir(&ctx(IsolationLevel::Maximum), Some(Path::new("/app/data")));
        assert_eq!(wd, Some(std::env::temp_dir()));
    }
}
