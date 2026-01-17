# Executor Refactoring Integration Plan

## Current Status
- 5/6 modules created (signals, display, resources, lifecycle, process)
- 708 lines extracted from 933-line monolith
- All modules compile successfully

## Integration Strategy

### Phase 3A: Replace method calls (No deletion yet)

**Methods to replace with module calls:**

1. **Lifecycle Module** (lifecycle.rs)
   - `start_biome_internal()` → Use `BiomeLifecycle::start_biome()`
   - `stop_biome_internal()` → Use `BiomeLifecycle::stop_biome()`

2. **Process Module** (process.rs)
   - `start_primal()` → Use `ProcessSpawner::start_primal()`
   - `start_service()` → Use `ProcessSpawner::start_service()`
   - `workload_source_to_spec()` → Use `ProcessSpawner::workload_source_to_spec()`

3. **Signal Module** (signals.rs)
   - `wait_for_interruption()` → Use `SignalManager::wait_for_interrupt()`
   - Signal sending → Use `SignalManager::send_signal()`

4. **Display Module** (display.rs)
   - Table printing → Use `DisplayManager::print_biomes_table()`
   - Log operations → Use `DisplayManager` methods

5. **Resources Module** (resources.rs)
   - `purge_biome_data()` → Use `ResourceManager::purge_biome_data()`
   - `get_actual_pid()` → Use `ResourceManager::get_actual_pid()`
   - Existence checks → Use `ResourceManager` methods

### Phase 3B: Remove extracted method definitions

After integration works and compiles:
- Delete `start_biome_internal()` method body (keep as wrapper if needed)
- Delete `stop_biome_internal()` method body
- Delete `start_primal()` method
- Delete `start_service()` method
- Delete `workload_source_to_spec()` method
- Delete `wait_for_interruption()` method
- Delete other extracted private methods

### Phase 3C: Verify
- Compile successfully
- executor_impl.rs < 500 lines
- All tests pass
- Functionality preserved

## Current executor_impl.rs size: 933 lines
## Target: < 500 lines (46% reduction)
## Expected after integration: ~350-400 lines

## Notes
- Keep public API unchanged
- Private methods can be fully replaced
- Use module structs with `new(self)` pattern
- Maintain error handling and logging
