#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Generates Package.swift for binary distribution via Swift Package Manager: the two
# products, each pointing at its XCFramework on GitHub Releases. The release workflow
# writes the same file (.github/workflows/release.yml, "Update Package.swift"); this
# is the manual way, for repairing or re-pointing it by hand. Keep the two in step.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
REPO_OWNER="cooklang"
REPO_NAME="cooklang-import"
# The full SDK, and the extract-only one (extractRecipeFromHtml, no network stack).
FULL_MODULE="CooklangImport"
FULL_ZIP="CooklangImportFFI.xcframework.zip"
FULL_BINDINGS_DIRS=("target/ios/swift" "target/bindings/swift")
EXTRACT_MODULE="CooklangImportExtract"
EXTRACT_ZIP="CooklangImportExtractFFI.xcframework.zip"
EXTRACT_BINDINGS_DIRS=("target/ios-extract/swift")

usage() {
    echo "Usage: $0 <version> [options]"
    echo ""
    echo "Arguments:"
    echo "  version              The release version (e.g., v0.13.1 or 0.13.1)"
    echo ""
    echo "Options, for the full SDK (${FULL_ZIP}):"
    echo "  --checksum <sum>     Pre-computed checksum of the zip"
    echo "  --local <zip>        Compute the checksum from a local zip"
    echo ""
    echo "Options, for the extract-only SDK (${EXTRACT_ZIP}):"
    echo "  --extract-checksum <sum>"
    echo "  --extract-local <zip>"
    echo ""
    echo "A checksum that is not given either way is computed from the release's asset,"
    echo "which is downloaded for that."
    echo ""
    echo "Examples:"
    echo "  $0 v0.13.1"
    echo "  $0 0.13.1 --checksum abc123... --extract-checksum def456..."
    echo "  $0 0.13.1 --local CooklangImportFFI.xcframework.zip \\"
    echo "            --extract-local target/ios-extract/CooklangImportExtractFFI.xcframework.zip"
    exit 1
}

release_url() {
    echo "https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/$1/$2"
}

# Prints the checksum of one zip: the given one, else from the local file, else from
# the release asset. Progress goes to stderr so that stdout is the checksum alone.
checksum_for() {
    local version="$1" zip_name="$2" given="$3" local_zip="$4"

    if [[ -n "$given" ]]; then
        echo "$given"
        return
    fi

    if [[ -n "$local_zip" ]]; then
        if [[ ! -f "$local_zip" ]]; then
            echo -e "${RED}Error: File not found: ${local_zip}${NC}" >&2
            exit 1
        fi
        echo -e "${YELLOW}Computing the checksum of ${local_zip}...${NC}" >&2
        swift package compute-checksum "$local_zip"
        return
    fi

    # SwiftPM tells the archive type by the file's extension, so the download keeps
    # its name inside a temporary folder.
    local url temp_dir checksum
    url=$(release_url "$version" "$zip_name")
    echo -e "${YELLOW}Downloading ${zip_name} to compute its checksum...${NC}" >&2
    temp_dir=$(mktemp -d)
    if ! curl -fsSL "$url" -o "${temp_dir}/${zip_name}"; then
        rm -rf "$temp_dir"
        echo -e "${RED}Error: Failed to download ${url}${NC}" >&2
        echo -e "${RED}Make sure the release exists and contains ${zip_name}${NC}" >&2
        exit 1
    fi
    if ! checksum=$(swift package compute-checksum "${temp_dir}/${zip_name}"); then
        rm -rf "$temp_dir"
        echo -e "${RED}Error: Could not compute the checksum of ${zip_name}${NC}" >&2
        exit 1
    fi
    rm -rf "$temp_dir"
    echo "$checksum"
}

generate_package_swift() {
    local version="$1" full_checksum="$2" extract_checksum="$3"

    cat << EOF
// swift-tools-version:5.5
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "CooklangImport",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15)
    ],
    products: [
        .library(
            name: "${FULL_MODULE}",
            targets: ["${FULL_MODULE}", "${FULL_MODULE}FFI"]
        ),
        // extractRecipeFromHtml only: no network, no LLM. Link this or
        // CooklangImport, never both.
        .library(
            name: "${EXTRACT_MODULE}",
            targets: ["${EXTRACT_MODULE}", "${EXTRACT_MODULE}FFI"]
        ),
    ],
    targets: [
        .target(
            name: "${FULL_MODULE}",
            dependencies: ["${FULL_MODULE}FFI"],
            path: "Sources/${FULL_MODULE}"
        ),
        .binaryTarget(
            name: "${FULL_MODULE}FFI",
            url: "$(release_url "$version" "$FULL_ZIP")",
            checksum: "${full_checksum}"
        ),
        .target(
            name: "${EXTRACT_MODULE}",
            dependencies: ["${EXTRACT_MODULE}FFI"],
            path: "Sources/${EXTRACT_MODULE}"
        ),
        .binaryTarget(
            name: "${EXTRACT_MODULE}FFI",
            url: "$(release_url "$version" "$EXTRACT_ZIP")",
            checksum: "${extract_checksum}"
        ),
    ]
)
EOF
}

# Copies one module's generated Swift bindings into Sources/. Bindings already there
# are kept when no freshly generated ones are found: they are checked in, and a
# placeholder must never replace them.
copy_bindings() {
    local module="$1" build_hint="$2"
    shift 2
    local sources_dir="${PROJECT_ROOT}/Sources/${module}"
    local target_file="${sources_dir}/${module}.swift"

    local dir
    for dir in "$@"; do
        local candidate="${PROJECT_ROOT}/${dir}/${module}.swift"
        if [[ -f "$candidate" ]]; then
            mkdir -p "$sources_dir"
            cp "$candidate" "$target_file"
            echo -e "${GREEN}Copied ${module}.swift from ${dir}${NC}"
            return
        fi
    done

    if [[ -f "$target_file" ]]; then
        echo -e "${YELLOW}No freshly generated ${module}.swift; keeping the one in Sources/${module}${NC}"
        echo -e "${YELLOW}Run '${build_hint}' first if the bindings changed${NC}"
        return
    fi

    echo -e "${RED}Warning: ${module}.swift not found, and Sources/${module} has none${NC}"
    echo -e "${RED}Run '${build_hint}' to generate it; writing a placeholder for now${NC}"
    mkdir -p "$sources_dir"
    cat > "$target_file" << SWIFT_EOF
// Re-export the FFI module for convenience
// NOTE: This is a placeholder. For full functionality, run the iOS build
// to generate proper Swift bindings.
@_exported import ${module}FFI
SWIFT_EOF
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

    local full_checksum="" full_local="" extract_checksum="" extract_local=""

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --checksum|--local|--extract-checksum|--extract-local)
                if [[ $# -lt 2 ]]; then
                    echo -e "${RED}$1 needs a value${NC}"
                    usage
                fi
                case "$1" in
                    --checksum) full_checksum="$2" ;;
                    --local) full_local="$2" ;;
                    --extract-checksum) extract_checksum="$2" ;;
                    --extract-local) extract_local="$2" ;;
                esac
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

    full_checksum=$(checksum_for "$version" "$FULL_ZIP" "$full_checksum" "$full_local")
    extract_checksum=$(checksum_for "$version" "$EXTRACT_ZIP" "$extract_checksum" "$extract_local")

    echo -e "${GREEN}${FULL_ZIP}: ${full_checksum}${NC}"
    echo -e "${GREEN}${EXTRACT_ZIP}: ${extract_checksum}${NC}"
    echo ""

    local package_swift_path="${PROJECT_ROOT}/Package.swift"
    echo -e "${YELLOW}Generating Package.swift...${NC}"
    generate_package_swift "$version" "$full_checksum" "$extract_checksum" > "$package_swift_path"
    echo -e "${GREEN}Package.swift generated at: ${package_swift_path}${NC}"
    echo ""

    copy_bindings "$FULL_MODULE" "make ios" "${FULL_BINDINGS_DIRS[@]}"
    copy_bindings "$EXTRACT_MODULE" "make ios-extract" "${EXTRACT_BINDINGS_DIRS[@]}"

    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Generation Complete!                  ${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "To use this package in your project, add:"
    echo ""
    echo "  .package(url: \"https://github.com/${REPO_OWNER}/${REPO_NAME}.git\", from: \"${version#v}\")"
    echo ""
    echo "and one of the products, never both:"
    echo ""
    echo "  .product(name: \"${FULL_MODULE}\", package: \"${REPO_NAME}\")         // full SDK"
    echo "  .product(name: \"${EXTRACT_MODULE}\", package: \"${REPO_NAME}\")  // HTML extraction only"
}

main "$@"
