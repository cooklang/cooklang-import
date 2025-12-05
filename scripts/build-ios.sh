#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building cooklang-import for iOS...${NC}"

# Configuration
PACKAGE_NAME="cooklang_import"
LIB_NAME="libcooklang_import.a"
FRAMEWORK_NAME="CooklangImportFFI"
OUTPUT_DIR="target/ios"
SWIFT_OUTPUT_DIR="${OUTPUT_DIR}/swift"
XCFRAMEWORK_OUTPUT="${OUTPUT_DIR}/${FRAMEWORK_NAME}.xcframework"

# iOS targets
IOS_TARGETS=(
    "aarch64-apple-ios"           # iOS devices (arm64)
    "aarch64-apple-ios-sim"       # iOS Simulator (arm64, Apple Silicon Macs)
    "x86_64-apple-ios"            # iOS Simulator (x86_64, Intel Macs)
)

# Check for required tools
check_requirements() {
    echo -e "${YELLOW}Checking requirements...${NC}"

    if ! command -v rustup &> /dev/null; then
        echo -e "${RED}Error: rustup is not installed${NC}"
        exit 1
    fi

    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: cargo is not installed${NC}"
        exit 1
    fi

    if ! command -v xcrun &> /dev/null; then
        echo -e "${RED}Error: Xcode command line tools are not installed${NC}"
        exit 1
    fi

    # Check if uniffi-bindgen is installed
    if ! cargo install --list | grep -q "uniffi-bindgen-cli"; then
        echo -e "${YELLOW}Installing uniffi-bindgen-cli...${NC}"
        cargo install uniffi-bindgen-cli --version 0.28
    fi

    echo -e "${GREEN}All requirements met!${NC}"
}

# Install iOS targets if needed
install_targets() {
    echo -e "${YELLOW}Installing iOS targets...${NC}"
    for target in "${IOS_TARGETS[@]}"; do
        if ! rustup target list --installed | grep -q "$target"; then
            echo "Installing target: $target"
            rustup target add "$target"
        fi
    done
    echo -e "${GREEN}All targets installed!${NC}"
}

# Build for all iOS targets
build_targets() {
    echo -e "${YELLOW}Building for iOS targets...${NC}"

    for target in "${IOS_TARGETS[@]}"; do
        echo "Building for $target..."
        cargo build --release --target "$target" --features uniffi
    done

    echo -e "${GREEN}All targets built!${NC}"
}

# Generate Swift bindings
generate_swift_bindings() {
    echo -e "${YELLOW}Generating Swift bindings...${NC}"

    mkdir -p "$SWIFT_OUTPUT_DIR"

    # Use the first available library to generate bindings
    local lib_path="target/${IOS_TARGETS[0]}/release/${LIB_NAME}"

    if [[ -f "$lib_path" ]]; then
        cargo run --features uniffi --bin uniffi-bindgen generate \
            --library "$lib_path" \
            --language swift \
            --out-dir "$SWIFT_OUTPUT_DIR" 2>/dev/null || \
        uniffi-bindgen generate \
            --library "$lib_path" \
            --language swift \
            --out-dir "$SWIFT_OUTPUT_DIR"
    else
        echo -e "${RED}Error: Library not found at $lib_path${NC}"
        exit 1
    fi

    echo -e "${GREEN}Swift bindings generated at ${SWIFT_OUTPUT_DIR}${NC}"
}

# Create XCFramework
create_xcframework() {
    echo -e "${YELLOW}Creating XCFramework...${NC}"

    # Clean previous xcframework
    rm -rf "$XCFRAMEWORK_OUTPUT"

    # Create temporary directories for each platform
    local ios_device_dir="${OUTPUT_DIR}/ios-device"
    local ios_sim_dir="${OUTPUT_DIR}/ios-simulator"

    mkdir -p "$ios_device_dir" "$ios_sim_dir"

    # Copy device library
    cp "target/aarch64-apple-ios/release/${LIB_NAME}" "$ios_device_dir/"

    # Create fat library for simulator (arm64 + x86_64)
    echo "Creating fat library for iOS Simulator..."
    lipo -create \
        "target/aarch64-apple-ios-sim/release/${LIB_NAME}" \
        "target/x86_64-apple-ios/release/${LIB_NAME}" \
        -output "$ios_sim_dir/${LIB_NAME}"

    # Create module.modulemap
    local modulemap_content="module ${FRAMEWORK_NAME} {
    header \"${PACKAGE_NAME}FFI.h\"
    export *
}"

    # Create headers and modulemap for device
    mkdir -p "$ios_device_dir/Headers"
    cp "${SWIFT_OUTPUT_DIR}/${PACKAGE_NAME}FFI.h" "$ios_device_dir/Headers/"
    mkdir -p "$ios_device_dir/Modules"
    echo "$modulemap_content" > "$ios_device_dir/Modules/module.modulemap"

    # Create headers and modulemap for simulator
    mkdir -p "$ios_sim_dir/Headers"
    cp "${SWIFT_OUTPUT_DIR}/${PACKAGE_NAME}FFI.h" "$ios_sim_dir/Headers/"
    mkdir -p "$ios_sim_dir/Modules"
    echo "$modulemap_content" > "$ios_sim_dir/Modules/module.modulemap"

    # Create XCFramework
    xcodebuild -create-xcframework \
        -library "$ios_device_dir/${LIB_NAME}" \
        -headers "$ios_device_dir/Headers" \
        -library "$ios_sim_dir/${LIB_NAME}" \
        -headers "$ios_sim_dir/Headers" \
        -output "$XCFRAMEWORK_OUTPUT"

    # Copy Swift file alongside XCFramework
    cp "${SWIFT_OUTPUT_DIR}/${PACKAGE_NAME}.swift" "${OUTPUT_DIR}/"

    # Cleanup temp directories
    rm -rf "$ios_device_dir" "$ios_sim_dir"

    echo -e "${GREEN}XCFramework created at ${XCFRAMEWORK_OUTPUT}${NC}"
}

# Create Swift Package
create_swift_package() {
    echo -e "${YELLOW}Creating Swift Package structure...${NC}"

    local swift_pkg_dir="${OUTPUT_DIR}/CooklangImport"
    mkdir -p "${swift_pkg_dir}/Sources/CooklangImport"

    # Copy Swift bindings
    cp "${SWIFT_OUTPUT_DIR}/${PACKAGE_NAME}.swift" "${swift_pkg_dir}/Sources/CooklangImport/"

    # Create Package.swift
    cat > "${swift_pkg_dir}/Package.swift" << 'EOF'
// swift-tools-version:5.5
import PackageDescription

let package = Package(
    name: "CooklangImport",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15)
    ],
    products: [
        .library(
            name: "CooklangImport",
            targets: ["CooklangImport", "CooklangImportFFI"]
        ),
    ],
    targets: [
        .target(
            name: "CooklangImport",
            dependencies: ["CooklangImportFFI"]
        ),
        .binaryTarget(
            name: "CooklangImportFFI",
            path: "CooklangImportFFI.xcframework"
        ),
    ]
)
EOF

    # Copy XCFramework into the package
    cp -R "$XCFRAMEWORK_OUTPUT" "${swift_pkg_dir}/"

    echo -e "${GREEN}Swift Package created at ${swift_pkg_dir}${NC}"
}

# Main execution
main() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  cooklang-import iOS Build Script      ${NC}"
    echo -e "${GREEN}========================================${NC}"

    check_requirements
    install_targets
    build_targets
    generate_swift_bindings
    create_xcframework
    create_swift_package

    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Build Complete!                       ${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Output files:"
    echo "  - XCFramework: ${XCFRAMEWORK_OUTPUT}"
    echo "  - Swift bindings: ${OUTPUT_DIR}/${PACKAGE_NAME}.swift"
    echo "  - Swift Package: ${OUTPUT_DIR}/CooklangImport/"
    echo ""
    echo "To use in your iOS project:"
    echo "  1. Add the XCFramework to your Xcode project"
    echo "  2. Add the Swift bindings file to your project"
    echo ""
    echo "Or use the Swift Package:"
    echo "  1. Copy ${OUTPUT_DIR}/CooklangImport to your project"
    echo "  2. Add it as a local Swift Package dependency"
}

main "$@"
