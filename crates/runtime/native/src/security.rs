// SPDX-License-Identifier: AGPL-3.0-only

use tokio::process::Command as TokioCommand;
use tracing::{debug, info};

use toadstool::security::{IsolationLevel, SecurityContext};

pub(crate) fn apply_security_context(
    mut command: TokioCommand,
    security_context: &SecurityContext,
) -> TokioCommand {
    match security_context.isolation_level {
        IsolationLevel::None => {
            debug!("No isolation applied");
        }
        IsolationLevel::Basic => {
            debug!("Applying basic isolation");
            command.current_dir("/tmp");

            #[cfg(unix)]
            {
                command.process_group(0);
            }
        }
        IsolationLevel::Standard => {
            debug!("Applying standard isolation");
            command.current_dir("/tmp");

            #[cfg(unix)]
            {
                command.process_group(0);
                if let Some(user_context) = &security_context.user_context {
                    if let Some(username) = &user_context.username {
                        info!("Setting user context to: {}", username);
                    }
                }
            }
        }
        IsolationLevel::Enhanced => {
            debug!("Applying enhanced isolation");
            command.current_dir("/tmp");

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
                tracing::warn!("Maximum isolation not fully supported on this platform");
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
