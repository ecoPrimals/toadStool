#!/usr/bin/env bash
#
# Hardcoding Migration Script
# Migrates hardcoded values to capability-based discovery
#

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 ToadStool Hardcoding Migration${NC}"
echo -e "${BLUE}===================================${NC}"
echo

# Step 1: Find all usages of deprecated functions
echo -e "${YELLOW}Step 1: Analyzing hardcoded function usage...${NC}"

echo "Finding get_songbird_port() usages..."
SONGBIRD_COUNT=$(grep -r "get_songbird_port()" crates/ --include="*.rs" | wc -l | tr -d ' ')
echo -e "  Found ${YELLOW}${SONGBIRD_COUNT}${NC} usages"

echo "Finding get_beardog_port() usages..."
BEARDOG_COUNT=$(grep -r "get_beardog_port()" crates/ --include="*.rs" | wc -l | tr -d ' ')
echo -e "  Found ${YELLOW}${BEARDOG_COUNT}${NC} usages"

echo "Finding get_nestgate_port() usages..."
NESTGATE_COUNT=$(grep -r "get_nestgate_port()" crates/ --include="*.rs" | wc -l | tr -d ' ')
echo -e "  Found ${YELLOW}${NESTGATE_COUNT}${NC} usages"

echo "Finding get_squirrel_port() usages..."
SQUIRREL_COUNT=$(grep -r "get_squirrel_port()" crates/ --include="*.rs" | wc -l | tr -d ' ')
echo -e "  Found ${YELLOW}${SQUIRREL_COUNT}${NC} usages"

TOTAL=$((SONGBIRD_COUNT + BEARDOG_COUNT + NESTGATE_COUNT + SQUIRREL_COUNT))
echo
echo -e "${GREEN}Total deprecated function calls: ${TOTAL}${NC}"
echo

# Step 2: Find hardcoded localhost references
echo -e "${YELLOW}Step 2: Finding hardcoded localhost references...${NC}"
LOCALHOST_COUNT=$(grep -r "localhost\|127\.0\.0\.1" crates/ --include="*.rs" ! -path "*/tests/*" | wc -l | tr -d ' ')
echo -e "  Found ${YELLOW}${LOCALHOST_COUNT}${NC} localhost references in production code"
echo

# Step 3: Find hardcoded port numbers
echo -e "${YELLOW}Step 3: Finding hardcoded port numbers...${NC}"
PORT_COUNT=$(grep -rE ":8080|:50001|:50002|:50003" crates/ --include="*.rs" ! -path "*/tests/*" | wc -l | tr -d ' ')
echo -e "  Found ${YELLOW}${PORT_COUNT}${NC} hardcoded ports in production code"
echo

# Step 4: Generate migration report
echo -e "${YELLOW}Step 4: Generating migration plan...${NC}"

cat > HARDCODING_MIGRATION_REPORT.md << 'EOF'
# Hardcoding Migration Report
## Generated: $(date)

## Summary

| Type | Count | Priority |
|------|-------|----------|
| Deprecated port functions | ${TOTAL} | HIGH |
| Hardcoded localhost | ${LOCALHOST_COUNT} | HIGH |
| Hardcoded ports | ${PORT_COUNT} | MEDIUM |

## Migration Pattern

### BEFORE (Hardcoded):
```rust
use toadstool_config::config_utils::ConfigUtils;

let port = ConfigUtils::get_songbird_port();
let url = format!("http://localhost:{}", port);
let client = Client::new(url);
```

### AFTER (Capability-Based):
```rust
use toadstool_common::runtime_discovery::RuntimeDiscovery;
use toadstool_common::primal_identity::Capability;

let discovery = RuntimeDiscovery::new(primary_client);
let services = discovery
    .discover_capability(&Capability::Coordination)
    .await?;

// Use first available service
let client = Client::from_service(&services[0]);
```

## Files Requiring Migration

### High Priority (Production Code):
EOF

# Find production files using deprecated functions
echo "Scanning production files..."
find crates -name "*.rs" ! -path "*/tests/*" ! -name "*test*.rs" \
    -exec grep -l "get_songbird_port\|get_beardog_port\|get_nestgate_port\|get_squirrel_port" {} \; \
    >> HARDCODING_MIGRATION_REPORT.md

echo >> HARDCODING_MIGRATION_REPORT.md
echo "### Medium Priority (Test Code):" >> HARDCODING_MIGRATION_REPORT.md

# Find test files
find crates -name "*.rs" -path "*/tests/*" \
    -exec grep -l "get_songbird_port\|get_beardog_port\|get_nestgate_port\|get_squirrel_port" {} \; \
    >> HARDCODING_MIGRATION_REPORT.md

echo
echo -e "${GREEN}✓ Migration report generated: HARDCODING_MIGRATION_REPORT.md${NC}"
echo

# Step 5: Show next actions
echo -e "${BLUE}Next Actions:${NC}"
echo "1. Review HARDCODING_MIGRATION_REPORT.md"
echo "2. Run: ./scripts/migrate-single-file.sh <file> to migrate individual files"
echo "3. Run: cargo test to verify migrations"
echo "4. Run: cargo clippy to check for deprecated warnings"
echo

echo -e "${GREEN}✓ Analysis complete!${NC}"

