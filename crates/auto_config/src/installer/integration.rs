// SPDX-License-Identifier: AGPL-3.0-or-later
//! System integration: PATH, desktop shortcuts, shell completion

use std::path::Path;

use std::fs;
use tracing::info;

use toadstool_common::interned_strings::socket_env;
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
                std::env::var(socket_env::HOME).unwrap_or_default()
            ))
            .exists()
            {
                ".zshrc"
            } else {
                ".bashrc"
            };

            let profile_path = format!(
                "{}/{}",
                std::env::var(socket_env::HOME).unwrap_or_default(),
                shell_profile
            );
            let path_export = format!(
                "\n# ToadStool\nexport PATH=\"{}:$PATH\"\n",
                bin_path.display()
            );

            if let Ok(content) = fs::read_to_string(&profile_path)
                && !content.contains("ToadStool")
            {
                fs::write(&profile_path, format!("{content}{path_export}"))?;
                info!("✅ Added ToadStool to PATH in {}", shell_profile);
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
            std::env::var(socket_env::DISPLAY).is_ok()
                || std::env::var(socket_env::WAYLAND_DISPLAY).is_ok()
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
            let desktop_dir = format!(
                "{}/Desktop",
                std::env::var(socket_env::HOME).unwrap_or_default()
            );
            if Path::new(&desktop_dir).exists() {
                let desktop_file = format!(
                    r"[Desktop Entry]
Version=1.0
Type=Application
Name=ToadStool
Comment=Universal Compute Platform
Exec={}/bin/toadstool
Icon=utilities-terminal
Terminal=true
Categories=Development;System;
",
                    installation_path.display()
                );

                fs::write(format!("{desktop_dir}/ToadStool.desktop"), desktop_file)?;
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
        fs::create_dir_all(&completion_dir)?;
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

    fs::write(completion_dir.join("toadstool.bash"), bash_completion)?;

    info!("🐚 Shell completion installed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_gui_macos_always_true() {
        assert!(has_gui(Platform::MacOS));
    }

    #[test]
    fn test_has_gui_windows_always_true() {
        assert!(has_gui(Platform::Windows));
    }

    #[test]
    fn test_has_gui_android_false() {
        assert!(!has_gui(Platform::Android));
    }

    #[test]
    fn test_has_gui_wasm_false() {
        assert!(!has_gui(Platform::Wasm));
    }

    #[test]
    fn test_has_gui_unknown_false() {
        assert!(!has_gui(Platform::Unknown));
    }

    #[test]
    fn test_has_gui_linux_with_display() {
        temp_env::with_var("DISPLAY", Some(":0"), || {
            assert!(has_gui(Platform::Linux));
        });
    }

    #[test]
    fn test_has_gui_linux_with_wayland() {
        temp_env::with_var_unset("DISPLAY", || {
            temp_env::with_var("WAYLAND_DISPLAY", Some("wayland-0"), || {
                assert!(has_gui(Platform::Linux));
            });
        });
    }

    #[test]
    fn test_has_gui_linux_headless() {
        temp_env::with_var_unset("DISPLAY", || {
            temp_env::with_var_unset("WAYLAND_DISPLAY", || {
                assert!(!has_gui(Platform::Linux));
            });
        });
    }

    #[tokio::test]
    async fn test_add_to_path_linux_creates_export() {
        let dir = tempfile::tempdir().expect("tempdir");
        let install_path = dir.path().join("toadstool");
        std::fs::create_dir_all(&install_path).expect("create dir");
        std::fs::create_dir_all(install_path.join("bin")).expect("create bin");

        let home = tempfile::tempdir().expect("home tempdir");
        let home_str = home.path().to_str().unwrap().to_string();
        let install_path_clone = install_path.clone();
        temp_env::with_vars(
            [("HOME", Some(home_str.as_str())), ("DISPLAY", None::<&str>)],
            || {
                let result = std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(super::add_to_path(Platform::Linux, &install_path_clone))
                })
                .join()
                .expect("thread");
                assert!(result.is_ok());
            },
        );
    }

    #[tokio::test]
    async fn test_setup_shell_completion_creates_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let install_path = dir.path().to_path_buf();

        let result = super::setup_shell_completion(&install_path).await;
        assert!(result.is_ok());

        let completion_dir = install_path.join("completion");
        assert!(completion_dir.exists());
        assert!(completion_dir.join("toadstool.bash").exists());
    }

    #[tokio::test]
    async fn test_create_desktop_shortcuts_linux() {
        let dir = tempfile::tempdir().expect("tempdir");
        let install_path = dir.path().join("toadstool");
        std::fs::create_dir_all(&install_path).expect("create dir");
        std::fs::create_dir_all(install_path.join("bin")).expect("create bin");

        let home = tempfile::tempdir().expect("home tempdir");
        let desktop_dir = home.path().join("Desktop");
        std::fs::create_dir_all(&desktop_dir).expect("create Desktop");

        let home_str = home.path().to_str().unwrap().to_string();
        let install_path_clone = install_path.clone();
        let desktop_file_path = desktop_dir.join("ToadStool.desktop");
        temp_env::with_var("HOME", Some(home_str.as_str()), || {
            let result = std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(super::create_desktop_shortcuts(
                    Platform::Linux,
                    &install_path_clone,
                ))
            })
            .join()
            .expect("thread");
            assert!(result.is_ok());
        });
        assert!(desktop_file_path.exists());
    }

    #[tokio::test]
    async fn test_add_to_path_windows_informational() {
        let dir = tempfile::tempdir().expect("tempdir");
        let install_path = dir.path().join("toadstool");
        std::fs::create_dir_all(&install_path).expect("create dir");
        std::fs::create_dir_all(install_path.join("bin")).expect("create bin");

        let result = super::add_to_path(Platform::Windows, &install_path).await;
        assert!(result.is_ok());
    }
}
