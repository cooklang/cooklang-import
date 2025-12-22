#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Configuration
REPO="cooklang/cooklang-import"
TEST_DIR=$(mktemp -d)
TEST_URL="https://www.bbcgoodfood.com/recipes/easy-pancakes"
ORIGINAL_PWD="$(pwd)"

cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  iOS Release Package Test             ${NC}"
echo -e "${GREEN}========================================${NC}"

# Get version to test (default: latest)
VERSION="${1:-latest}"

if [ "$VERSION" = "latest" ]; then
    echo -e "${YELLOW}Fetching latest release...${NC}"
    VERSION=$(gh release view --repo "$REPO" --json tagName -q '.tagName')
fi

echo -e "${GREEN}Testing version: ${VERSION}${NC}"

# Download release assets
echo -e "${YELLOW}Downloading release assets...${NC}"
cd "$TEST_DIR"

gh release download "$VERSION" \
    --repo "$REPO" \
    --pattern "CooklangImportFFI.xcframework.zip" \
    --pattern "CooklangImport-ios.zip"

# Extract
echo -e "${YELLOW}Extracting packages...${NC}"
unzip -oq CooklangImportFFI.xcframework.zip
unzip -oq CooklangImport-ios.zip

echo -e "${YELLOW}Package contents:${NC}"
ls -la
echo ""
ls -la CooklangImportFFI.xcframework/

# Verify XCFramework structure
echo -e "${YELLOW}Verifying XCFramework structure...${NC}"

check_exists() {
    if [ -e "$1" ]; then
        echo -e "  ${GREEN}✓${NC} $1"
        return 0
    else
        echo -e "  ${RED}✗${NC} $1 missing"
        return 1
    fi
}

ERRORS=0

# Check device slice
check_exists "CooklangImportFFI.xcframework/ios-arm64" || ((ERRORS++))
check_exists "CooklangImportFFI.xcframework/ios-arm64/libcooklang_import.a" || ((ERRORS++))
check_exists "CooklangImportFFI.xcframework/ios-arm64/Headers/CooklangImportFFI.h" || ((ERRORS++))

# Check simulator slice
if [ -d "CooklangImportFFI.xcframework/ios-arm64-simulator" ]; then
    check_exists "CooklangImportFFI.xcframework/ios-arm64-simulator/libcooklang_import.a" || ((ERRORS++))
    check_exists "CooklangImportFFI.xcframework/ios-arm64-simulator/Headers/CooklangImportFFI.h" || ((ERRORS++))
elif [ -d "CooklangImportFFI.xcframework/ios-arm64_x86_64-simulator" ]; then
    check_exists "CooklangImportFFI.xcframework/ios-arm64_x86_64-simulator/libcooklang_import.a" || ((ERRORS++))
    check_exists "CooklangImportFFI.xcframework/ios-arm64_x86_64-simulator/Headers/CooklangImportFFI.h" || ((ERRORS++))
else
    echo -e "  ${RED}✗${NC} No simulator slice found"
    ((ERRORS++))
fi

# Check Info.plist
check_exists "CooklangImportFFI.xcframework/Info.plist" || ((ERRORS++))

# Check Swift bindings (can be in different locations)
SWIFT_BINDINGS=""
for path in "CooklangImport.swift" "swift/CooklangImport.swift" "CooklangImport/Sources/CooklangImport/CooklangImport.swift"; do
    if [ -f "$path" ]; then
        SWIFT_BINDINGS="$path"
        break
    fi
done

if [ -n "$SWIFT_BINDINGS" ]; then
    echo -e "  ${GREEN}✓${NC} Swift bindings: $SWIFT_BINDINGS"
else
    echo -e "  ${RED}✗${NC} Swift bindings not found"
    ((ERRORS++))
fi

# Verify library architectures
echo -e "${YELLOW}Verifying library architectures...${NC}"

DEVICE_LIB="CooklangImportFFI.xcframework/ios-arm64/libcooklang_import.a"
if [ -f "$DEVICE_LIB" ]; then
    ARCHS=$(lipo -info "$DEVICE_LIB" 2>/dev/null || echo "unknown")
    echo -e "  Device lib: $ARCHS"
    if [[ "$ARCHS" == *"arm64"* ]]; then
        echo -e "  ${GREEN}✓${NC} arm64 architecture present"
    else
        echo -e "  ${RED}✗${NC} arm64 architecture missing"
        ((ERRORS++))
    fi
fi

# Check library size
echo -e "${YELLOW}Checking library sizes...${NC}"
DEVICE_SIZE=$(stat -f%z "$DEVICE_LIB" 2>/dev/null || stat -c%s "$DEVICE_LIB" 2>/dev/null)
DEVICE_SIZE_MB=$(echo "scale=2; $DEVICE_SIZE / 1024 / 1024" | bc)
echo -e "  Device library: ${DEVICE_SIZE_MB} MB"

if (( $(echo "$DEVICE_SIZE_MB > 50" | bc -l) )); then
    echo -e "  ${RED}✗${NC} Library seems too large (>50MB)"
    ((ERRORS++))
else
    echo -e "  ${GREEN}✓${NC} Library size OK"
fi

# Check Swift bindings content
echo -e "${YELLOW}Verifying Swift bindings...${NC}"

check_swift_symbol() {
    if [ -n "$SWIFT_BINDINGS" ] && grep -q "$1" "$SWIFT_BINDINGS" 2>/dev/null; then
        echo -e "  ${GREEN}✓${NC} $1"
        return 0
    else
        echo -e "  ${RED}✗${NC} $1 not found"
        return 1
    fi
}

check_swift_symbol "func extractRecipeFromUrl" || ((ERRORS++))
check_swift_symbol "func simpleImport" || ((ERRORS++))
check_swift_symbol "func getVersion" || ((ERRORS++))
check_swift_symbol "struct FfiRecipeComponents" || ((ERRORS++))
check_swift_symbol "struct FfiImportConfig" || ((ERRORS++))
check_swift_symbol "extractOnly" || ((ERRORS++))

# Verify the bundled Swift Package structure
echo -e "${YELLOW}Verifying Swift Package structure...${NC}"

if [ -d "CooklangImport" ]; then
    check_exists "CooklangImport/Package.swift" || ((ERRORS++))
    check_exists "CooklangImport/Sources/CooklangImport/CooklangImport.swift" || ((ERRORS++))
    check_exists "CooklangImport/CooklangImportFFI.xcframework" || ((ERRORS++))

    # Verify Package.swift is valid
    if grep -q "CooklangImportFFI" CooklangImport/Package.swift 2>/dev/null; then
        echo -e "  ${GREEN}✓${NC} Package.swift references CooklangImportFFI"
    else
        echo -e "  ${RED}✗${NC} Package.swift missing CooklangImportFFI reference"
        ((ERRORS++))
    fi
else
    echo -e "  ${YELLOW}⚠${NC} CooklangImport Swift Package not found in archive"
fi

# Runtime test using CLI (same code path as Swift SDK)
echo -e "${YELLOW}Testing extract functionality (via CLI)...${NC}"

# Find CLI - check common locations
CLI_CMD=""

# Check if we have a pre-built CLI in the original working directory
ORIGINAL_DIR="${ORIGINAL_PWD:-$(pwd)}"
if [ -f "$ORIGINAL_DIR/target/release/cooklang-import" ]; then
    CLI_CMD="$ORIGINAL_DIR/target/release/cooklang-import"
elif command -v cooklang-import &>/dev/null; then
    CLI_CMD="cooklang-import"
elif [ -f "/Users/alexeydubovskoy/Cooklang/cooklang-import/target/release/cooklang-import" ]; then
    # Fallback to known location
    CLI_CMD="/Users/alexeydubovskoy/Cooklang/cooklang-import/target/release/cooklang-import"
fi

if [ -n "$CLI_CMD" ]; then
    echo -e "  Testing extractRecipeFromUrl with: $TEST_URL"

    EXTRACT_OUTPUT=$("$CLI_CMD" "$TEST_URL" --extract-only 2>&1) || true

    # Check for expected fields in output
    if echo "$EXTRACT_OUTPUT" | grep -q "title:"; then
        echo -e "  ${GREEN}✓${NC} title field extracted"
    else
        echo -e "  ${RED}✗${NC} title field missing"
        ((ERRORS++))
    fi

    if echo "$EXTRACT_OUTPUT" | grep -q "flour\|eggs\|milk"; then
        echo -e "  ${GREEN}✓${NC} ingredients extracted"
    else
        echo -e "  ${RED}✗${NC} ingredients missing"
        ((ERRORS++))
    fi

    if echo "$EXTRACT_OUTPUT" | grep -q "whisk\|batter\|pan"; then
        echo -e "  ${GREEN}✓${NC} instructions extracted"
    else
        echo -e "  ${RED}✗${NC} instructions missing"
        ((ERRORS++))
    fi

    echo -e "  ${GREEN}✓${NC} Extract functionality works (same code path as Swift SDK)"
else
    echo -e "  ${YELLOW}⚠${NC} CLI not available, skipping runtime test"
fi

# Note about iOS-only testing
echo -e "${YELLOW}Note:${NC} XCFramework is iOS-only. Full Swift integration tests require Xcode."

# Summary
echo ""
echo -e "${GREEN}========================================${NC}"
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}  All checks passed! ✓                 ${NC}"
else
    echo -e "${RED}  $ERRORS check(s) failed ✗             ${NC}"
fi
echo -e "${GREEN}========================================${NC}"

exit $ERRORS
