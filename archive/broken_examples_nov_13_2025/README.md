# Broken Examples Archive - November 13, 2025

These example files were archived because they failed to compile due to API changes.

## Files Archived

1. `current_api_demo.rs` - RetryConfig type mismatch (DistributedRetryConfig vs RetryConfig)
2. `hardcoding_elimination_example.rs` - Missing types: SelfIdentity, InfantDiscovery, ConcreteUniversalAdapter
3. `universal_compute_platform_demo.rs` - Unresolved import: RetryConfig
4. `universal_substrate_demo.rs` - Import issues
5. `universal_substrate_demonstration.rs` - Import issues
6. `ecosystem_massive_job_demo.rs` - Import issues

## Why Archived

These examples were written for an older API and haven't been updated to match current implementation. Rather than maintain outdated examples, they have been archived for reference.

## Working Examples

See the `examples/` directory for current, working examples:
- `simple_workload_demo.rs`
- `distributed_job_demo.rs`
- `cooperative_network_demo.rs`
- `universal_compute_demo.rs`
- `standalone_universal_compute.rs`
- And others...

## If You Need These

To update these examples:
1. Review current API in `crates/` directories
2. Update type names and imports
3. Test compilation with `cargo test --example <name>`
4. Move back to `examples/` directory

**Date Archived**: November 13, 2025  
**Reason**: API evolution, compilation failures  
**Action**: Kept for reference, not maintained

