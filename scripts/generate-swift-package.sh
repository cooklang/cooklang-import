#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script to generate/update Package.swift for binary distribution via Swift Package Manager
# This creates a Package.swift that references the XCFramework hosted on GitHub Releases

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
PACKAGE_NAME="CooklangImport"
REPO_OWNER="cooklang"
REPO_NAME="cooklang-import"
XCFRAMEWORK_ZIP="CooklangImportFFI.xcframework.zip"

usage() {
    echo "Usage: $0 <version> [--checksum <checksum>]"
    echo ""
    echo "Arguments:"
    echo "  version     The release version (e.g., v0.8.0 or 0.8.0)"
    echo ""
    echo "Options:"
    echo "  --checksum  Pre-computed checksum of the XCFramework zip"
    echo "              If not provided, will attempt to download and compute"
    echo ""
    echo "Examples:"
    echo "  $0 v0.8.0"
    echo "  $0 0.8.0 --checksum abc123..."
    exit 1
}

compute_checksum() {
    local version="$1"
    local url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/${XCFRAMEWORK_ZIP}"

    echo -e "${YELLOW}Downloading XCFramework to compute checksum...${NC}"

    local temp_file
    temp_file=$(mktemp)
    trap "rm -f $temp_file" EXIT

    if curl -fsSL "$url" -o "$temp_file"; then
        swift package compute-checksum "$temp_file"
    else
        echo -e "${RED}Error: Failed to download ${url}${NC}" >&2
        echo -e "${RED}Make sure the release exists and contains ${XCFRAMEWORK_ZIP}${NC}" >&2
        exit 1
    fi
}

compute_checksum_from_local() {
    local zip_path="$1"

    if [[ -f "$zip_path" ]]; then
        swift package compute-checksum "$zip_path"
    else
        echo -e "${RED}Error: File not found: ${zip_path}${NC}" >&2
        exit 1
    fi
}

generate_package_swift() {
    local version="$1"
    local checksum="$2"
    local url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/${XCFRAMEWORK_ZIP}"

    cat << EOF
// swift-tools-version:5.5
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "${PACKAGE_NAME}",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15)
    ],
    products: [
        .library(
            name: "${PACKAGE_NAME}",
            targets: ["${PACKAGE_NAME}", "${PACKAGE_NAME}FFI"]
        ),
    ],
    targets: [
        .target(
            name: "${PACKAGE_NAME}",
            dependencies: ["${PACKAGE_NAME}FFI"],
            path: "Sources/${PACKAGE_NAME}"
        ),
        .binaryTarget(
            name: "${PACKAGE_NAME}FFI",
            url: "${url}",
            checksum: "${checksum}"
        ),
    ]
)
EOF
}

main() {
    if [[ $# -lt 1 ]]; then
        usage
    fi

    local version="$1"
    shift

    # Ensure version has 'v' prefix
    if [[ ! "$version" =~ ^v ]]; then
        version="v${version}"
    fi

    local checksum=""
    local local_zip=""

    # Parse optional arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --checksum)
                checksum="$2"
                shift 2
                ;;
            --local)
                local_zip="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                usage
                ;;
        esac
    done

    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Swift Package Generator               ${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Version: ${version}"
    echo "Repository: ${REPO_OWNER}/${REPO_NAME}"
    echo ""

    # Compute checksum if not provided
    if [[ -z "$checksum" ]]; then
        if [[ -n "$local_zip" ]]; then
            echo -e "${YELLOW}Computing checksum from local file...${NC}"
            checksum=$(compute_checksum_from_local "$local_zip")
        else
            checksum=$(compute_checksum "$version")
        fi
    fi

    echo -e "${GREEN}Checksum: ${checksum}${NC}"
    echo ""

    # Generate Package.swift
    local package_swift_path="${PROJECT_ROOT}/Package.swift"
    echo -e "${YELLOW}Generating Package.swift...${NC}"
    generate_package_swift "$version" "$checksum" > "$package_swift_path"

    echo -e "${GREEN}Package.swift generated at: ${package_swift_path}${NC}"
    echo ""

    # Also create the Sources directory structure if it doesn't exist
    local sources_dir="${PROJECT_ROOT}/Sources/${PACKAGE_NAME}"
    if [[ ! -d "$sources_dir" ]]; then
        echo -e "${YELLOW}Creating Sources directory structure...${NC}"
        mkdir -p "$sources_dir"

        # Create a placeholder file that re-exports the FFI module
        cat > "${sources_dir}/CooklangImport.swift" << 'SWIFT_EOF'
// Re-export the FFI module for convenience
@_exported import CooklangImportFFI
SWIFT_EOF
        echo -e "${GREEN}Created placeholder Swift file${NC}"
    fi

    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Generation Complete!                  ${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "To use this package in your project, add:"
    echo ""
    echo "  .package(url: \"https://github.com/${REPO_OWNER}/${REPO_NAME}.git\", from: \"${version#v}\")"
    echo ""
    echo "Or in Xcode:"
    echo "  File > Add Package Dependencies..."
    echo "  URL: https://github.com/${REPO_OWNER}/${REPO_NAME}"
}

main "$@"
