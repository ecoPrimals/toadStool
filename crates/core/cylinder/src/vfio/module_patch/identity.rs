// SPDX-License-Identifier: AGPL-3.0-or-later
use super::types::PatchError;

/// Rename the module identity inside a `.ko` binary.
///
/// Replaces occurrences of `old_name` with `new_name` in the module's
/// `.modinfo` and `.gnu.linkonce.this_module` sections. The new name must
/// be <= the old name's length (we pad with NUL bytes). This allows
/// `insmod` to load the module alongside an already-loaded copy with the
/// original name.
///
/// Returns the number of replacements made.
pub fn rename_module_identity(
    module_bytes: &mut [u8],
    old_name: &str,
    new_name: &str,
) -> Result<usize, PatchError> {
    if new_name.len() > old_name.len() {
        return Err(PatchError::NmFailed {
            path: "(in-memory)".into(),
            detail: format!(
                "new module name '{}' ({} bytes) exceeds old name '{}' ({} bytes)",
                new_name, new_name.len(), old_name, old_name.len(),
            ),
        });
    }

    let old_bytes = old_name.as_bytes();
    let mut new_padded = vec![0u8; old_bytes.len()];
    new_padded[..new_name.len()].copy_from_slice(new_name.as_bytes());

    let mut replacements = 0;
    let mut pos = 0;
    while pos + old_bytes.len() <= module_bytes.len() {
        if &module_bytes[pos..pos + old_bytes.len()] == old_bytes {
            let before = if pos > 0 { module_bytes[pos - 1] } else { 0 };
            let after_pos = pos + old_bytes.len();
            let after = if after_pos < module_bytes.len() {
                module_bytes[after_pos]
            } else {
                0
            };
            if before == 0 && (after == 0 || after == b'=') {
                module_bytes[pos..pos + old_bytes.len()].copy_from_slice(&new_padded);
                replacements += 1;
                tracing::debug!(
                    offset = format_args!("{pos:#x}"),
                    old = old_name,
                    new = new_name,
                    "renamed module identity"
                );
            }
        }
        pos += 1;
    }

    tracing::info!(
        old = old_name,
        new = new_name,
        replacements,
        "module identity rename complete"
    );

    Ok(replacements)
}
