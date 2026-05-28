# toadStool S280 — Wave 59 Env Centralization + Clippy Allow Evolution

**Date:** May 28, 2026
**Audit Source:** primalSpring Wave 59 Mountain Blurb
**Status:** RESOLVED — 64% centralized, 0 bare clippy allows

---

## What was done

### Env var centralization (primalSpring audit: "~200 env sites, env_overrides.rs split needed")

**Phase 0 — Cleanup:**
- Deleted orphan `core/config/src/env_overrides.rs` (342 lines, 70 raw literals, not `mod`'d anywhere)
- Active split already exists at `runtime_defaults/env_overrides/` (7 submodules)

**Phase 1 — Registry expansion:**
- Added +73 env var name constants to `socket_env.rs` across 7 categories:
  - POSIX/XDG: `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME`, `TMPDIR`, `TMP`, `TEMP`, `USERPROFILE`, `USERNAME`, `APPDATA`, `HOSTNAME`
  - systemd: `NOTIFY_SOCKET`, `LISTEN_FDS`, `LISTEN_PID`, `LISTEN_FDNAMES`
  - Server identity: `TOADSTOOL_GATE_ID`, `TOADSTOOL_AUTH_MODE`, `TOADSTOOL_DEPLOYMENT_MODEL`
  - Domain discovery: `COMPUTE_DOMAIN`, `COORDINATION_DOMAIN`, `SECURITY_DOMAIN`, `STORAGE_DOMAIN`, `AI_PROCESSING_DOMAIN`, `BIOMEOS_DOMAIN` + deprecated `SONGBIRD_DOMAIN`, `BEARDOG_DOMAIN`, `NESTGATE_DOMAIN`, `SQUIRREL_DOMAIN`
  - Legacy endpoint aliases (deprecated): `TOADSTOOL_SONGBIRD_ENDPOINT`, `TOADSTOOL_BEARDOG_ENDPOINT`, `TOADSTOOL_NESTGATE_ENDPOINT`, `TOADSTOOL_SQUIRREL_ENDPOINT`
  - Crypto provider keys: `CRYPTO_PROVIDER_PUBLIC_KEY`, `STORAGE_PROVIDER_PUBLIC_KEY`, `DISCOVERY_PROVIDER_PUBLIC_KEY` + deprecated identity-based aliases
  - Cylinder/ember: `TOADSTOOL_EMBER_GATE`, `TOADSTOOL_DRI_RENDER_PREFIX`, `TOADSTOOL_EMBER_SOCKET`, `BIOMEOS_ECOSYSTEM_NAMESPACE` + deprecated `CORALREEF_*` aliases
  - DNS/config: `TOADSTOOL_DNS_SERVERS`, `DNS_SEARCH_DOMAINS`, `TEMP_DIR`, `HEADLESS`, `HW_LEARN_STORE`, `SHADER_COMPILER_ADDR`, `CI`, `VK_ICD_FILENAMES`, `SECURITY_WARNING_ACKNOWLEDGED`

**Phase 2 — Migration (117 raw sites → constants across 30 files):**

| Crate | Files | Sites migrated |
|-------|------:|---------------:|
| config (env_overrides) | 3 | 14 |
| common (platform_paths) | 1 | 11 |
| server | 14 | 25 |
| cylinder | 4 | 12 |
| CLI | 8 | 30 |
| **Total** | **30** | **117** |

**Metrics:**
- Before: ~265 raw inline string literals, ~188 using constants (43%)
- After: 148 raw, 258 using constants (64%)

### Clippy allow evolution (primalSpring audit: "~17 #[allow(clippy::)] fixes")

- Fixed 2 `#[allow(clippy::collapsible_str_replace)]` at source — `replace([':', '.'], "-")` replaces chained `.replace()`
- Added `reason =` to 3 bare `#[allow(clippy::)]` attributes:
  - `rollback.rs:35` (`too_many_arguments`)
  - `rollback.rs:53` (`too_many_arguments` + `fn_params_excessive_bools`)
  - `identity.rs:203` (`unused_async`)
- All 13 remaining `#[allow(clippy::)]` in production have `reason =` (9 `float_cmp` in `#[cfg(test)]`, 2 `too_many_arguments` with WIP note, 1 `too_many_lines`)

---

## Remaining work (P4 — non-blocking)

148 raw env reads remain in deployment infrastructure and CLI defaults:
- `cli/src/network_config/configurator/core/defaults.rs` (~14 — JAEGER, CA_CERT, PROMETHEUS, sidecar)
- `core/config/src/env_config/loader.rs` (~9 — dynamic key construction, acceptable)
- Various scattered 1-2 per file

These are lower-ROI and mostly in deployment config paths that rarely change.
Future Phase 3: CI grep enforcement to forbid new raw `std::env::var("` outside allowlisted modules.

---

## Dependencies added

- `toadstool-common` added to `toadstool-cylinder` Cargo.toml (env constant access)

## Test results

- All lib tests pass
- 0 clippy warnings (`-D warnings`)
- All workspace crates compile clean
