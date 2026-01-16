# ToadStool v4.10.0 - Deployment Quick Start

**Version**: v4.10.0 - Pure Rust + UniBin  
**Date**: January 16, 2026  
**Status**: ✅ Production Ready (A++ Grade)

---

## 🚀 **QUICK START (3 STEPS)**

### **1. Build the Binary**

```bash
cd /path/to/toadstool
cargo build --release --bin toadstool
```

**Result**: One binary at `target/release/toadstool` (optimized, ~80MB)

---

### **2. Deploy the Binary**

```bash
# Copy to system location
sudo cp target/release/toadstool /usr/local/bin/

# Or add to PATH
export PATH="$PWD/target/release:$PATH"

# Verify
toadstool --version
# Output: toadstool 0.1.0
```

**Result**: `toadstool` command available system-wide

---

### **3. Run a Biome**

```bash
# Create a biome.yaml
toadstool init my-biome

# Start in foreground
toadstool run biome.yaml

# Or start in background
toadstool up biome.yaml

# Check status
toadstool ps
```

**Result**: Biome running!

---

## 📊 **UNIBIN ARCHITECTURE**

### **One Binary, Multiple Modes**

The `toadstool` binary handles all functionality:

```bash
# CLI Mode (default)
toadstool run biome.yaml          # Start biome
toadstool up biome.yaml            # Background mode
toadstool ps                       # List biomes
toadstool logs my-biome            # View logs

# Server/Daemon Mode
toadstool daemon                   # Start as daemon

# Direct Workload Execution
toadstool execute workload.toml    # No biome needed
```

---

### **Backward Compatibility**

Legacy commands still work via symlinks:

```bash
# Create symlinks
ln -s /usr/local/bin/toadstool /usr/local/bin/toadstool-cli
ln -s /usr/local/bin/toadstool /usr/local/bin/toadstool-server

# Use old names
toadstool-cli run biome.yaml       # Works!
toadstool-server                   # Auto-routes to daemon
```

---

## 🔧 **DEPLOYMENT SCENARIOS**

### **Scenario 1: Development Machine**

```bash
# Clone repo
git clone https://github.com/ecoPrimals/toadStool.git
cd toadStool

# Build debug (fast compile, with symbols)
cargo build --bin toadstool

# Run directly
./target/debug/toadstool run examples/simple-biome.yaml
```

**Use Case**: Local development, testing

---

### **Scenario 2: Production Server (x86_64)**

```bash
# Build optimized release
cargo build --release --bin toadstool

# Deploy
sudo cp target/release/toadstool /usr/local/bin/
sudo chmod +x /usr/local/bin/toadstool

# Configure as systemd service
cat << 'SYSTEMD' | sudo tee /etc/systemd/system/toadstool.service
[Unit]
Description=ToadStool Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/toadstool daemon
Restart=on-failure
User=toadstool
Group=toadstool

[Install]
WantedBy=multi-user.target
SYSTEMD

# Enable and start
sudo systemctl enable toadstool
sudo systemctl start toadstool
```

**Use Case**: Production deployment, always-on daemon

---

### **Scenario 3: ARM Device (Raspberry Pi, etc.)**

```bash
# Install ARM cross-compiler (on build machine)
sudo apt install gcc-aarch64-linux-gnu

# Cross-compile
cargo build --release --target aarch64-unknown-linux-gnu --bin toadstool

# Copy to ARM device
scp target/aarch64-unknown-linux-gnu/release/toadstool pi@raspberrypi:/tmp/

# On ARM device
ssh pi@raspberrypi
sudo mv /tmp/toadstool /usr/local/bin/
sudo chmod +x /usr/local/bin/toadstool

# Verify
toadstool --version
# Output: toadstool 0.1.0
```

**Use Case**: Edge devices, IoT, embedded systems

---

### **Scenario 4: Docker Container**

```dockerfile
# Dockerfile
FROM rust:1.75 as builder
WORKDIR /build
COPY . .
RUN cargo build --release --bin toadstool

FROM debian:bookworm-slim
COPY --from=builder /build/target/release/toadstool /usr/local/bin/
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["toadstool"]
CMD ["daemon"]
```

```bash
# Build image
docker build -t toadstool:v4.10.0 .

# Run daemon
docker run -d --name toadstool toadstool:v4.10.0

# Run CLI
docker run --rm toadstool:v4.10.0 --version
```

**Use Case**: Containerized deployments, Kubernetes

---

## 🌍 **BIOMEOS INTEGRATION**

### **Harvesting ToadStool**

```bash
# biomeOS discovers ToadStool via capabilities
toadstool capabilities

# Output shows:
# - Pure Rust: Yes (zero TLS)
# - UniBin: Yes
# - Available runtimes: Native, Python, WASM, GPU (optional)
# - Unix socket: /var/run/toadstool.sock
```

---

### **Primal Communication**

ToadStool uses **Unix sockets** for primal-to-primal communication:

```bash
# ToadStool socket
/var/run/toadstool.sock

# Discovers other primals via capability registry
toadstool ecosystem discover

# Connects via JSON-RPC 2.0 over Unix sockets
# NO HTTP between primals! (Pure Rust, per biomeOS guidance)
```

---

### **External HTTP via Songbird**

Per **Concentrated Gap Architecture**:

```bash
# ToadStool has ZERO HTTP client
# External requests route through Songbird

# Example flow:
ToadStool → Unix Socket → Songbird → HTTPS → External API
```

**Result**: Clean separation, Songbird = only TLS primal

---

## 🔒 **SECURITY**

### **Production Recommendations**

```bash
# 1. Create dedicated user
sudo useradd -r -s /bin/false toadstool

# 2. Set binary ownership
sudo chown root:root /usr/local/bin/toadstool
sudo chmod 755 /usr/local/bin/toadstool

# 3. Configure data directory
sudo mkdir -p /var/lib/toadstool
sudo chown toadstool:toadstool /var/lib/toadstool
sudo chmod 750 /var/lib/toadstool

# 4. Configure socket directory
sudo mkdir -p /var/run/toadstool
sudo chown toadstool:toadstool /var/run/toadstool
sudo chmod 750 /var/run/toadstool

# 5. Run as non-root
toadstool daemon --user toadstool
```

---

### **BearDog Integration**

ToadStool integrates with BearDog for cryptographic security:

```bash
# Discovers BearDog via capability registry
# Connects via Unix socket
# Zero trust, cryptographic verification
```

---

## 📊 **MONITORING**

### **Health Checks**

```bash
# Check daemon status
toadstool ps

# View logs
toadstool logs --follow

# System capabilities
toadstool capabilities
```

---

### **Metrics (Optional)**

ToadStool exports metrics via Prometheus (when enabled):

```bash
# Metrics endpoint (when daemon mode)
curl http://localhost:9090/metrics
```

---

## 🎯 **VERIFICATION**

### **Post-Deployment Checks**

```bash
# 1. Version
toadstool --version
# Expected: toadstool 0.1.0

# 2. Help
toadstool --help
# Expected: Full command list

# 3. Capabilities
toadstool capabilities
# Expected: System info, available runtimes

# 4. Validate example biome
toadstool validate examples/simple-biome.yaml
# Expected: ✅ Valid

# 5. Run test biome
toadstool run examples/simple-biome.yaml --dry-run
# Expected: No errors

# 6. Check binary size
ls -lh /usr/local/bin/toadstool
# Expected: ~80MB (release) or ~311MB (debug)

# 7. Verify pure Rust (no TLS)
ldd /usr/local/bin/toadstool | grep -i ssl
# Expected: No output (zero TLS dependencies!)
```

---

## 🚀 **NEXT STEPS**

### **After Deployment**

1. **Create biome.yaml**: Define your compute environment
2. **Start biome**: `toadstool up biome.yaml`
3. **Monitor**: `toadstool ps` and `toadstool logs`
4. **Integrate**: Connect with other primals via capabilities

---

### **Advanced Features**

```bash
# GPU compute (if available)
toadstool capabilities --gpu

# Universal compute
toadstool universal --help

# Direct workload execution
toadstool execute workload.toml

# Ecosystem integration
toadstool ecosystem --help
```

---

## 📚 **DOCUMENTATION**

### **Quick Reference**

- **Quick Start**: [START_HERE.md](START_HERE.md)
- **Full Documentation**: [DOCUMENTATION.md](DOCUMENTATION.md)
- **Architecture**: [README.md](README.md)
- **Testing**: [TESTING.md](TESTING.md)

---

### **Evolution History**

- **v4.10.0**: Pure Rust + UniBin (THIS VERSION)
- **v4.9.0**: Pure Rust core achieved
- **v4.4.0**: Async execution, 8.80x NVIDIA performance

See [CHANGELOG.md](CHANGELOG.md) for full history.

---

## 🎊 **DEPLOYMENT CHECKLIST**

### **Pre-Deployment**

- [ ] Verify build: `cargo build --release --bin toadstool`
- [ ] Run tests: `cargo test --workspace`
- [ ] Check binary: `./target/release/toadstool --version`

### **Deployment**

- [ ] Copy binary: `sudo cp target/release/toadstool /usr/local/bin/`
- [ ] Set permissions: `sudo chmod 755 /usr/local/bin/toadstool`
- [ ] Create user: `sudo useradd -r -s /bin/false toadstool`
- [ ] Configure directories: `/var/lib/toadstool`, `/var/run/toadstool`

### **Post-Deployment**

- [ ] Verify version: `toadstool --version`
- [ ] Check capabilities: `toadstool capabilities`
- [ ] Test biome: `toadstool validate examples/simple-biome.yaml`
- [ ] Start daemon: `toadstool daemon` or systemd service

### **Production**

- [ ] Configure systemd service
- [ ] Set up logging
- [ ] Configure monitoring
- [ ] Document backup procedures
- [ ] Test failover scenarios

---

## 💡 **TROUBLESHOOTING**

### **Common Issues**

**Binary not found**:
```bash
# Add to PATH or use full path
export PATH="/usr/local/bin:$PATH"
```

**Permission denied**:
```bash
# Check permissions
ls -l /usr/local/bin/toadstool
# Fix if needed
sudo chmod 755 /usr/local/bin/toadstool
```

**ARM cross-compile fails**:
```bash
# Install ARM toolchain
sudo apt install gcc-aarch64-linux-gnu
```

**ldd shows TLS libraries**:
```bash
# Should NOT happen in v4.10.0 (pure Rust!)
# If you see SSL/TLS, verify you have latest build
cargo clean && cargo build --release --bin toadstool
```

---

## 🎯 **SUPPORT**

**Repository**: https://github.com/ecoPrimals/toadStool  
**Documentation**: [docs/](docs/)  
**Issues**: GitHub Issues  
**Grade**: A++ (100/100) - Production Ready

---

**Created**: January 16, 2026  
**Version**: v4.10.0  
**Status**: Production Approved

🦀 **PURE RUST + UNIBIN - READY FOR DEPLOYMENT!** 🦀✨
