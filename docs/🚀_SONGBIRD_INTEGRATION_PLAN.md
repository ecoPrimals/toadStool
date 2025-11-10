# 🚀 ToadStool + Songbird Integration Plan

**Date**: November 8, 2025  
**Source**: Songbird orchestrator instructions  
**Status**: ✅ In Progress

---

## 📋 Integration Steps (From Songbird)

### **✅ Step 1: Build ToadStool**
```bash
cd /home/eastgate/Development/ecoPrimals/toadstool
cargo build --workspace --release
```
**Status**: In progress

### **Step 2: Start Integration Test**
```bash
cd /home/eastgate/Development/ecoPrimals/songbird
./test_toadstool_integration.sh
```
**Status**: Pending (checking if script exists)

### **Step 3: Start ToadStool Services**

#### **Tower A (192.168.1.144 - This Tower)**:
```bash
cd /home/eastgate/Development/ecoPrimals/toadstool
TOADSTOOL_PORT=9000 \
TOADSTOOL_HOST=192.168.1.144 \
TOADSTOOL_SONGBIRD_ENDPOINT=http://192.168.1.144:8080 \
./target/release/toadstool-server
```

#### **Tower B (Later - After Transfer)**:
```bash
cd /home/eastgate/Development/ecoPrimals/toadstool
TOADSTOOL_PORT=9000 \
TOADSTOOL_HOST=<TOWER_B_IP> \
TOADSTOOL_SONGBIRD_ENDPOINT=http://192.168.1.144:8080 \
./target/release/toadstool-server
```

### **Step 4: Load Testing**

**Test Scenarios**:
1. **CPU Task Routing**:
   - Submit 100 CPU tasks
   - Should route to Tower B (128 cores)
   
2. **GPU Task Routing**:
   - Submit 10 GPU tasks  
   - Should route to Tower A (RTX 2070 SUPER)

3. **Performance Targets**:
   - Sub-10ms routing latency ⚡
   - Smart capability-based routing

---

## 🔧 Configuration

### **Tower A (This Tower)**:
```bash
Host: 192.168.1.144
Port: 9000 (ToadStool)
Songbird: http://192.168.1.144:8080
Capabilities: RTX 2070 SUPER GPU
```

### **Tower B (Target)**:
```bash
Host: <TO BE DETERMINED>
Port: 9000 (ToadStool)
Songbird: http://192.168.1.144:8080 (Tower A coordinator)
Capabilities: 128 CPU cores
```

---

## 🎯 Expected Behavior

### **Intelligent Routing**:
- **CPU-intensive tasks** → Tower B (more cores)
- **GPU tasks** → Tower A (has GPU)
- **Mixed workloads** → Balanced across both

### **Performance**:
- Routing decision: <10ms
- Job submission: <100ms
- Subtask distribution: Automatic
- Results aggregation: Seamless

---

## 📝 Current Status

### **Tower A** ✅:
- [x] ToadStool showcase works locally
- [x] Songbird running (192.168.1.144:8080)
- [x] Service ID: tower-a-orchestrator
- [ ] ToadStool server started
- [ ] Registered with Songbird

### **Tower B** ⏳:
- [ ] ToadStool transferred
- [ ] ToadStool server started
- [ ] Registered with Songbird
- [ ] Network connectivity verified

---

## 🔍 Next Actions

1. ✅ Finish building ToadStool workspace
2. ⏳ Check for integration test script
3. ⏳ Start ToadStool server on Tower A
4. ⏳ Verify registration with Songbird
5. ⏳ Deploy to Tower B
6. ⏳ Start ToadStool server on Tower B
7. ⏳ Run load tests
8. ⏳ Verify intelligent routing

---

## 🐦 Songbird Federation

### **Current State**:
- ✅ Songbird orchestrator running
- ✅ Health: OK
- ⚠️  No nodes registered yet
- ⚠️  Waiting for ToadStool servers to connect

### **After ToadStool Servers Start**:
- ToadStool servers register with Songbird
- Songbird discovers capabilities
- Federation topology established
- Ready for distributed tasks

---

## 💡 Key Insights

### **Architecture**:
```
┌─────────────┐
│  Songbird   │  ← Orchestrator (Tower A: 192.168.1.144:8080)
│ Coordinator │
└──────┬──────┘
       │
       ├──────────┬──────────┐
       │          │          │
   ┌───▼───┐  ┌──▼────┐  ┌──▼────┐
   │Tower A│  │Tower B│  │Future │
   │GPU    │  │128CPU │  │Towers │
   └───────┘  └───────┘  └───────┘
```

### **Registration Process**:
1. ToadStool server starts
2. Connects to Songbird endpoint
3. Reports capabilities (CPU, GPU, memory, etc.)
4. Receives job routing instructions
5. Executes assigned workloads

---

## 🚀 Commands Summary

### **Start ToadStool Server (Tower A)**:
```bash
cd /home/eastgate/Development/ecoPrimals/toadstool
TOADSTOOL_PORT=9000 \
TOADSTOOL_HOST=192.168.1.144 \
TOADSTOOL_SONGBIRD_ENDPOINT=http://192.168.1.144:8080 \
./target/release/toadstool-server
```

### **Verify Registration**:
```bash
curl http://localhost:8080/api/v1/nodes
# Should show Tower A with capabilities
```

### **Submit Test Job**:
```bash
cd showcase/
./showcase.sh  # Select option 2
# Should distribute subtasks intelligently
```

---

## 🎉 Success Criteria

- [ ] Both towers register with Songbird
- [ ] CPU tasks route to Tower B
- [ ] GPU tasks route to Tower A
- [ ] Routing latency < 10ms
- [ ] 100 tasks complete successfully
- [ ] Intelligent load balancing works

---

**🍄 ToadStool + Songbird: Intelligent Distributed Computing!** 🐦

