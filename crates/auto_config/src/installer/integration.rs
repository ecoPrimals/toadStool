//! System integration: PATH, desktop shortcuts, shell completion

use std::path::Path;

use tokio::fs;
use tracing::info;

use toadstool_common::platform_paths::Platform;

use crate::ToadStoolError;

/// Add ToadStool to system PATH
pub async fn add_to_path(
    platform: Platform,
    installation_path: &Path,
) -> Result<(), ToadStoolError> {
    let bin_path = installation_path.join("bin");

    match platform {
        Platform::Linux | Platform::MacOS => {
            let shell_profile = if Path::new(&format!(
                "{}/.zshrc",
                std::env::var("HOME").unwrap_or_default()
            ))
            .exists()
            {
                ".zshrc"
            } else {
                ".bashrc"
            };

            let profile_path = format!(
                "{}/{}",
                std::env::var("HOME").unwrap_or_default(),
                shell_profile
            );
            let path_export = format!(
                "\n# ToadStool\nexport PATH=\"{}:$PATH\"\n",
                bin_path.display()
            );

            if let Ok(content) = fs::read_to_string(&profile_path).await {
                if !content.contains("ToadStool") {
                    fs::write(&profile_path, format!("{content}{path_export}")).await?;
                    info!("✅ Added ToadStool to PATH in {}", shell_profile);
                }
            }
        }
        Platform::Windows => {
            info!(
                "💡 Please add {} to your PATH environment variable",
                bin_path.display()
            );
        }
        Platform::Android | Platform::Wasm | Platform::Unknown => {}
    }

    Ok(())
}

/// Check if GUI is available
pub fn has_gui(platform: Platform) -> bool {
    match platform {
        Platform::Linux => {
            std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
        }
        Platform::MacOS | Platform::Windows => true,
        Platform::Android | Platform::Wasm | Platform::Unknown => false,
    }
}

/// Create desktop shortcuts
pub async fn create_desktop_shortcuts(
    platform: Platform,
    installation_path: &Path,
) -> Result<(), ToadStoolError> {
    info!("🖥️ Creating desktop shortcuts...");

    match platform {
        Platform::Linux => {
            let desktop_dir = format!("{}/Desktop", std::env::var("HOME").unwrap_or_default());
            if Path::new(&desktop_dir).exists() {
                let desktop_file = format!(
                    r#"[Desktop Entry]
Version=1.0
Type=Application
Name=ToadStool
Comment=Universal Compute Platform
Exec={}/bin/toadstool
Icon=utilities-terminal
Terminal=true
Categories=Development;System;
"#,
                    installation_path.display()
                );

                fs::write(format!("{desktop_dir}/ToadStool.desktop"), desktop_file).await?;
            }
        }
        Platform::MacOS => {
            info!("💡 macOS desktop shortcuts not implemented yet");
        }
        Platform::Windows => {
            info!("💡 Windows desktop shortcuts not implemented yet");
        }
        Platform::Android | Platform::Wasm | Platform::Unknown => {}
    }

    Ok(())
}

/// Setup shell completion
pub async fn setup_shell_completion(installation_path: &Path) -> Result<(), ToadStoolError> {
    info!("🐚 Setting up shell completion...");

    let completion_dir = installation_path.join("completion");
    if !completion_dir.exists() {
        fs::create_dir_all(&completion_dir).await?;
    }

    let bash_completion = r#"# ToadStool completion
_toadstool_complete() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="status config run help daemon"
    
    if [[ ${cur} == -* ]]; then
        COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
        return 0
    fi
    
    case "${prev}" in
        run)
            COMPREPLY=( $(compgen -f -- ${cur}) )
            return 0
            ;;
        *)
            COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            return 0
            ;;
    esac
}

complete -F _toadstool_complete toadstool
"#;

    fs::write(completion_dir.join("toadstool.bash"), bash_completion).await?;

    info!("🐚 Shell completion installed");
    Ok(())
}
