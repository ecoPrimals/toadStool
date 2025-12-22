# Wiring Showcases to Real Services

**Current Status**: Showcases use **mock/simulated** responses  
**Goal**: Connect to **live primal instances**

---

## Why Mocks?

The showcases are designed to work **standalone** for:
- ✅ Development without dependencies
- ✅ Testing integration patterns
- ✅ Demonstrating architecture
- ✅ Reference implementation

But they're **ready** to connect to real services!

---

## How to Wire to Real Services

### Step 1: Check What's Running

```bash
# Songbird (Strandgate)
curl -sk https://192.168.1.134:8081/health

# Squirrel (if running)
curl -s http://localhost:8085/health

# BearDog (if running)
curl -s http://localhost:8083/health

# NestGate (if running)
curl -s http://localhost:8084/health
```

### Step 2: Set Environment Variables

For each running service:

```bash
# Songbird
export SONGBIRD_ENDPOINT=https://192.168.1.134:8081

# Squirrel (when available)
export SQUIRREL_ENDPOINT=http://squirrel-host:8085

# BearDog (when available)
export BEARDOG_ENDPOINT=http://beardog-host:8083

# NestGate (when available)
export NESTGATE_ENDPOINT=http://nestgate-host:8084
```

### Step 3: Run Showcases

They'll automatically detect and use real services!

```bash
cd showcase/inter-primal/02-songbird-distributed-training
export SONGBIRD_ENDPOINT=https://192.168.1.134:8081
./02-run-distributed-training.sh
```

---

## What Happens When Services Are Available?

### Mock Behavior (Current)

```rust
// Squirrel client health check
match self.http_client.get(&url).send().await {
    Err(_) => {
        warn!("Squirrel unavailable - using fallback");
        Ok(self.fallback_recommendation(workload)) // ← Returns mock data
    }
}
```

### Real Behavior (With Services)

```rust
// Squirrel client health check
match self.http_client.get(&url).send().await {
    Ok(response) => {
        let recommendation = response.json().await?; // ← Real API response!
        Ok(recommendation)
    }
}
```

**Same code, different behavior based on availability!**

---

## Federation Integration

### ToadStool → Songbird Registration

To make ToadStool discoverable by Songbird:

```bash
# On Eastgate (this tower)
cd ~/Development/ecoPrimals/toadstool
cargo run --release -- \
  --register-with-songbird \
  --songbird-url https://192.168.1.134:8081 \
  --tower-id eastgate \
  --gpu-info "RTX 2070, 8GB"
```

### Northgate Federation

Northgate isn't in the federation yet because:
- Need to start ToadStool on Northgate
- Register with Songbird
- Then it'll be discoverable

```bash
# On Northgate (when available)
cd ~/Development/ecoPrimals/toadstool
cargo run --release -- \
  --register-with-songbird \
  --songbird-url https://192.168.1.134:8081 \
  --tower-id northgate \
  --gpu-info "RTX 5090, 32GB"
```

---

## Quick Test: Songbird Discovery

Let's test with the ONE service we know is running:

```bash
cd showcase/inter-primal/02-songbird-distributed-training

# Point to real Songbird
export SONGBIRD_ENDPOINT=https://192.168.1.134:8081

# Discover towers (will show real federation if ToadStool instances registered)
./01-reconnect-federation.sh
```

**Expected**:
- If ToadStool towers are registered: Shows real towers
- If not registered: Shows "no towers found" (still works, falls back to local)

---

## Evolution Path

### Phase 1: ✅ Showcases with Mocks (CURRENT)
- Demonstrates architecture
- Works standalone
- Fast development

### Phase 2: 🚧 Partial Real Integration (NEXT)
- Connect to Songbird (available)
- Keep other services mocked
- Hybrid approach

### Phase 3: 🔮 Full Real Integration
- All 5 primals running
- All services live
- Production-ready mesh

---

## Benefits of Mock-First Approach

1. **Development Speed**: Build without waiting for all services
2. **Testing**: Predictable, repeatable results
3. **Documentation**: Clear examples of expected behavior
4. **Resilience**: Showcases work even if services are down

**But the code is ready for real integration - just add services!**

---

## Action Items for Real Integration

### Immediate (Today)
- [ ] Start ToadStool server on Eastgate
- [ ] Register Eastgate with Songbird
- [ ] Test real discovery via Songbird API

### Short-term (This Week)
- [ ] Deploy ToadStool to Northgate
- [ ] Register Northgate with Songbird
- [ ] Run distributed training across real towers

### Medium-term (This Month)
- [ ] Start remaining primal services (Squirrel, BearDog, NestGate)
- [ ] Update showcase env vars to point to real services
- [ ] Demo full ecosystem with real services

---

## Testing Real vs Mock

Add this to your demos to show what's real:

```rust
if response.status().is_success() {
    println!("✅ Using REAL service response");
    // ... handle real data
} else {
    println!("⚠️  Using MOCK fallback (service unavailable)");
    // ... use mock data
}
```

---

**Status**: Showcases are architecturally complete and ready for real services!  
**Next**: Wire to live Songbird, then expand to other primals as they come online.

🦀 **The patterns are proven - now let's make them real!** 🚀

