# mDNS Service Discovery Guide

**Status**: ✅ Implemented (November 18, 2025)  
**Component**: Infant Discovery System - Phase 3

---

## 🎯 What is mDNS Discovery?

**mDNS** (Multicast DNS) enables **zero-configuration** service discovery on local networks.

Services announce themselves on the network, and ToadStool automatically discovers them **without any configuration files or environment variables**.

---

## 🏗️ How It Works

### 1. Service Advertisement

Services advertise themselves using DNS-SD (DNS Service Discovery) over mDNS:

```bash
# Service announces itself on the network
# Pattern: _<capability>-service._tcp.local

_crypto-service._tcp.local       # Crypto capability provider
_storage-service._tcp.local      # Storage capability provider
_coord-service._tcp.local        # Coordination capability provider
_messaging-service._tcp.local    # Messaging capability provider
```

### 2. ToadStool Discovery

ToadStool listens for these announcements and automatically discovers available services:

```rust
use crate::ecosystem::discovery::discover_service_by_capability;

// Automatically discovers via mDNS (no config needed!)
let endpoints = discover_service_by_capability("crypto").await?;

// Returns all crypto services advertising on the network
for endpoint in endpoints {
    println!("Found crypto service at: {}", endpoint.address);
}
```

### 3. Discovery Priority

mDNS is **third** in the discovery priority chain:

1. **Environment variables** (highest priority)
2. **Configuration files**
3. **mDNS discovery** ← Zero-config!
4. **Service mesh** (lowest priority)

---

## 📡 Service Name Format

Services MUST advertise using this naming pattern:

```
_<capability>-service._tcp.local
```

### Examples

| Capability Category | mDNS Service Name |
|---------------------|-------------------|
| `crypto` | `_crypto-service._tcp.local` |
| `storage` | `_storage-service._tcp.local` |
| `coordination` | `_coord-service._tcp.local` |
| `compute` | `_compute-service._tcp.local` |
| `messaging` | `_messaging-service._tcp.local` |
| `monitoring` | `_monitoring-service._tcp.local` |
| `auth` | `_auth-service._tcp.local` |

---

## 🚀 Setting Up a Service for mDNS

### For ecoPrimals (BearDog, NestGate, Songbird, Squirrel)

Each primal should advertise its capabilities via mDNS.

#### Example: BearDog (Crypto Service)

```rust
use mdns::Responder;
use std::time::Duration;

// Create mDNS responder
let responder = Responder::new()?;

// Register crypto service
let service = responder.register(
    "_crypto-service._tcp.local".to_string(),
    "BearDog Crypto Service".to_string(),
    9876,  // Port
    &["capability=crypto", "version=1.0.0"]
);

println!("✅ BearDog advertising crypto capability on port 9876");

// Keep service registered
std::thread::park();
```

#### Example: NestGate (Storage Service)

```rust
let responder = Responder::new()?;

let service = responder.register(
    "_storage-service._tcp.local".to_string(),
    "NestGate Storage Service".to_string(),
    8082,  // Port
    &["capability=storage", "filesystem=zfs", "version=1.0.0"]
);

println!("✅ NestGate advertising storage capability on port 8082");
```

#### Example: Songbird (Coordination Service)

```rust
let responder = Responder::new()?;

let service = responder.register(
    "_coord-service._tcp.local".to_string(),
    "Songbird Coordination Service".to_string(),
    8080,  // Port
    &["capability=coordination", "version=1.0.0"]
);

println!("✅ Songbird advertising coordination capability on port 8080");
```

### For Custom Services

Any service can advertise capabilities:

```rust
// AWS KMS advertising crypto capability
let service = responder.register(
    "_crypto-service._tcp.local".to_string(),
    "AWS KMS Proxy".to_string(),
    9999,
    &["capability=crypto", "provider=aws-kms"]
);

// MinIO advertising storage capability
let service = responder.register(
    "_storage-service._tcp.local".to_string(),
    "MinIO Object Storage".to_string(),
    9000,
    &["capability=storage", "type=object-store"]
);
```

---

## 🧪 Testing mDNS Discovery

### 1. Start a Test Service

```bash
# Terminal 1: Start a mock service advertising crypto capability
# (Use any mDNS responder tool or library)

# Example using avahi-publish (Linux)
avahi-publish -s "BearDog Test" _crypto-service._tcp 9876 "capability=crypto"

# Example using dns-sd (macOS)
dns-sd -R "BearDog Test" _crypto-service._tcp local 9876
```

### 2. Test Discovery from ToadStool

```rust
use toadstool::ecosystem::discovery::discover_service_by_capability;

#[tokio::test]
async fn test_mdns_discovery() {
    // Discover crypto services via mDNS
    let endpoints = discover_service_by_capability("crypto").await.unwrap();
    
    assert!(!endpoints.is_empty(), "Should discover at least one service");
    
    for endpoint in endpoints {
        println!("Discovered: {} at {}", 
            endpoint.service_type.name(), 
            endpoint.address
        );
        assert_eq!(endpoint.trust_level, TrustLevel::Advertised);
    }
}
```

### 3. Browse Available Services

```bash
# List all services on the network

# Linux (Avahi)
avahi-browse -a

# macOS/Windows
dns-sd -B _services._dns-sd._udp

# Look for:
# - _crypto-service._tcp
# - _storage-service._tcp
# - _coord-service._tcp
```

---

## 🔒 Security Considerations

### Trust Level: Advertised

mDNS-discovered services have `TrustLevel::Advertised`, indicating:
- ✅ Service was found on the local network
- ⚠️ Identity has NOT been cryptographically verified
- ⚠️ Could potentially be malicious

### Recommended Security Practices

1. **Verify Before Use**: Always verify discovered services before using them for sensitive operations.

```rust
let endpoints = discover_service_by_capability("crypto").await?;

for endpoint in endpoints {
    // Verify service identity before using
    if verify_service(&endpoint).await? {
        endpoint.trust_level = TrustLevel::Verified;
        // Now safe to use
    }
}
```

2. **Use TLS**: Always use TLS/HTTPS for service communication.

3. **Network Isolation**: Use mDNS discovery only on trusted networks.

4. **Prefer Configuration**: For production, prefer environment variables or config files over mDNS.

---

## 🌐 Network Requirements

### Firewall Rules

mDNS uses **UDP port 5353** for multicast DNS queries:

```bash
# Allow mDNS traffic (port 5353)
sudo ufw allow 5353/udp  # Linux
```

### Multicast Address

mDNS uses the multicast address:
- **IPv4**: `224.0.0.251`
- **IPv6**: `FF02::FB`

Ensure your network allows multicast traffic.

### Docker/Containers

For mDNS to work in containers:

```yaml
# docker-compose.yml
services:
  toadstool:
    network_mode: "host"  # Required for mDNS
    # OR
    networks:
      - host
```

---

## 🧭 Discovery Flow

### Complete Discovery Chain

```
1. Check Environment Variables
   ↓ Not found
2. Check Configuration Files
   ↓ Not found
3. mDNS Discovery (2-second scan) ← You are here
   ↓ Not found
4. Service Mesh Query
   ↓ Not found
5. Return error (no service found)
```

### Example: Full Discovery

```rust
// ToadStool tries discovery methods in order

// 1. Environment variable (fastest, explicit)
if let Ok(url) = std::env::var("TOADSTOOL_CRYPTO_SERVICE_URL") {
    return Ok(vec![parse_endpoint(url)?]);
}

// 2. Config file (persistent, user-configured)
if let Some(url) = discover_from_config("crypto") {
    return Ok(vec![parse_endpoint(url)?]);
}

// 3. mDNS discovery (zero-config, automatic)
if let Ok(endpoints) = discover_via_mdns("crypto").await {
    if !endpoints.is_empty() {
        return Ok(endpoints);
    }
}

// 4. Service mesh (cluster-aware)
// ... (not yet implemented)

// No service found
Err(anyhow!("No crypto service found"))
```

---

## 📊 Discovery Timeout

mDNS discovery waits **2 seconds** for responses.

This balances:
- **Speed**: Fast enough for good UX
- **Completeness**: Long enough for most services to respond

### Adjusting Timeout

To change the timeout, modify `discover_via_mdns()` in `discovery.rs`:

```rust
// Change from 2 seconds to 5 seconds
let mdns_service = mdns::discover::all(&service_name, StdDuration::from_secs(5))?;
let timeout = StdDuration::from_secs(5);
```

---

## 🎓 Best Practices

### For Service Providers (BearDog, NestGate, etc.)

1. **Advertise Early**: Start mDNS advertisement as soon as the service is ready.
2. **Use Standard Names**: Follow the `_<capability>-service._tcp.local` pattern.
3. **Include Metadata**: Add useful TXT records (version, capabilities, etc.).
4. **Keep Alive**: Maintain the mDNS registration for the service lifetime.

### For Service Consumers (ToadStool)

1. **Prefer Explicit Config**: Use environment variables or config files when available.
2. **Verify Services**: Always verify mDNS-discovered services before sensitive operations.
3. **Cache Results**: Cache discovered endpoints to avoid repeated scans.
4. **Handle Failures**: Gracefully handle cases where no services are found.

---

## 🐛 Troubleshooting

### No Services Discovered

**Check 1**: Is the service advertising?
```bash
# Linux
avahi-browse -a | grep -i crypto

# macOS
dns-sd -B _crypto-service._tcp local
```

**Check 2**: Is multicast enabled on your network?
```bash
# Check multicast routes
ip route show | grep 224.0.0.0

# Should see something like:
# 224.0.0.0/4 dev eth0 scope link
```

**Check 3**: Firewall blocking port 5353?
```bash
# Temporarily disable firewall to test
sudo ufw disable  # Linux
```

**Check 4**: Running in Docker without host networking?
```yaml
# Use host networking for mDNS
network_mode: "host"
```

### Services Discovered But Connection Fails

**Check 1**: Is the service actually running on the advertised port?
```bash
nc -zv <ip> <port>
```

**Check 2**: Firewall blocking the service port?
```bash
# Allow the service port
sudo ufw allow <port>
```

---

## ✅ Status & Roadmap

### ✅ Implemented
- mDNS discovery function
- Service name pattern (`_<capability>-service._tcp.local`)
- 2-second discovery timeout
- Trust level: Advertised
- Integration with discovery chain

### ⏳ TODO
- Add mDNS advertisement support to ecoPrimals
  - BearDog (crypto)
  - NestGate (storage)
  - Songbird (coordination)
  - Squirrel (messaging)
- Add mDNS discovery tests
- Add retry logic for failed discoveries
- Add discovery caching

---

## 📚 References

- **mdns crate**: https://crates.io/crates/mdns
- **DNS-SD RFC 6763**: https://tools.ietf.org/html/rfc6763
- **mDNS RFC 6762**: https://tools.ietf.org/html/rfc6762
- **Avahi**: https://www.avahi.org/
- **Bonjour**: https://developer.apple.com/bonjour/

---

**Last Updated**: November 18, 2025  
**Status**: ✅ Ready for use  
**Next**: Add advertisement support to ecoPrimals

🍄 **ToadStool: Truly Universal Compute, Zero-Configuration Discovery.** 🚀

