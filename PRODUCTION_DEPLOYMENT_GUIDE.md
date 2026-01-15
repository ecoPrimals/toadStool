# Production Deployment Guide

**Version**: 0.7.0  
**Date**: January 15, 2026  
**Status**: ✅ Production Ready (A+ Grade)

---

## 🎯 OVERVIEW

This guide provides step-by-step instructions for deploying ToadStool to production environments. ToadStool has achieved **A+ grade (98/100)** with comprehensive testing, performance optimization, and Deep Debt architectural compliance.

**Prerequisites**: See [Deployment Readiness Assessment](DEPLOYMENT_READINESS_ASSESSMENT.md)

---

## 📋 PRE-DEPLOYMENT CHECKLIST

### Environment Setup ✅

- [ ] Production server provisioned (Linux, macOS, or Windows)
- [ ] Rust toolchain installed (1.75.0+)
- [ ] Docker installed (if using containers)
- [ ] Environment variables configured
- [ ] Network ports available (8080-8083, 9090)
- [ ] Monitoring tools ready (Prometheus, Grafana)
- [ ] Log aggregation configured

### Configuration ✅

- [ ] Review `toadstool.toml` configuration
- [ ] Set capability environment variables
- [ ] Configure bind addresses and ports
- [ ] Set log levels appropriately
- [ ] Configure service discovery
- [ ] Set resource limits

### Security ✅

- [ ] Secrets stored securely (environment variables)
- [ ] TLS certificates ready (if using HTTPS)
- [ ] Firewall rules configured
- [ ] Authentication configured (Beardog integration)
- [ ] Authorization policies reviewed

---

## 🚀 DEPLOYMENT METHODS

### Method 1: Native Binary (Recommended)

**Best for**: Direct deployment, maximum performance

#### Build Production Binary

```bash
# Clone repository
git clone https://github.com/your-org/toadstool
cd toadstool

# Build in release mode
cargo build --release --workspace

# Binary location
ls -lh target/release/toadstool
```

#### Configure Environment

```bash
# Create .env file
cat > /opt/toadstool/.env <<EOF
# ToadStool Configuration

# Self-knowledge (ToadStool's own endpoints)
BIND_ADDRESS=0.0.0.0
TOADSTOOL_API_PORT=8080

# Capability-based service discovery
TOADSTOOL_COORDINATION_SERVICE_URL=http://songbird:8080
TOADSTOOL_CRYPTO_SERVICE_URL=http://beardog:8081
TOADSTOOL_STORAGE_SERVICE_URL=http://nestgate:8082
TOADSTOOL_AI_SERVICE_URL=http://squirrel:6000

# Monitoring
PROMETHEUS_PORT=9090
METRICS_PORT=9090
HEALTH_CHECK_PORT=8082

# Logging
RUST_LOG=info
LOG_FORMAT=json

# Performance
TOKIO_WORKER_THREADS=4
EOF
```

#### Run Service

```bash
# Copy binary
sudo cp target/release/toadstool /usr/local/bin/

# Create systemd service
sudo cat > /etc/systemd/system/toadstool.service <<EOF
[Unit]
Description=ToadStool Universal Compute Platform
After=network.target

[Service]
Type=simple
User=toadstool
Group=toadstool
WorkingDirectory=/opt/toadstool
EnvironmentFile=/opt/toadstool/.env
ExecStart=/usr/local/bin/toadstool server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Start service
sudo systemctl daemon-reload
sudo systemctl enable toadstool
sudo systemctl start toadstool
sudo systemctl status toadstool
```

### Method 2: Docker Container

**Best for**: Containerized deployments, Kubernetes

#### Build Docker Image

```bash
# Create Dockerfile
cat > Dockerfile <<EOF
FROM rust:1.75-slim as builder

WORKDIR /build
COPY . .

# Build release binary
RUN cargo build --release --workspace

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/toadstool /usr/local/bin/

# Create non-root user
RUN useradd -m -U toadstool
USER toadstool

# Expose ports
EXPOSE 8080 8082 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8082/health || exit 1

ENTRYPOINT ["/usr/local/bin/toadstool"]
CMD ["server"]
EOF

# Build image
docker build -t toadstool:0.7.0 .

# Tag for registry
docker tag toadstool:0.7.0 registry.ecosystem.sovereignscience.org/toadstool:0.7.0
docker tag toadstool:0.7.0 registry.ecosystem.sovereignscience.org/toadstool:latest

# Push to registry
docker push registry.ecosystem.sovereignscience.org/toadstool:0.7.0
docker push registry.ecosystem.sovereignscience.org/toadstool:latest
```

#### Run Docker Container

```bash
# Run with environment variables
docker run -d \
    --name toadstool \
    --restart unless-stopped \
    -p 8080:8080 \
    -p 8082:8082 \
    -p 9090:9090 \
    -e BIND_ADDRESS=0.0.0.0 \
    -e TOADSTOOL_API_PORT=8080 \
    -e TOADSTOOL_COORDINATION_SERVICE_URL=http://songbird:8080 \
    -e TOADSTOOL_CRYPTO_SERVICE_URL=http://beardog:8081 \
    -e TOADSTOOL_STORAGE_SERVICE_URL=http://nestgate:8082 \
    -e RUST_LOG=info \
    registry.ecosystem.sovereignscience.org/toadstool:0.7.0

# Check status
docker ps | grep toadstool
docker logs toadstool
```

### Method 3: Kubernetes Deployment

**Best for**: Cloud-native, highly available deployments

#### Kubernetes Manifests

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: toadstool
  labels:
    app: toadstool
spec:
  replicas: 3
  selector:
    matchLabels:
      app: toadstool
  template:
    metadata:
      labels:
        app: toadstool
    spec:
      containers:
      - name: toadstool
        image: registry.ecosystem.sovereignscience.org/toadstool:0.7.0
        ports:
        - containerPort: 8080
          name: api
        - containerPort: 8082
          name: health
        - containerPort: 9090
          name: metrics
        env:
        - name: BIND_ADDRESS
          value: "0.0.0.0"
        - name: TOADSTOOL_API_PORT
          value: "8080"
        - name: TOADSTOOL_COORDINATION_SERVICE_URL
          value: "http://songbird:8080"
        - name: TOADSTOOL_CRYPTO_SERVICE_URL
          value: "http://beardog:8081"
        - name: TOADSTOOL_STORAGE_SERVICE_URL
          value: "http://nestgate:8082"
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8082
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8082
          initialDelaySeconds: 5
          periodSeconds: 5

---
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: toadstool
spec:
  selector:
    app: toadstool
  ports:
  - name: api
    port: 8080
    targetPort: 8080
  - name: health
    port: 8082
    targetPort: 8082
  - name: metrics
    port: 9090
    targetPort: 9090
  type: ClusterIP

---
# servicemonitor.yaml (for Prometheus Operator)
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: toadstool
spec:
  selector:
    matchLabels:
      app: toadstool
  endpoints:
  - port: metrics
    interval: 30s
```

#### Deploy to Kubernetes

```bash
# Apply manifests
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f servicemonitor.yaml

# Check deployment
kubectl get deployments
kubectl get pods -l app=toadstool
kubectl logs -l app=toadstool

# Check service
kubectl get svc toadstool
```

---

## 🔧 CONFIGURATION

### Environment Variables

#### Required (Self-Knowledge)

```bash
# ToadStool's own bind address and port
BIND_ADDRESS=0.0.0.0
TOADSTOOL_API_PORT=8080
```

#### Optional (Capability Discovery)

```bash
# Coordination service (Songbird)
TOADSTOOL_COORDINATION_SERVICE_URL=http://songbird:8080

# Crypto service (Beardog)
TOADSTOOL_CRYPTO_SERVICE_URL=http://beardog:8081

# Storage service (NestGate)
TOADSTOOL_STORAGE_SERVICE_URL=http://nestgate:8082

# AI service (Squirrel)
TOADSTOOL_AI_SERVICE_URL=http://squirrel:6000
```

If not set, falls back to localhost defaults.

#### Monitoring

```bash
# Prometheus metrics endpoint
PROMETHEUS_PORT=9090
METRICS_PORT=9090

# Health check endpoint
HEALTH_CHECK_PORT=8082
```

#### Logging

```bash
# Log level (error, warn, info, debug, trace)
RUST_LOG=info

# Log format (text, json)
LOG_FORMAT=json
```

#### Performance

```bash
# Tokio runtime threads (default: number of cores)
TOKIO_WORKER_THREADS=4
```

### Configuration File (toadstool.toml)

```toml
[network]
bind_address = "0.0.0.0"
api_port = 8080
health_port = 8082
metrics_port = 9090
federation_port = 8084

[logging]
level = "info"
format = "json"

[performance]
tokio_worker_threads = 4

[security]
# Authentication via Beardog
beardog_enabled = true

[features]
# Enable GPU support
gpu_enabled = true

# Enable secure enclave
secure_enclave_enabled = true
```

---

## 📊 MONITORING

### Health Checks

```bash
# Health check endpoint
curl http://localhost:8082/health

# Expected response (HTTP 200)
{
  "status": "healthy",
  "version": "0.7.0",
  "uptime_seconds": 3600
}
```

### Metrics

```bash
# Prometheus metrics endpoint
curl http://localhost:9090/metrics

# Example metrics
# HELP toadstool_requests_total Total number of requests
# TYPE toadstool_requests_total counter
toadstool_requests_total{endpoint="/api/v1/workloads"} 1234

# HELP toadstool_request_duration_seconds Request duration
# TYPE toadstool_request_duration_seconds histogram
toadstool_request_duration_seconds_sum{endpoint="/api/v1/workloads"} 12.34
```

### Logs

```bash
# View logs (systemd)
journalctl -u toadstool -f

# View logs (Docker)
docker logs -f toadstool

# View logs (Kubernetes)
kubectl logs -f deployment/toadstool
```

### Prometheus Configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'toadstool'
    static_configs:
      - targets: ['toadstool:9090']
    scrape_interval: 30s
```

### Grafana Dashboards

Import ToadStool dashboard from `docs/monitoring/grafana-dashboard.json`

**Key Panels**:
- Request rate
- Error rate
- Latency percentiles (P50, P95, P99)
- Active connections
- Resource usage (CPU, memory)
- Service discovery timing

---

## 🔐 SECURITY

### TLS Configuration

```bash
# Generate self-signed certificate (development)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Configure TLS
export TOADSTOOL_TLS_CERT=/path/to/cert.pem
export TOADSTOOL_TLS_KEY=/path/to/key.pem
```

### Firewall Rules

```bash
# Allow API port
sudo ufw allow 8080/tcp

# Allow health check port (internal only)
sudo ufw allow from 10.0.0.0/8 to any port 8082

# Allow metrics port (internal only)
sudo ufw allow from 10.0.0.0/8 to any port 9090
```

### Secrets Management

**Do NOT hardcode secrets!**

```bash
# Use environment variables
export BEARDOG_API_KEY=secret-key-here

# Or use secrets management tools
# - HashiCorp Vault
# - AWS Secrets Manager
# - Kubernetes Secrets
```

---

## 🚦 DEPLOYMENT STRATEGIES

### Blue-Green Deployment

```bash
# Deploy new version (green)
docker run -d --name toadstool-green \
    -p 8081:8080 \
    registry.ecosystem.sovereignscience.org/toadstool:0.7.0

# Test green deployment
curl http://localhost:8081/health

# Switch traffic (nginx/load balancer)
# Update upstream to point to green

# Stop old version (blue)
docker stop toadstool-blue
```

### Canary Deployment

```bash
# Deploy canary (5% traffic)
# Update load balancer to send 5% to new version

# Monitor metrics for 1 hour
# Compare error rates and latency

# Gradually increase (5% → 25% → 50% → 100%)
```

### Rolling Deployment (Kubernetes)

```bash
# Update image
kubectl set image deployment/toadstool \
    toadstool=registry.ecosystem.sovereignscience.org/toadstool:0.7.0

# Watch rollout
kubectl rollout status deployment/toadstool

# Rollback if needed
kubectl rollout undo deployment/toadstool
```

---

## 🔄 ROLLBACK PROCEDURES

### When to Rollback

**Triggers**:
- Error rate >5% increase
- P95 latency >50% increase
- Health check failures
- Critical functionality broken

### Rollback Steps

#### Systemd

```bash
# Stop current version
sudo systemctl stop toadstool

# Restore previous binary
sudo cp /usr/local/bin/toadstool.backup /usr/local/bin/toadstool

# Start service
sudo systemctl start toadstool
sudo systemctl status toadstool
```

#### Docker

```bash
# Stop current container
docker stop toadstool
docker rm toadstool

# Run previous version
docker run -d --name toadstool \
    registry.ecosystem.sovereignscience.org/toadstool:0.6.0
```

#### Kubernetes

```bash
# Rollback to previous revision
kubectl rollout undo deployment/toadstool

# Or rollback to specific revision
kubectl rollout history deployment/toadstool
kubectl rollout undo deployment/toadstool --to-revision=2
```

**Estimated Rollback Time**: <5 minutes

---

## 🧪 POST-DEPLOYMENT VALIDATION

### Smoke Tests

```bash
# Check health
curl http://localhost:8082/health

# Check metrics
curl http://localhost:9090/metrics

# Test API
curl http://localhost:8080/api/v1/health

# Test workload submission
curl -X POST http://localhost:8080/api/v1/workloads \
    -H "Content-Type: application/json" \
    -d '{"type":"test","command":"echo hello"}'
```

### Integration Tests

```bash
# Run E2E tests against production
cargo test --test e2e_tests -- --ignored

# Run smoke tests
bash scripts/smoke-tests.sh
```

### Performance Validation

```bash
# Check latency
ab -n 1000 -c 10 http://localhost:8080/api/v1/health

# Check service discovery timing
time curl http://localhost:8080/api/v1/services
```

---

## 📈 MONITORING & ALERTS

### Key Metrics to Monitor

**Application Metrics**:
- Request rate (requests/sec)
- Error rate (%)
- Request latency (P50, P95, P99)
- Active connections
- Service discovery timing

**System Metrics**:
- CPU usage (%)
- Memory usage (MB)
- Disk I/O
- Network I/O

### Recommended Alerts

```yaml
# Prometheus alerting rules
groups:
  - name: toadstool
    rules:
      # High error rate
      - alert: HighErrorRate
        expr: rate(toadstool_requests_total{status="error"}[5m]) > 0.05
        for: 5m
        annotations:
          summary: "High error rate detected"
      
      # High latency
      - alert: HighLatency
        expr: histogram_quantile(0.95, toadstool_request_duration_seconds) > 1.0
        for: 5m
        annotations:
          summary: "High request latency detected"
      
      # Service down
      - alert: ServiceDown
        expr: up{job="toadstool"} == 0
        for: 1m
        annotations:
          summary: "ToadStool service is down"
```

---

## 🐛 TROUBLESHOOTING

### Common Issues

**Issue**: Service won't start

```bash
# Check logs
journalctl -u toadstool -n 50

# Check port availability
netstat -tuln | grep 8080

# Check environment variables
env | grep TOADSTOOL
```

**Issue**: Service discovery failing

```bash
# Check environment variables
echo $TOADSTOOL_COORDINATION_SERVICE_URL

# Test connectivity
curl http://songbird:8080/health

# Check fallback to localhost
curl http://localhost:8080/health
```

**Issue**: High memory usage

```bash
# Check metrics
curl http://localhost:9090/metrics | grep memory

# Check for memory leaks
ps aux | grep toadstool

# Review resource limits
ulimit -a
```

---

## 📚 ADDITIONAL RESOURCES

- [Deployment Readiness Assessment](DEPLOYMENT_READINESS_ASSESSMENT.md)
- [Benchmark Regression Tracking](BENCHMARK_REGRESSION_TRACKING.md)
- [Deep Debt Evolution Complete](DEEP_DEBT_EVOLUTION_COMPLETE.md)
- [A+ Grade Achievement](A_PLUS_GRADE_ACHIEVED.md)
- [Root Documentation Index](ROOT_DOCS_INDEX.md)

---

## ✅ DEPLOYMENT CHECKLIST

### Pre-Deployment

- [ ] All tests passing (68 suites)
- [ ] Benchmarks validated
- [ ] Configuration reviewed
- [ ] Secrets configured
- [ ] Monitoring ready
- [ ] Rollback plan documented

### Deployment

- [ ] Binary/container built
- [ ] Deployed to staging
- [ ] Smoke tests passed
- [ ] Deployed to canary (5%)
- [ ] Metrics validated
- [ ] Full rollout (100%)

### Post-Deployment

- [ ] Health checks passing
- [ ] Metrics collecting
- [ ] Logs flowing
- [ ] Alerts configured
- [ ] Performance validated
- [ ] Documentation updated

---

## 🎯 SUCCESS CRITERIA

**Day 1**:
- ✅ Service running
- ✅ Health checks passing
- ✅ No critical errors

**Week 1**:
- ✅ Uptime >99.9%
- ✅ Error rate <0.1%
- ✅ P95 latency <100ms

**Month 1**:
- ✅ Uptime >99.95%
- ✅ Error rate <0.05%
- ✅ P95 latency <50ms
- ✅ Zero critical incidents

---

**GUIDE STATUS**: ✅ **READY FOR PRODUCTION**

**Deploy with confidence!** 🚀

---

**Version**: 0.7.0  
**Last Updated**: January 15, 2026  
**Next Review**: After 30 days in production
