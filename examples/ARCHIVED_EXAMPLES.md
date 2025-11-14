# Archived Examples

**Date**: November 13, 2025  
**Reason**: Reference non-existent runtime crates

---

## Archived Files

The following example files have been removed from the build but preserved in the repository for historical reference:

### 1. edge_runtime_comprehensive_demo.rs
- **Reason**: References `toadstool_runtime_edge` crate which doesn't exist
- **Status**: Archived (removed from Cargo.toml)
- **Location**: `examples/edge_runtime_comprehensive_demo.rs`
- **Action Needed**: Update to use existing `toadstool-runtime-gpu` or similar, or delete

### 2. legacy_systems_comprehensive_demo.rs
- **Reason**: References `toadstool_runtime_legacy` crate which doesn't exist
- **Also**: Has API mismatches with current types (field names changed)
- **Status**: Archived (removed from Cargo.toml)
- **Location**: `examples/legacy_systems_comprehensive_demo.rs`
- **Action Needed**: Update to current API or delete

---

## How to Re-enable

If you want to re-enable these examples:

1. **Create the missing crates** in `crates/runtime/`:
   - `crates/runtime/edge/` for edge runtime
   - `crates/runtime/legacy/` for legacy systems

2. **Update the example code** to match current API:
   - Fix struct field names
   - Update type imports
   - Fix compilation errors

3. **Re-enable in Cargo.toml**:
   ```toml
   [[bin]]
   name = "edge_runtime_comprehensive_demo"
   path = "edge_runtime_comprehensive_demo.rs"
   ```

---

## Alternative

These examples could be rewritten to use existing runtimes:
- Use `toadstool-runtime-gpu` instead of `toadstool-runtime-edge`
- Use `toadstool-runtime-specialty` (if it exists) instead of `toadstool-runtime-legacy`
- Or simply demonstrate concepts without the specialized runtime crates

---

**Status**: Archived, not deleted (can be restored if needed)

