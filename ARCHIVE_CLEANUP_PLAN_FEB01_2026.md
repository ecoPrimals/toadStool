# 🧹 Archive Cleanup Plan - February 1, 2026

**Status**: READY FOR REVIEW & EXECUTION  
**Impact**: Code cleanup + documentation archiving  
**Grade Improvement**: A+ → A++ (cleanup technical debt)

═══════════════════════════════════════════════════════════════

## 🎯 CLEANUP STRATEGY

### **Goals**:
1. Archive dated completion docs (fossil record in docs/archive/)
2. Remove or document outdated TODOs
3. Review DEPRECATED code for removal/evolution
4. Update HIGH_LEVEL_API_ROADMAP.md status
5. Maintain clean, current root documentation

═══════════════════════════════════════════════════════════════

## 📂 PART 1: ARCHIVE DATED COMPLETION DOCS

### **Root Documents to Archive**:

**February 1, 2026 Session Docs** (Move to `docs/archive/feb01_2026_session/`):
- ✅ Already archived:
  - `ALL_APIS_COMPLETE_JAN30_2026.md`
  - `ISOMORPHIC_IPC_PHASES_1_2_COMPLETE.md`
  - `UNIVERSAL_IPC_IMPLEMENTATION_PLAN_REVISED.md`

**Still in Root** (Archive candidates):
1. `CLIPPY_FIXES_COMPLETE_FEB01_2026.md` - Completion report (archive)
2. `DEEP_DEBT_AUDIT_FEB01_2026.md` - Audit snapshot (archive)
3. `DEEP_DEBT_EXECUTION_PLAN_FEB01_2026.md` - Keep (active plan)
4. `DEEP_DEBT_SESSION_COMPLETE_FEB01_2026.md` - **Keep** (master summary)
5. `ISOMORPHIC_IPC_PHASE_3_COMPLETE.md` - Completion report (archive)
6. `TOADSTOOL_ISOMORPHIC_IPC_SESSION_COMPLETE_FEB01_2026.md` - Completion (archive)
7. `TODO_1_RUNTIME_CAPABILITY_COMPLETE_FEB01_2026.md` - Completion (archive)
8. `TODO_2_NN_TRAINING_METRICS_COMPLETE_FEB01_2026.md` - Completion (archive)
9. `TODO_3_AKIDA_DEVICE_VALUES_COMPLETE_FEB01_2026.md` - Completion (archive)

**Evolution/Planning Docs** (Review/Update):
- `TOADSTOOL_EVOLUTION_PRIORITY_PLAN.md` - Review for updates
- `HIGH_LEVEL_API_ROADMAP.md` - Update status section

**Handoff Docs** (Archive or keep):
- `PETALTONGUE_INTEGRATION_HANDOFF.md` - Archive if complete, else keep

### **Archive Structure**:
```
docs/archive/feb01_2026_session/
├── CLIPPY_FIXES_COMPLETE_FEB01_2026.md
├── DEEP_DEBT_AUDIT_FEB01_2026.md
├── ISOMORPHIC_IPC_PHASE_3_COMPLETE.md
├── TOADSTOOL_ISOMORPHIC_IPC_SESSION_COMPLETE_FEB01_2026.md
├── TODO_1_RUNTIME_CAPABILITY_COMPLETE_FEB01_2026.md
├── TODO_2_NN_TRAINING_METRICS_COMPLETE_FEB01_2026.md
├── TODO_3_AKIDA_DEVICE_VALUES_COMPLETE_FEB01_2026.md
└── ... (already archived docs)
```

### **Keep in Root**:
- `DEEP_DEBT_SESSION_COMPLETE_FEB01_2026.md` - Master session summary
- `DEEP_DEBT_EXECUTION_PLAN_FEB01_2026.md` - Active execution plan
- `STATUS.md` - Current status (always current)
- `ROOT_DOCS_INDEX.md` - Documentation index (always current)

═══════════════════════════════════════════════════════════════

## 📝 PART 2: TODO AUDIT & CLEANUP

### **TODO Categories Found**:

#### **Category 1: Future Features** (Document, keep for now)
```rust
// TODO(future): Implement tarpc message sending
// TODO(future): Implement WebSocket message sending
// TODO(future): Implement mDNS discovery
// TODO(future): Implement registry discovery (Consul, etcd, etc.)
// TODO(future): Implement Kubernetes DNS discovery
// TODO(future): Implement Docker Compose discovery
// TODO(future): Try LocalKeyringProvider
// TODO(future): Try SoftwareHSMProvider
```
**Action**: Keep with `TODO(future)` prefix - clear they're planned features

#### **Category 2: Active Implementation TODOs** (Track in execution plan)
```rust
// TODO: Zero-copy reshape when striding allows (barracuda/src/tensor.rs)
// TODO: Implement remaining layers (barracuda/src/nn.rs)
// TODO: Implement gradients for other activations (barracuda/src/nn.rs)
// TODO: Implement Adam, momentum, etc. (barracuda/src/nn.rs)
// TODO: Implement weight initialization for other layer types
// TODO: Implement parallel batch processing (barracuda/src/genomics.rs)
```
**Action**: These are tracked in `DEEP_DEBT_EXECUTION_PLAN_FEB01_2026.md` - keep

#### **Category 3: Integration TODOs** (Phase 4+)
```rust
// TODO: Integrate with mdns crate when available
// TODO: Implement actual mDNS discovery
// TODO: Implement local registry file reading
// TODO: Add MdnsDiscoveryClient when we wire up config crate
// TODO: Wire up to config crate's MdnsDiscoveryClient
```
**Action**: Document as Phase 4 work, keep in code

#### **Category 4: Mock Replacements** (Production evolution)
```rust
// TODO: Implement actual status query (server/src/pure_jsonrpc.rs)
// TODO: Implement actual workload listing (server/src/pure_jsonrpc.rs)
// TODO: Query actual utilization (server/src/tarpc_server.rs)
// TODO: Check if client is responsive (beardog_impl/client.rs)
// TODO: Return actual discovered capabilities (beardog_integration/client.rs)
```
**Action**: **PRIORITY** - These violate "no mocks" principle!

#### **Category 5: Outdated/False Positives** (Review for removal)
```rust
// ConfigUtils::print_current_config(); // TODO: Re-enable when method exists
// TODO: Component model tests disabled - feature not fully integrated
```
**Action**: Check if these are still relevant or can be removed

#### **Category 6: Accuracy/Completeness TODOs** (Enhancement)
```rust
// TODO: For more accurate disk usage, consider using std::fs
// TODO: Add error tracking (server/src/tarpc_server.rs)
// TODO: Use Unix socket ping instead of HTTP
// TODO: Measure actual latency (primal_discovery_mdns.rs)
```
**Action**: Track as enhancement TODOs, not critical

### **TODO Cleanup Actions**:

**IMMEDIATE** (Deep Debt Violations):
1. Review mock implementations in `server/src/pure_jsonrpc.rs`
2. Review mock implementations in `server/src/tarpc_server.rs`
3. Review mock implementations in `distributed/src/beardog_integration/client.rs`
4. Determine if these are temporary scaffolding or need full implementation

**SHORT-TERM** (Active Work):
- TODOs in `barracuda/` are tracked and being addressed
- Keep these visible until completed

**LONG-TERM** (Future Features):
- `TODO(future)` items are clearly marked
- Move to feature tracking doc if desired

═══════════════════════════════════════════════════════════════

## 🗑️ PART 3: DEPRECATED CODE REVIEW

### **DEPRECATED Code Found**:

#### **Category 1: Legacy Communication** (Remove candidates)
```rust
// crates/core/toadstool/src/ecosystem/communication.rs
/// Send message via HTTP - DEPRECATED
/// Send message via JSON-RPC - DEPRECATED
/// Fallback HTTP sending - DEPRECATED
```
**Status**: Replaced by Isomorphic IPC  
**Action**: **REMOVE** these deprecated methods

#### **Category 2: Legacy Primal Names** (Keep with warnings)
```rust
// crates/core/common/src/interned_strings.rs
/// ⚠️ DEPRECATED: Legacy primal name constants
/// **DEPRECATED**: Use `capabilities::SECURITY` instead
/// **DEPRECATED**: Use `capabilities::COORDINATION` instead
/// **DEPRECATED**: Use `capabilities::STORAGE` instead
```
**Status**: Replaced by capability-based discovery  
**Action**: **KEEP** for backward compatibility, warnings are clear

#### **Category 3: Hardcoded Ports/Endpoints** (Keep with warnings)
```rust
// crates/core/config/src/constants.rs
/// # ⚠️ DEPRECATED: Default network ports for ecosystem primals
/// **DEPRECATED**: Use `RuntimeDiscovery` to find coordinator at runtime
/// **DEPRECATED**: Use `ServiceDiscovery::find_by_capability(Capability::Coordination)`
```
**Status**: Replaced by runtime discovery  
**Action**: **KEEP** as fallbacks, warnings are clear

#### **Category 4: Test Code for Deprecated Features**
```rust
// crates/core/config/tests/config_expansion_tests.rs
// These test the DEPRECATED hardcoded endpoint functions
```
**Status**: Tests for backward compatibility  
**Action**: **KEEP** - ensures deprecated code still works

### **DEPRECATED Cleanup Actions**:

**SAFE TO REMOVE** (Replaced by Isomorphic IPC):
1. `ecosystem/communication.rs` - HTTP/JSON-RPC deprecated methods
   - Check if anything still calls them
   - Remove if no references

**KEEP WITH WARNINGS** (Backward compatibility):
1. Hardcoded ports in `config/src/constants.rs`
2. Legacy primal names in `common/src/interned_strings.rs`
3. Deprecated config methods with clear warnings

**EVOLUTION OPPORTUNITY** (Track for future):
1. Eventually remove all hardcoded endpoints (Phase 5+)
2. Full migration to capability-based discovery

═══════════════════════════════════════════════════════════════

## 📊 PART 4: UPDATE PLANNING DOCS

### **HIGH_LEVEL_API_ROADMAP.md** (Needs update):

**Current Status** (Dated: January 31, 2026):
- Lists 5 API opportunities
- ESN API marked complete
- Planning phase

**Updates Needed**:
1. Update date to February 1, 2026
2. Note barracuda progress (training metrics complete!)
3. Update priority based on deep debt work
4. Cross-reference with `DEEP_DEBT_EXECUTION_PLAN_FEB01_2026.md`

**Suggested Changes**:
```diff
- **Date**: January 31, 2026  
+ **Date**: February 1, 2026  
- **Status**: Planning Phase  
+ **Status**: Active Implementation Phase  
- **Current APIs**: 1 (ESN - Complete ✅)  
+ **Current APIs**: 1 (ESN - Complete ✅)  
+ **Recent Progress**: NN Training Metrics complete (accuracy, epoch, batch tracking)
```

### **TOADSTOOL_EVOLUTION_PRIORITY_PLAN.md** (Review):

**Check if**:
- Priorities still current
- Isomorphic IPC completion changes priorities
- Deep debt work changes priorities

═══════════════════════════════════════════════════════════════

## 🎯 EXECUTION PLAN

### **Phase 1: Archive Docs** (15 minutes)
```bash
# Move completion reports to archive
mkdir -p docs/archive/feb01_2026_session/
git mv CLIPPY_FIXES_COMPLETE_FEB01_2026.md docs/archive/feb01_2026_session/
git mv DEEP_DEBT_AUDIT_FEB01_2026.md docs/archive/feb01_2026_session/
git mv ISOMORPHIC_IPC_PHASE_3_COMPLETE.md docs/archive/feb01_2026_session/
git mv TOADSTOOL_ISOMORPHIC_IPC_SESSION_COMPLETE_FEB01_2026.md docs/archive/feb01_2026_session/
git mv TODO_1_RUNTIME_CAPABILITY_COMPLETE_FEB01_2026.md docs/archive/feb01_2026_session/
git mv TODO_2_NN_TRAINING_METRICS_COMPLETE_FEB01_2026.md docs/archive/feb01_2026_session/
git mv TODO_3_AKIDA_DEVICE_VALUES_COMPLETE_FEB01_2026.md docs/archive/feb01_2026_session/

# Review PETALTONGUE_INTEGRATION_HANDOFF.md
# - If complete, archive it
# - If still active, keep in root
```

### **Phase 2: Review Mock TODOs** (30 minutes)
1. Check `server/src/pure_jsonrpc.rs` for mock implementations
2. Check `server/src/tarpc_server.rs` for mock implementations
3. Check `distributed/src/beardog_integration/client.rs` for mocks
4. Determine action: implement, document, or mark as scaffolding

### **Phase 3: Remove Deprecated Code** (20 minutes)
1. Check references to deprecated communication methods
2. Remove if no active usage
3. Update tests if needed

### **Phase 4: Update Planning Docs** (15 minutes)
1. Update `HIGH_LEVEL_API_ROADMAP.md` status
2. Review `TOADSTOOL_EVOLUTION_PRIORITY_PLAN.md`
3. Ensure consistency with `STATUS.md`

### **Phase 5: Final Review** (10 minutes)
1. Check root documentation count
2. Verify archive structure
3. Update `ROOT_DOCS_INDEX.md` if needed
4. Update `STATUS.md` if needed

### **Total Time**: ~90 minutes

═══════════════════════════════════════════════════════════════

## 📈 EXPECTED OUTCOMES

### **Before Cleanup**:
- 33 root docs (some dated)
- ~50 TODOs (mix of active/future/outdated)
- DEPRECATED code (some removable)
- Planning docs slightly outdated

### **After Cleanup**:
- 26-28 root docs (only current docs)
- ~40 TODOs (clearly categorized)
- Deprecated code pruned or documented
- Planning docs current
- Clear fossil record in archives

### **Deep Debt Impact**:
- ✅ Code cleanliness improved
- ✅ Documentation organization improved
- ✅ TODO management clearer
- ✅ Grade: A+ → A++ readiness

═══════════════════════════════════════════════════════════════

## 🎊 SUMMARY

**Found**:
- 7 completion docs to archive
- ~50 TODOs (various categories)
- ~25 DEPRECATED items
- 2 planning docs to update

**Actions**:
1. Archive 7 completion docs
2. Categorize and document TODOs
3. Review mock implementations (deep debt!)
4. Prune deprecated communication code
5. Update 2 planning docs

**Timeline**: 90 minutes  
**Impact**: Clean codebase + organized archives  
**Grade**: Maintains A+ → enables A++

═══════════════════════════════════════════════════════════════

**Status**: ✅ READY FOR EXECUTION  
**Next**: Phase 1 - Archive completion docs

🦀🧹 **Clean Code = Happy Code!** 🧹🦀
