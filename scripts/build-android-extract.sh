#!/bin/bash
set -euo pipefail

# Builds the extract-only Android SDK: `extract_recipe_from_html` and nothing that
# needs the network, an LLM or an async runtime. For an app that already holds the
# page's HTML (an in-app browser, a share target) and should not carry the HTTP and
# TLS stack of the full SDK.
#
# Output, in target/android-extract:
#   cooklang-import-extract-android/       an Android library module:
#     src/main/kotlin/org/cooklang/extract/  the generated Kotlin bindings
#     src/main/jniLibs/<abi>/                libcooklang_import.so per ABI
#     src/main/AndroidManifest.xml           no permissions
#     consumer-rules.pro
#   cooklang-import-extract-android.zip    the release asset, and what gets published
#                                          as org.cooklang:cooklang-import-extract
#                                          (scripts/publish-android-module.sh)
#
# Needs cargo-ndk and ANDROID_NDK_HOME. The full SDK is built by
# scripts/build-android.sh and by the release workflow.

LIB_NAME="libcooklang_import.so"
KOTLIN_PACKAGE_DIR="org/cooklang/extract"
OUTPUT_DIR="target/android-extract"
MODULE_NAME="cooklang-import-extract-android"
MODULE_DIR="${OUTPUT_DIR}/${MODULE_NAME}"
KOTLIN_DIR="${MODULE_DIR}/src/main/kotlin"
JNI_LIBS_DIR="${MODULE_DIR}/src/main/jniLibs"
MIN_API_LEVEL="${ANDROID_MIN_API_LEVEL:-21}"

# The full SDK and this build both produce target/<triple>/release/libcooklang_import.so.
# This one gets a target directory of its own, so that neither build overwrites the
# other's libraries, whichever runs first. It is not inside OUTPUT_DIR, which is
# removed below.
BUILD_DIR="target/extract-android-build"

# Every cargo command of this build takes these. Without them cargo builds the full
# library, and nothing fails until the app is three times the size it should be.
EXTRACT_FEATURES=(--no-default-features --features uniffi)

# Rust target and the Android ABI it is packaged as.
TARGETS=(
    "aarch64-linux-android:arm64-v8a"
    "armv7-linux-androideabi:armeabi-v7a"
    "x86_64-linux-android:x86_64"
)

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
    echo "error: ANDROID_NDK_HOME is not set" >&2
    exit 1
fi
NDK_BIN=$(echo "${ANDROID_NDK_HOME}"/toolchains/llvm/prebuilt/*/bin)
if [[ ! -x "${NDK_BIN}/llvm-strip" ]]; then
    echo "error: no llvm-strip under ${ANDROID_NDK_HOME}" >&2
    exit 1
fi

for entry in "${TARGETS[@]}"; do
    target="${entry%%:*}"
    if ! rustup target list --installed | grep -qx "$target"; then
        rustup target add "$target"
    fi
    echo "Building the extract-only library for $target..."
    cargo ndk --target "$target" --platform "$MIN_API_LEVEL" \
        build --release "${EXTRACT_FEATURES[@]}" --target-dir "$BUILD_DIR"
done

rm -rf "$OUTPUT_DIR"
mkdir -p "$KOTLIN_DIR" "$JNI_LIBS_DIR"

# From the library as cargo left it: uniffi-bindgen reads its metadata from symbols,
# and generates nothing, without an error, from a stripped library. The bindgen binary
# itself is a host tool with the default features; it only reads the library.
echo "Generating Kotlin bindings..."
cargo run --release --features uniffi-cli --bin uniffi-bindgen -- generate \
    --config uniffi-extract.toml \
    --library "${BUILD_DIR}/aarch64-linux-android/release/${LIB_NAME}" \
    --language kotlin \
    --out-dir "$KOTLIN_DIR"

# The bindings land in the package's directory, which a Gradle module needs kept.
BINDINGS="${KOTLIN_DIR}/${KOTLIN_PACKAGE_DIR}/cooklang_import.kt"
if [[ ! -f "$BINDINGS" ]]; then
    echo "error: no Kotlin bindings at ${BINDINGS} (stripped library, or uniffi-extract.toml not applied?)" >&2
    exit 1
fi
if ! grep -q 'fun `extractRecipeFromHtml`' "$BINDINGS"; then
    echo "error: ${BINDINGS} does not declare extractRecipeFromHtml" >&2
    exit 1
fi
# Either of these means a library with the default features was built.
if grep -q -e 'importFromUrl' -e 'kotlinx.coroutines' "$BINDINGS"; then
    echo "error: ${BINDINGS} has the full SDK's API; the extract-only features were not applied" >&2
    exit 1
fi

for entry in "${TARGETS[@]}"; do
    target="${entry%%:*}"
    abi="${entry##*:}"
    lib="${JNI_LIBS_DIR}/${abi}/${LIB_NAME}"
    mkdir -p "${JNI_LIBS_DIR}/${abi}"
    cp "${BUILD_DIR}/${target}/release/${LIB_NAME}" "$lib"

    # The symbol table is a third of the file and nothing JNA looks up: it resolves
    # the exported functions through the dynamic symbols, which stay.
    "${NDK_BIN}/llvm-strip" --strip-unneeded "$lib"

    # Android 15+ devices with 16 KB pages refuse a 64-bit library whose segments
    # are aligned to less. The toolchain links with 16 KB as it is (seen with NDK
    # 25.2 and 27.0); this fails the build if a change of toolchain takes that away.
    if [[ "$abi" != "armeabi-v7a" ]]; then
        if "${NDK_BIN}/llvm-readelf" -lW "$lib" | awk '$1 == "LOAD" && $NF != "0x4000" { bad = 1 } END { exit !bad }'; then
            echo "error: ${lib} is not aligned to 16 KB pages" >&2
            exit 1
        fi
    fi
done

# No INTERNET permission: this build has no code that could use it.
cat > "${MODULE_DIR}/src/main/AndroidManifest.xml" << 'XML'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android" />
XML

# JNA finds the generated classes and its own by name, through reflection.
cat > "${MODULE_DIR}/consumer-rules.pro" << 'PRO'
-keep class org.cooklang.extract.** { *; }
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
PRO

(cd "$OUTPUT_DIR" && zip -qr "${MODULE_NAME}.zip" "$MODULE_NAME")

echo ""
echo "Done:"
echo "  ${MODULE_DIR}"
echo "  ${OUTPUT_DIR}/${MODULE_NAME}.zip"
for entry in "${TARGETS[@]}"; do
    abi="${entry##*:}"
    echo "  ${abi}: $(wc -c < "${JNI_LIBS_DIR}/${abi}/${LIB_NAME}" | tr -d ' ') bytes"
done
