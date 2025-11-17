# 🔍 48-Hour Staging Monitoring Guide

**Start Date**: November 15, 2025  
**End Date**: November 17-18, 2025  
**Binary**: `/usr/local/bin/toadstool-staging`  
**Version**: toadstool 0.1.0

---

## 📋 MONITORING CHECKLIST

### Hour 0 (Deployment Complete) ✅
- [x] Binary deployed successfully
- [x] Version verified: toadstool 0.1.0
- [x] Capabilities tested
- [x] Documentation complete
- [ ] **Initial smoke test** ⬅️ START HERE

---

## 🚀 QUICK START MONITORING

### 1. Basic Health Check (Every 2-4 Hours)

```bash
# Check version (should respond instantly)
toadstool-staging --version

# Test capabilities
toadstool-staging capabilities

# Check if process is running (if daemon)
ps aux | grep toadstool-staging

# Check system resources
htop  # Press 'q' to exit
```

**Expected**: 
- Version displays: `toadstool 0.1.0`
- Capabilities show security warnings (expected)
- Process responds quickly (<1 second)

---

### 2. Log Monitoring (If Running as Service)

```bash
# Live log monitoring
journalctl -u toadstool-staging -f

# Check for errors in last hour
journalctl -u toadstool-staging --since "1 hour ago" | grep -i error

# Check for warnings
journalctl -u toadstool-staging --since "1 hour ago" | grep -i warn

# Last 50 log lines
journalctl -u toadstool-staging -n 50
```

**Expected**:
- Security warnings present (expected for v0.1.0)
- No crashes or panics
- No unexpected errors

---

### 3. System Resource Check

```bash
# CPU and Memory usage
top -b -n 1 | grep toadstool

# Disk usage
df -h

# Memory details
free -h

# Network connections (if applicable)
netstat -tuln | grep toadstool || echo "No network connections"
```

**Expected**:
- CPU usage: <50% (should be idle most of the time)
- Memory usage: <500MB for idle process
- Disk space: Adequate for logs
- Network: Depends on configuration

---

## 📊 HOURLY MONITORING LOG

### Template (Copy for Each Check)

```
=== MONITORING LOG ===
Date/Time: _______________
Hour Since Deploy: _______

✅ Binary Status:
   - Version check: [ ] Pass [ ] Fail
   - Response time: _______ seconds
   - Capabilities: [ ] Pass [ ] Fail

✅ System Resources:
   - CPU: _______% 
   - Memory: _______MB
   - Disk: _______GB free
   
✅ Logs:
   - New errors: [ ] None [ ] Found: _______
   - Warnings: [ ] Expected only [ ] Unexpected: _______
   - Crashes: [ ] None [ ] Found: _______

✅ Tests Run:
   - Test executed: _______
   - Result: [ ] Pass [ ] Fail
   - Notes: _______

Overall Status: [ ] GOOD [ ] CONCERN [ ] CRITICAL

Notes:
_________________________________
_________________________________
```

---

## 🔍 DETAILED MONITORING SCHEDULE

### Hour 1 (Immediate After Deployment)
**Actions:**
- [x] Deployment complete
- [ ] Run smoke tests
- [ ] Check initial logs
- [ ] Verify binary responds
- [ ] Document baseline resource usage

**Commands:**
```bash
# Smoke test
toadstool-staging --version
toadstool-staging --help
toadstool-staging capabilities

# Baseline resources
top -b -n 1 | head -20
free -h
df -h
```

---

### Hours 2-4 (First Check)
**Actions:**
- [ ] Re-run smoke tests
- [ ] Check for new logs/errors
- [ ] Verify no resource leaks
- [ ] Test a simple workflow (if applicable)

**Commands:**
```bash
# Quick health check
toadstool-staging --version

# Check logs since deployment
journalctl -u toadstool-staging --since "2 hours ago"

# Resource check
ps aux | grep toadstool-staging
```

---

### Hours 6-8 (Second Check)
**Actions:**
- [ ] Full smoke test
- [ ] Review accumulated logs
- [ ] Check for patterns
- [ ] Document any issues

---

### Hour 12 (Halfway Through Day 1)
**Actions:**
- [ ] Comprehensive check
- [ ] Test all basic commands
- [ ] Review all logs
- [ ] Document stability

**Commands:**
```bash
# Full test
toadstool-staging --version
toadstool-staging --help
toadstool-staging capabilities

# Full log review
journalctl -u toadstool-staging --since "12 hours ago" | less

# Resource trend
echo "=== 12 Hour Resource Check ===" >> /tmp/toadstool-monitoring.log
top -b -n 1 | head -20 >> /tmp/toadstool-monitoring.log
free -h >> /tmp/toadstool-monitoring.log
```

---

### Hour 24 (End of Day 1)
**Actions:**
- [ ] Full 24-hour review
- [ ] Analyze all logs
- [ ] Check resource trends
- [ ] Document Day 1 stability
- [ ] Decision: Continue or rollback

**Evaluation Criteria:**
- [ ] Zero crashes
- [ ] Error rate <0.1%
- [ ] Resource usage stable
- [ ] All tests pass
- [ ] No critical issues

**Decision:**
- [ ] ✅ Continue to Day 2
- [ ] ⚠️ Investigate issues
- [ ] 🚨 Rollback needed

---

### Day 2 (Hours 24-48)
**Actions:**
- [ ] Morning check (8am)
- [ ] Midday check (12pm)
- [ ] Afternoon check (4pm)
- [ ] Evening check (8pm)
- [ ] End of Day 2 review

**Each Check:**
```bash
# Quick status
toadstool-staging --version && echo "✅ Responding"

# New errors
journalctl -u toadstool-staging --since "4 hours ago" | grep -i error

# Resource trend
ps aux | grep toadstool-staging | awk '{print "CPU: "$3"% MEM: "$4"%"}'
```

---

### Day 3 (Hours 48-72)
**Actions:**
- [ ] Morning check
- [ ] Midday check
- [ ] Evening check
- [ ] Final 72-hour review

**72-Hour Review:**
- [ ] Total uptime: _______
- [ ] Total crashes: _______
- [ ] Error rate: _______%
- [ ] Memory leaks: [ ] None detected
- [ ] CPU issues: [ ] None detected
- [ ] All tests passing: [ ] Yes [ ] No

---

## 🚨 ISSUE RESPONSE GUIDE

### Minor Issue (Warning/Non-Critical)
**Action:**
1. Document in monitoring log
2. Continue monitoring
3. Note pattern if recurring
4. Address in next update

**Example:** Unexpected warning message

---

### Moderate Issue (Impacts Function)
**Action:**
1. Document issue with details
2. Attempt to reproduce
3. Check if workaround exists
4. Create GitHub issue
5. Consider if rollback needed

**Example:** Command fails intermittently

---

### Critical Issue (Crash/Data Loss)
**Action:**
1. 🚨 **IMMEDIATE ROLLBACK**
2. Capture all logs
3. Document exact conditions
4. Create detailed bug report
5. Fix before redeployment

**Rollback Command:**
```bash
# If backup exists
sudo cp /usr/local/bin/toadstool-staging.backup.* /usr/local/bin/toadstool-staging

# Or remove
sudo rm /usr/local/bin/toadstool-staging

# Verify
toadstool-staging --version || echo "Rollback complete"
```

---

## 📈 SUCCESS METRICS

### Required for Production (All Must Pass)

**Stability:**
- [ ] Zero crashes in 72 hours
- [ ] Uptime: >99.9%
- [ ] Process restarts: 0

**Performance:**
- [ ] Response time: <1 second for --version
- [ ] CPU usage: <50% average, <80% peak
- [ ] Memory usage: Stable (no leaks)

**Reliability:**
- [ ] All smoke tests passing
- [ ] Error rate: <0.1%
- [ ] No data corruption
- [ ] No security incidents

**Functionality:**
- [ ] All commands respond
- [ ] Capabilities list correctly
- [ ] Help text displays
- [ ] Critical workflows work (if applicable)

---

## ✅ PRODUCTION READINESS DECISION

### At 72 Hours, Answer These:

1. **Did it crash?** [ ] No [ ] Yes → If yes, ROLLBACK
2. **Error rate <0.1%?** [ ] Yes [ ] No → If no, investigate
3. **Memory leaks?** [ ] None [ ] Detected → If detected, fix
4. **CPU spikes?** [ ] None [ ] Frequent → If frequent, optimize
5. **Tests passing?** [ ] All [ ] Some failing → If failing, fix
6. **Ready for production?** [ ] Yes [ ] No [ ] Maybe

### Decision Tree:
```
All "Yes" above? 
  ├─ YES → ✅ DEPLOY TO PRODUCTION
  └─ NO → 
      ├─ Minor issues → Investigate & fix, then redeploy staging
      └─ Major issues → Rollback, fix, restart staging
```

---

## 📊 MONITORING TOOLS

### Built-in Linux Tools
```bash
# Process monitoring
top          # Interactive process viewer
htop         # Better top (install if needed)
ps aux       # Process list

# Resource monitoring  
free -h      # Memory usage
df -h        # Disk usage
uptime       # System uptime

# Log viewing
journalctl   # Systemd logs
tail -f      # Follow log file
less         # View log file
grep         # Search logs
```

### Custom Monitoring Script
```bash
#!/bin/bash
# Save as: monitor-toadstool.sh

echo "=== ToadStool Staging Health Check ==="
echo "Time: $(date)"
echo ""

# Version check
echo "1. Version:"
toadstool-staging --version 2>&1 || echo "❌ Not responding"
echo ""

# Resource check
echo "2. Resources:"
ps aux | grep toadstool-staging | grep -v grep || echo "Not running as daemon"
echo ""

# Recent errors
echo "3. Recent Errors:"
journalctl -u toadstool-staging --since "1 hour ago" 2>/dev/null | grep -i error | tail -5 || echo "No systemd service or no errors"
echo ""

# System health
echo "4. System:"
uptime
free -h | grep Mem
echo ""

echo "=== Check Complete ==="
```

**Usage:**
```bash
chmod +x monitor-toadstool.sh
./monitor-toadstool.sh
```

---

## 📝 DAILY SUMMARY TEMPLATE

### Day 1 Summary (Nov 15, 2025)
```
Deployment Time: _______
Total Checks: _______
Issues Found: _______
  - Critical: _______
  - Moderate: _______
  - Minor: _______
Uptime: _______
Average Response Time: _______
Status: [ ] STABLE [ ] UNSTABLE [ ] ROLLBACK NEEDED

Notes:
_________________________________
_________________________________

Continue? [ ] YES [ ] NO
```

### Day 2 Summary (Nov 16, 2025)
```
[Same template as Day 1]
```

### Day 3 Summary (Nov 17, 2025)
```
[Same template as Day 1]
```

### Final 72-Hour Summary
```
Total Monitoring Period: 72 hours
Total Uptime: _______
Total Issues: _______
Critical Issues: _______
Resolution Time: _______

Production Ready? [ ] YES [ ] NO

Reasoning:
_________________________________
_________________________________

Next Steps:
_________________________________
_________________________________
```

---

## 🎯 MONITORING CHECKLIST SUMMARY

**Every 2-4 Hours (First 24 Hours):**
- [ ] Run: `toadstool-staging --version`
- [ ] Check logs for errors
- [ ] Monitor resource usage
- [ ] Document status

**Every 4-6 Hours (Day 2-3):**
- [ ] Quick health check
- [ ] Review log patterns
- [ ] Check resource trends
- [ ] Update monitoring log

**End of 72 Hours:**
- [ ] Complete final review
- [ ] Make production decision
- [ ] Document lessons learned
- [ ] Plan next deployment

---

## 📞 CONTACTS & RESOURCES

**Documentation:**
- Full Audit: `COMPREHENSIVE_AUDIT_FINAL_NOV_15_2025.md`
- Deployment: `DEPLOYMENT_SUCCESS_NOV_15_2025.md`
- Mission Complete: `🎉_MISSION_COMPLETE_NOV_15_2025.md`

**Support:**
- Rollback: See "ISSUE RESPONSE GUIDE" above
- Bug Reporting: Create GitHub issue
- Questions: Review audit documentation

---

## ✅ START MONITORING NOW

**Your first check:**
```bash
# Run this now
toadstool-staging --version
toadstool-staging capabilities

# Document results
echo "First check completed at $(date)" >> /tmp/toadstool-monitoring.log
```

**Then set calendar reminders:**
- Today, in 4 hours
- Today, in 8 hours  
- Today, at bedtime
- Tomorrow morning
- Tomorrow afternoon
- Tomorrow evening
- Day 3 morning
- Day 3 final check

---

**Status**: ⏳ **MONITORING IN PROGRESS**  
**Duration**: 48-72 hours  
**Next Review**: November 17-18, 2025

---

*Monitor actively. Document everything. Deploy to production when stable.*

🍄🔍 **Happy monitoring!**

