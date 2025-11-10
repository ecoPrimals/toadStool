#!/bin/bash
# async_trait Migration Helper Script
# 
# This script performs automated steps of async_trait migration.
# Manual steps are still required after running this script.
#
# Usage: ./migrate_async_trait.sh <file_path>

set -e

FILE="$1"

if [ -z "$FILE" ]; then
    echo "❌ Error: No file specified"
    echo ""
    echo "Usage: $0 <file_path>"
    echo ""
    echo "Example:"
    echo "  $0 crates/core/common/src/infant_discovery/detectors.rs"
    exit 1
fi

if [ ! -f "$FILE" ]; then
    echo "❌ Error: File not found: $FILE"
    exit 1
fi

# Get package name from path
PACKAGE=$(echo "$FILE" | sed -n 's|crates/\([^/]*\)/.*|\1|p')
if [ -z "$PACKAGE" ]; then
    PACKAGE=$(echo "$FILE" | sed -n 's|crates/.*/\([^/]*\)/.*|toadstool-\1|p')
fi

echo "🔄 Migrating async_trait in: $FILE"
echo "📦 Package: $PACKAGE"
echo "---"
echo ""

# Count async_trait instances before
BEFORE_COUNT=$(grep -c "#\[async_trait\]" "$FILE" 2>/dev/null || echo "0")
echo "📊 Found $BEFORE_COUNT async_trait instance(s)"
echo ""

if [ "$BEFORE_COUNT" = "0" ]; then
    echo "✅ No async_trait instances found - file may already be migrated!"
    exit 0
fi

# Create backup
BACKUP="${FILE}.backup"
cp "$FILE" "$BACKUP"
echo "💾 Backup created: $BACKUP"
echo ""

# Step 1: Remove async_trait import
echo "Step 1: Removing async_trait import..."
sed -i '/^use async_trait::async_trait;$/d' "$FILE"

# Step 2: Add required imports if not present
if ! grep -q "use std::pin::Pin;" "$FILE"; then
    echo "Step 2: Adding Pin import..."
    # Find first use statement and add after it
    sed -i '0,/^use /{s|^\(use .*\)$|\1\nuse std::pin::Pin;|}' "$FILE"
fi

if ! grep -q "use std::future::Future;" "$FILE"; then
    echo "Step 3: Adding Future import..."
    sed -i '/^use std::pin::Pin;$/a use std::future::Future;' "$FILE"
fi

# Step 3: Remove #[async_trait] attributes
echo "Step 4: Removing #[async_trait] attributes..."
sed -i '/#\[async_trait\]$/d' "$FILE"

# Count remaining (should be 0)
AFTER_COUNT=$(grep -c "#\[async_trait\]" "$FILE" 2>/dev/null || echo "0")

echo ""
echo "---"
echo "✅ Automated steps complete!"
echo ""
echo "📊 Results:"
echo "  - Before: $BEFORE_COUNT instances"
echo "  - After:  $AFTER_COUNT instances"
echo "  - Removed: $((BEFORE_COUNT - AFTER_COUNT)) instances"
echo ""
echo "⚠️  MANUAL STEPS REQUIRED:"
echo ""
echo "1. Update trait method signatures:"
echo "   Change: async fn method(...) -> Result<T>"
echo "   To:     fn method(...) -> Pin<Box<dyn Future<Output = Result<T>> + Send + '_>>"
echo ""
echo "2. Update impl method signatures (same as above)"
echo ""
echo "3. Wrap async bodies in Box::pin(async move { ... })"
echo "   - Capture data from &self BEFORE async block"
echo "   - Convert &str to String if needed"
echo ""
echo "4. Example pattern:"
echo "   fn method(&self, param: &str) -> Pin<Box<...>> {"
echo "       let param = param.to_string();"
echo "       let field = self.field;"
echo "       Box::pin(async move {"
echo "           // your async code here"
echo "       })"
echo "   }"
echo ""
echo "📝 Next steps:"
echo ""
echo "1. Open file to complete manual migration:"
echo "   \$EDITOR $FILE"
echo ""
echo "2. Test compilation:"
echo "   cargo check --package $PACKAGE"
echo ""
echo "3. Run tests:"
echo "   cargo test --package $PACKAGE"
echo ""
echo "4. If errors occur, restore backup:"
echo "   mv $BACKUP $FILE"
echo ""
echo "5. When successful, remove backup:"
echo "   rm $BACKUP"
echo ""
echo "📚 Reference:"
echo "   See ASYNC_TRAIT_MIGRATION_KIT.md for patterns and examples"
echo ""

