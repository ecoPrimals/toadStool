# Test Coverage Report - November 14, 2025

## 📊 Overall Coverage Summary

**Current Coverage: 50.77% (region) / 51.95% (line)**

- Total Regions: 52,374
- Covered Regions: 26,589
- Total Lines: 40,291
- Covered Lines: 20,932

**Test Status: ✅ ALL TESTS PASSING**
- Build: ✅ Clean
- Linting: ✅ Clean  
- Formatting: ✅ Clean
- Tests: ✅ 100% Pass Rate

## 🎯 Critical Zero-Coverage Files

### Extremely Low Coverage (<5%)

1. **cli/src/executor/executor_impl.rs** - 1.81% (938 lines)
   - Core CLI execution engine
   - Needs integration tests with real biome manifests

2. **security/policies/src/evaluator.rs** - 2.58% (155 lines)
   - Policy condition evaluation
   - Requires complex context setup

3. **security/policies/src/executor.rs** - 2.31% (130 lines)
   - Policy action execution
   - Needs policy evaluation contexts

4. **distributed/substrate_detection.rs** - 4.78% (439 lines)
   - Runtime substrate detection
   - Requires actual runtime environments

5. **security/policies/src/manager.rs** - 6.63% (181 lines)
   - File-based policy management
   - Needs filesystem integration tests

### Complete Zero Coverage (0%)

#### CLI Network Configurator (0%)
- `cli/src/network_config/configurator/core.rs` (316 lines)
- `cli/src/network_config/configurator/discovery.rs` (23 lines)
- `cli/src/network_config/configurator/reliability.rs` (43 lines)
- `cli/src/network_config/configurator/security.rs` (26 lines)
- `cli/src/network_config/configurator/service_mesh.rs` (32 lines)
- `cli/src/network_config/configurator/traffic.rs` (44 lines)

#### Songbird Integration (0%)
- `distributed/src/songbird_integration/broadcasting.rs` (35 lines)
- `distributed/src/songbird_integration/discovery.rs` (467 lines) ⚠️ LARGE
- `distributed/src/songbird_integration/distribution.rs` (206 lines)
- `distributed/src/songbird_integration/integration.rs` (291 lines)
- `distributed/src/songbird_integration/load_balancing.rs` (26 lines)
- `distributed/src/songbird_integration/types.rs` (61 lines)

#### Other Zero Coverage
- `cli/src/main.rs` (872 lines) - Main binary entry point
- `cli/src/lib.rs` (62 lines)
- `cli/src/monitoring.rs` (522 lines)
- `distributed/src/cloud/orchestrator.rs` (399 lines)
- `distributed/src/crypto_lock.rs` (381 lines)
- `distributed/src/universal/substrate.rs` (411 lines)
- `distributed/src/primal_capabilities/adapters.rs` (47 lines)
- `integration/nestgate/src/client.rs` (367 lines)

## 🟢 High Coverage Files (>90%)

### Excellent Coverage (95-100%)
- `api/src/middleware.rs` - 93.96%
- `server/src/handlers.rs` - 94.73%
- `security/sandbox/src/manager.rs` - 92.98%
- `integration/nestgate/src/lib.rs` - 94.12%
- `runtime/gpu/src/lib.rs` - 99.13%
- `testing/src/integration/integration_impl.rs` - 88.71%
- `testing/src/properties.rs` - 96.34%
- `testing/src/mocks/resource_monitors.rs` - 96.41%

## 📈 Coverage by Module

### Core Modules
- **toadstool-config**: ~60-70% (Good)
- **toadstool-common**: ~70-80% (Good)
- **toadstool (core)**: ~50-70% (Moderate)

### Runtime Engines
- **Native Runtime**: 81.07% (Excellent)
- **WASM Runtime**: 48.62% (Needs improvement)
- **Python Runtime**: 61.40% (Good)
- **GPU Runtime**: 37.28% engine / 99.13% lib (Mixed)
- **Container Runtime**: 51.94% (Moderate)

### Distributed & Integration
- **Distributed Coordinator**: 48.28% (Needs improvement)
- **Songbird Integration**: 0% (Critical gap)
- **Network Components**: 98-100% (Excellent)
- **Protocols Client**: 70.78% (Good)

### Security
- **Sandbox Manager**: 92.98% (Excellent)
- **Security Policies**: 2-6% (Critical gap)

### Server & API
- **Server Handlers**: 94.73% (Excellent)
- **Server Background**: 78.60% (Good)
- **Server WebSocket**: 52.05% (Moderate)
- **API Middleware**: 93.96% (Excellent)
- **API Handlers**: 67.85% (Good)

### Testing Framework
- **Testing Integration**: 88.71% (Excellent)
- **Testing Properties**: 96.34% (Excellent)
- **Testing Mocks**: 79-96% (Excellent)
- **Testing Performance**: 85.81% (Excellent)

## 🎯 Recommended Next Steps

### Quick Wins (Can reach 90% easily)
1. **API Middleware** (93.96% → 100%) - Just 6.04% to go
2. **Server Handlers** (94.73% → 100%) - Just 5.27% to go
3. **Integration Nestgate Lib** (94.12% → 100%) - Just 5.88% to go

### High-Impact Targets
1. **Security Policies** (2-6% → 50%+)
   - Add unit tests for evaluator, executor, manager
   - Mock file system for manager tests
   
2. **CLI Executor** (1.81% → 30%+)
   - Add integration tests with test biome manifests
   - Mock distributed coordinator

3. **Songbird Integration** (0% → 40%+)
   - Add unit tests for types and connections
   - Mock Songbird service for integration tests

### Long-Term Goals
1. **WASM Runtime** (48.62% → 70%+)
2. **GPU Runtime Engine** (37.28% → 60%+)
3. **CLI Network Configurator** (0% → 50%+)
4. **Management Modules** (20-35% → 60%+)

## 🚧 Testing Challenges

### Why Some Files Have Low Coverage
1. **Integration Complexity**: Many modules require full system setup
2. **External Dependencies**: Songbird, NestGate, real runtimes
3. **File System Operations**: Policy manager, CLI operations
4. **Async/Concurrency**: Difficult to test thoroughly
5. **Platform-Specific**: Linux-only features, GPU requirements

### What's Been Done
- ✅ Fixed all test failures
- ✅ Achieved 100% test pass rate
- ✅ Fixed environment variable pollution in config tests
- ✅ Fixed doctest compilation errors
- ✅ Comprehensive test coverage in testing framework itself
- ✅ Excellent coverage in server handlers and API middleware

### What Still Needs Work
- ❌ Security policies need real integration tests
- ❌ CLI executor needs biome manifest tests
- ❌ Songbird integration has no tests at all
- ❌ Network configurator has no tests
- ❌ Many distributed components need better coverage

## 📝 Notes

- **Baseline Improved**: From 50.73% to 50.77% (+0.04%)
- **Test Infrastructure**: Excellent (88-96% coverage in testing crate)
- **Production Code**: Mixed (2% to 99% across different modules)
- **Quick Wins Available**: Several files at 90%+ can reach 100% easily

## 🎉 Achievements

1. **Zero Test Failures**: All tests now pass consistently
2. **Clean Build**: No linting or formatting issues
3. **Strong Foundation**: Testing framework has excellent coverage
4. **High-Quality Areas**: Server, API, security sandbox, native runtime all >75%

---

**Target**: 90% overall coverage  
**Current**: 51.95% line coverage  
**Gap**: 38.05 percentage points  
**Estimated Work**: ~200-300 additional test cases needed

