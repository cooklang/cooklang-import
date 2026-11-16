#!/bin/bash
set -euo pipefail

# Builds the extract-only Apple SDK: `extract_recipe_from_html` and nothing that
# needs the network, an LLM or an async runtime. For an app that already holds the
# page's HTML (an in-app browser, a share extension) and should not carry the HTTP
# and TLS stack of the full SDK.
#
# Output, in target/ios-extract:
#   CooklangImportExtractFFI.xcframework       iOS, iOS Simulator and Mac Catalyst
#   CooklangImportExtractFFI.xcframework.zip   the release asset Package.swift points at
#   swift/CooklangImportExtract.swift          the generated Swift bindings
#
# The full SDK is built by scripts/build-ios.sh.

LIB_NAME="libcooklang_import.a"
FRAMEWORK_NAME="CooklangImportExtractFFI"
SWIFT_MODULE="CooklangImportExtract"
OUTPUT_DIR="target/ios-extract"
SWIFT_OUTPUT_DIR="${OUTPUT_DIR}/swift"
XCFRAMEWORK_OUTPUT="${OUTPUT_DIR}/${FRAMEWORK_NAME}.xcframework"

# Mac Catalyst is here, and not in the full SDK, because this build is pure Rust and
# cross-compiles to it without further setup. It lets an app's package tests, which
# run as Mac Catalyst on a Mac, exercise the real extractors.
TARGETS=(
    "aarch64-apple-ios"
    "aarch64-apple-ios-sim"
    "aarch64-apple-ios-macabi"
    "x86_64-apple-ios-macabi"
)

for target in "${TARGETS[@]}"; do
    if ! rustup target list --installed | grep -qx "$target"; then
        rustup target add "$target"
    fi
    echo "Building the extract-only library for $target..."
    cargo build --release --target "$target" --no-default-features --features uniffi
done

rm -rf "$OUTPUT_DIR"
mkdir -p "$SWIFT_OUTPUT_DIR"

echo "Generating Swift bindings..."
cargo run --features uniffi-cli --bin uniffi-bindgen -- generate \
    --config uniffi-extract.toml \
    --library "target/aarch64-apple-ios/release/${LIB_NAME}" \
    --language swift \
    --out-dir "$SWIFT_OUTPUT_DIR"

# One folder per platform: the library, and the header with its module map.
#
# The header and module map go in a folder named after the module, not straight
# into Headers/. Xcode copies every static-library xcframework's Headers/ into one
# shared include folder, so two xcframeworks that each ship Headers/module.modulemap
# cannot be built into the same app ("Multiple commands produce .../include/
# module.modulemap"). Clang still finds <include>/<Module>/module.modulemap.
slice() {
    local dir="${OUTPUT_DIR}/$1"
    local headers="${dir}/Headers/${FRAMEWORK_NAME}"
    mkdir -p "$headers"
    cp "${SWIFT_OUTPUT_DIR}/${FRAMEWORK_NAME}.h" "${headers}/"
    printf 'module %s {\n    header "%s.h"\n    export *\n}\n' "$FRAMEWORK_NAME" "$FRAMEWORK_NAME" \
        > "${headers}/module.modulemap"
}

slice ios-device
cp "target/aarch64-apple-ios/release/${LIB_NAME}" "${OUTPUT_DIR}/ios-device/"

slice ios-simulator
cp "target/aarch64-apple-ios-sim/release/${LIB_NAME}" "${OUTPUT_DIR}/ios-simulator/"

slice mac-catalyst
lipo -create \
    "target/aarch64-apple-ios-macabi/release/${LIB_NAME}" \
    "target/x86_64-apple-ios-macabi/release/${LIB_NAME}" \
    -output "${OUTPUT_DIR}/mac-catalyst/${LIB_NAME}"

# Debug symbols make up a quarter of each archive and none of what an app links.
for platform in ios-device ios-simulator mac-catalyst; do
    xcrun strip -S "${OUTPUT_DIR}/${platform}/${LIB_NAME}"
done

echo "Creating the XCFramework..."
xcodebuild -create-xcframework \
    -library "${OUTPUT_DIR}/ios-device/${LIB_NAME}" -headers "${OUTPUT_DIR}/ios-device/Headers" \
    -library "${OUTPUT_DIR}/ios-simulator/${LIB_NAME}" -headers "${OUTPUT_DIR}/ios-simulator/Headers" \
    -library "${OUTPUT_DIR}/mac-catalyst/${LIB_NAME}" -headers "${OUTPUT_DIR}/mac-catalyst/Headers" \
    -output "$XCFRAMEWORK_OUTPUT"

rm -rf "${OUTPUT_DIR}/ios-device" "${OUTPUT_DIR}/ios-simulator" "${OUTPUT_DIR}/mac-catalyst"

(cd "$OUTPUT_DIR" && zip -qr "${FRAMEWORK_NAME}.xcframework.zip" "${FRAMEWORK_NAME}.xcframework")

echo ""
echo "Done:"
echo "  ${XCFRAMEWORK_OUTPUT}"
echo "  ${XCFRAMEWORK_OUTPUT}.zip"
echo "  ${SWIFT_OUTPUT_DIR}/${SWIFT_MODULE}.swift"
