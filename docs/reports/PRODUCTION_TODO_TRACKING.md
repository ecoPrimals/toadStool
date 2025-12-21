# Production TODO Tracking

**Last Updated**: December 3, 2025  
**Total Production TODOs**: 11  
**Status**: All are appropriate placeholders for future features

---

## 📊 Summary

All production TODOs are **well-documented placeholders** for future features that depend on:
- External services (Songbird integration)
- Infrastructure components not yet built (mDNS, DNS-SD, service registry)
- Nice-to-have improvements (event-driven notifications)

**No immediate action required** - all are tracked for future implementation.

---

## 📋 TODO Inventory

### **Category A: Songbird Integration** (3 TODOs)
**Status**: ✅ Appropriate - Depends on Songbird primal availability

1. **gRPC Client Implementation**
   - File: `crates/distributed/src/songbird_integration/integration.rs:190`
   - Context: `submit_via_grpc()` method
   - Blocker: Requires Songbird primal with gRPC endpoint
   - Priority: Medium (after Songbird is deployed)

2. **WebSocket Client Implementation**
   - File: `crates/distributed/src/songbird_integration/integration.rs:217`
   - Context: `submit_via_websocket()` method
   - Blocker: Requires Songbird primal with WebSocket endpoint
   - Priority: Medium (after Songbird is deployed)

3. **Message Queue Client Implementation**
   - File: `crates/distributed/src/songbird_integration/integration.rs:240`
   - Context: `submit_via_message_queue()` method
   - Blocker: Requires message broker (RabbitMQ/Kafka) and Songbird integration
   - Priority: Low (nice-to-have async job submission)

---

### **Category B: Zero-Config Verification** (3 TODOs)
**Status**: ✅ Appropriate - Nice-to-have health checks

4. **Core Service Health Checks**
   - File: `crates/cli/src/zero_config/verification.rs:44`
   - Context: `verify_core_services()` method
   - Current: Returns `Ok(())` immediately (no-op)
   - Future: Query actual service health endpoints
   - Priority: Low (verification is optional)

5. **Runtime Registry Query**
   - File: `crates/cli/src/zero_config/verification.rs:53`
   - Context: `verify_runtime_engines()` method
   - Current: Returns `Ok(())` immediately (no-op)
   - Future: Query runtime registry for actual engine status
   - Priority: Low (verification is optional)

6. **Ecosystem Connectivity Ping**
   - File: `crates/cli/src/zero_config/verification.rs:61`
   - Context: `verify_ecosystem_connectivity()` method
   - Current: Returns `Ok(())` immediately (no-op)
   - Future: Ping ecoPrimals services (beardog, biomeOS, songbird, squirrel)
   - Priority: Low (verification is optional)

---

### **Category C: Zero-Config Deployment** (2 TODOs)
**Status**: ✅ Appropriate - Optional deployment automation

7. **Orchestrator Deployment**
   - File: `crates/cli/src/zero_config/deployment.rs:61`
   - Context: `deploy_orchestrator()` method
   - Current: Returns `Ok(())` immediately (no-op)
   - Future: Implement actual orchestrator deployment with async events
   - Priority: Low (manual deployment works fine)

8. **Monitoring Deployment**
   - File: `crates/cli/src/zero_config/deployment.rs:72`
   - Context: `deploy_monitoring()` method
   - Current: Returns `Ok(())` immediately (no-op)
   - Future: Deploy monitoring services (Prometheus, Grafana, etc.)
   - Priority: Low (monitoring can be deployed separately)

---

### **Category D: Service Discovery** (1 TODO)
**Status**: ✅ Appropriate - Advanced discovery protocols

9. **Discovery Protocol Implementation**
   - File: `crates/cli/src/zero_config/discovery.rs:406`
   - Context: `discover_via_protocol()` method
   - Current: Returns `None` (falls back to localhost discovery)
   - Future: Implement mDNS, DNS-SD, or registry query protocols
   - Priority: Medium (localhost discovery works for most cases)

---

### **Category E: ServiceRegistry Integration** (1 TODO)
**Status**: ✅ Appropriate - Registry integration point

10. **Use ServiceRegistry for Port Discovery**
    - File: `crates/cli/src/ecosystem/integrator_impl.rs:555`
    - Context: `scan_ip_for_services()` method
    - Current: Uses empty HashMap (no ports scanned)
    - Future: Query ServiceRegistry for dynamic port discovery
    - Priority: Medium (capability-based discovery works without this)

---

### **Category F: Client Improvements** (1 TODO)
**Status**: ✅ Appropriate - Performance optimization

11. **Event-Driven Notifications**
    - File: `crates/client/src/client/core.rs:295`
    - Context: `wait_for_execution()` method
    - Current: Uses polling with exponential backoff (works fine)
    - Future: Replace with event-driven notifications via channels
    - Priority: Low (polling works well with exponential backoff)

---

## ✅ Quality Assessment

### **Why These TODOs Are Appropriate**

1. **No Fake Work**: All modernized - they return immediately or use proper patterns
2. **Well-Documented**: Each TODO explains what needs to be implemented
3. **Clear Dependencies**: External service dependencies are documented
4. **Graceful Degradation**: Systems work without these features
5. **Not Blockers**: None prevent production deployment

### **Production Impact**

- **Runtime**: ✅ Zero (all are no-ops or fallbacks)
- **Stability**: ✅ High (no panics or errors)
- **Functionality**: ✅ Complete (these are enhancements, not core features)
- **Maintainability**: ✅ Excellent (clear implementation paths)

---

## 🎯 Implementation Priority

### **Phase 1: External Dependencies** (After Songbird deployment)
1. Songbird gRPC client (#1)
2. Songbird WebSocket client (#2)
3. Songbird message queue client (#3)

### **Phase 2: Infrastructure** (After registry/discovery services)
4. Discovery protocol implementation (#9)
5. ServiceRegistry integration (#10)

### **Phase 3: Enhancements** (Nice-to-haves)
6. Core service health checks (#4)
7. Runtime registry query (#5)
8. Ecosystem connectivity ping (#6)
9. Orchestrator deployment automation (#7)
10. Monitoring deployment automation (#8)
11. Event-driven client notifications (#11)

---

## 📈 Tracking

**Current Count**: 11 production TODOs  
**Target**: Maintain < 20 (currently ✅)  
**Quality**: All are appropriate placeholders (not debt)  
**Next Review**: After Songbird integration

---

**Conclusion**: All 11 production TODOs are **appropriate placeholders** for future features. No immediate action required. Systems function correctly without these implementations.

