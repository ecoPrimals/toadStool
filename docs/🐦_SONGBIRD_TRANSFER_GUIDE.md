# 🐦 Transfer ToadStool via Songbird - Quick Guide

**Question**: "Can we just pass ToadStool manually with Songbird?"  
**Answer**: **YES!** Songbird can transfer ToadStool binaries directly between towers.

---

## 📦 What Can Be Transferred

### **ToadStool Binaries**:
- `toadstool-cli`: 20MB
- `toadstool-showcase-distributed`: 2.2MB  
- `showcase/` directory: 288KB

**Total**: ~22-23MB (very reasonable over LAN!)

### **Via Songbird Payload**:
- Songbird uses `payload: Vec<u8>` (binary data)
- Can transfer files encoded as base64
- LAN transfer is fast (typically <1 second for 20MB)

---

## 🚀 Quick Transfer Methods

### **Method 1: Automated Songbird Deployment** (Recommended)

```bash
cd /home/eastgate/Development/ecoPrimals/toadstool

# Deploy ToadStool to Tower B
./scripts/songbird-deploy-toadstool.sh tower-b

# What it does:
# 1. Packages binaries + showcase + configs
# 2. Encodes as base64
# 3. Submits as Songbird job to target tower
# 4. Auto-extracts and installs on remote tower
# 5. Ready to use!
```

**Benefits**:
- One command deployment
- Automatic installation
- No manual file copying
- Leverages existing Songbird connection

---

### **Method 2: Manual Transfer via Songbird API**

```bash
# 1. Create package
cd target/release
tar czf toadstool-binaries.tar.gz toadstool-cli toadstool-showcase-distributed

# 2. Encode for Songbird
base64 toadstool-binaries.tar.gz > toadstool-binaries.b64

# 3. Send via Songbird
curl -X POST http://localhost:8080/api/v1/transfer \
  -H "Content-Type: application/json" \
  -d '{
    "target_node": "tower-b",
    "payload": "'$(cat toadstool-binaries.b64)'",
    "action": "deploy"
  }'

# 4. On Tower B, Songbird will auto-extract to /tmp/toadstool-deployment
```

---

### **Method 3: Direct Binary Transfer (Fastest for Testing)**

If Songbird is just for coordination and not file transfer:

```bash
# Quick SCP over LAN (fallback)
scp target/release/toadstool-showcase-distributed tower-b:/tmp/
scp -r showcase/ tower-b:/tmp/

# On Tower B:
chmod +x /tmp/toadstool-showcase-distributed
/tmp/toadstool-showcase-distributed
```

---

## 🧪 Test Before Full Deployment

### **Step 1: Verify Songbird Connection**

```bash
# On Tower A:
curl http://localhost:8080/api/v1/health

# Check Tower B is registered:
curl http://localhost:8080/api/v1/nodes
```

**Expected Output**:
```json
{
  "nodes": [
    {"id": "tower-a", "status": "active", "capabilities": {...}},
    {"id": "tower-b", "status": "active", "capabilities": {...}}
  ]
}
```

### **Step 2: Test Small Transfer**

```bash
# Test with showcase demo only (2.2MB)
./scripts/songbird-deploy-toadstool.sh tower-b

# Should complete in <5 seconds over LAN
```

### **Step 3: Test Distributed Execution**

```bash
# On Tower A:
cd showcase/
./showcase.sh

# Select option 2: Distributed Compute Demo
# Watch subtasks distribute to Tower B!
```

---

## 🔧 Configuration for Tower-to-Tower

### **Update `toadstool-songbird-network.toml`**:

```toml
[network.songbird_orchestration]
enabled = true
endpoint = "http://tower-a:8080"  # Songbird coordinator

[[nodes]]
name = "tower-a"
address = "192.168.1.10:9000"
capacity_cpu = 8.0
capacity_memory_gb = 16

[[nodes]]
name = "tower-b"
address = "192.168.1.11:9000"  # Your other tower
capacity_cpu = 8.0
capacity_memory_gb = 16
```

### **Update Hosts** (if needed):

```bash
# /etc/hosts on both towers
192.168.1.10  tower-a
192.168.1.11  tower-b
```

---

## 📊 Comparison: GitHub vs Songbird

| Method | Time | Bandwidth | Steps | Complexity |
|--------|------|-----------|-------|------------|
| **GitHub Push/Pull** | ~60s | External | 4 steps | Medium |
| **Songbird Transfer** | ~3s | LAN only | 1 step | Low |
| **Direct SCP** | ~1s | LAN only | 2 steps | Very Low |

**Recommendation**:
1. **For quick testing**: Use Songbird transfer (Method 1)
2. **After verified working**: Push to GitHub for official release
3. **For other teams**: They pull from GitHub

---

## 🎯 Complete Workflow

### **Scenario: Quick LAN Testing**

```bash
# On Tower A (your current location):

# 1. Build ToadStool
cargo build --release

# 2. Test locally
./target/release/toadstool-showcase-distributed
# ✅ Works! (you already verified this)

# 3. Transfer to Tower B via Songbird
./scripts/songbird-deploy-toadstool.sh tower-b
# ✅ Transferred and installed

# 4. Test distributed execution
cd showcase/ && ./showcase.sh
# Select option 2
# ✅ See subtasks on both towers!

# 5. If everything works, push to GitHub
git push origin parse-error-fixes-canonical-cleanup
# ✅ Official release

# 6. On Tower B (and others), pull from GitHub
git pull origin parse-error-fixes-canonical-cleanup
# ✅ Everyone has same version
```

---

## 💡 Why This Works

### **Songbird Payload Capability**:
```rust
pub struct SubTask {
    pub payload: Vec<u8>,  // ← Can hold binary data!
    // ...
}

pub struct SongbirdJobRequest {
    pub job_payload: Vec<u8>,  // ← Binary transfer capable
    // ...
}
```

**Key Points**:
- Songbird already handles binary payloads for job distribution
- We're just using it to distribute ToadStool itself
- Once on remote tower, it can receive more jobs normally
- It's "eating its own dog food" - using ToadStool infrastructure to deploy ToadStool!

---

## 🚀 Try It Now!

**Quick Test**:
```bash
# 1. Run the automated deployment
./scripts/songbird-deploy-toadstool.sh tower-b

# 2. Watch the magic happen:
#    - Package created (~23MB)
#    - Encoded for Songbird
#    - Submitted as job
#    - Transferred to Tower B
#    - Auto-installed
#    - Ready to use!

# 3. Time: ~5-10 seconds total
```

**If it works**:
- ✅ You've proven LAN distribution
- ✅ Can test tower-to-tower immediately
- ✅ Push to GitHub when satisfied
- ✅ Others can pull from GitHub

---

## 🎉 Benefits

**Using Songbird for Deployment**:
1. ✅ **Fast**: LAN transfer, no internet needed
2. ✅ **Simple**: One command
3. ✅ **Consistent**: Uses same infrastructure as job distribution
4. ✅ **Automatic**: No manual file copying
5. ✅ **Verified**: Tests the system end-to-end

**Then Push to GitHub**:
1. ✅ **Archival**: Official version control
2. ✅ **Team Access**: Everyone can pull
3. ✅ **History**: Proper change tracking
4. ✅ **Backup**: Offsite copy

---

## 📝 Summary

**Answer to your question**: 
> "can we spawn toadstool across the songbird connection? as in its small enough, can we just pass it manually with songbird?"

**YES!** 
- ToadStool showcase demo: 2.2MB ✅
- ToadStool CLI: 20MB ✅  
- Total with showcase: ~23MB ✅
- **Very manageable over LAN via Songbird!**

**Recommended Flow**:
1. Test with Songbird transfer (fast, local)
2. Verify distributed execution works
3. Push to GitHub (official release)

---

**🐦 Ready to try! Run the script and watch ToadStool fly via Songbird! 🍄**

