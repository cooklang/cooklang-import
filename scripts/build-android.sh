#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building cooklang-import for Android...${NC}"

# Configuration
PACKAGE_NAME="cooklang_import"
LIB_NAME="libcooklang_import.so"
OUTPUT_DIR="target/android"
KOTLIN_OUTPUT_DIR="${OUTPUT_DIR}/kotlin"
JNI_LIBS_DIR="${OUTPUT_DIR}/jniLibs"

# Android targets and their ABI mappings
declare -A ANDROID_TARGETS=(
    ["aarch64-linux-android"]="arm64-v8a"
    ["armv7-linux-androideabi"]="armeabi-v7a"
    ["x86_64-linux-android"]="x86_64"
    ["i686-linux-android"]="x86"
)

# Minimum API level
MIN_API_LEVEL="${ANDROID_MIN_API_LEVEL:-21}"

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

    # Check for NDK
    if [[ -z "${ANDROID_NDK_HOME:-}" ]] && [[ -z "${NDK_HOME:-}" ]]; then
        # Try to find NDK in common locations
        local ndk_locations=(
            "$HOME/Android/Sdk/ndk"
            "$HOME/Library/Android/sdk/ndk"
            "/usr/local/android-sdk/ndk"
        )

        for loc in "${ndk_locations[@]}"; do
            if [[ -d "$loc" ]]; then
                # Get the latest NDK version
                local latest_ndk
                latest_ndk=$(ls -1 "$loc" 2>/dev/null | sort -V | tail -n1)
                if [[ -n "$latest_ndk" ]]; then
                    export ANDROID_NDK_HOME="$loc/$latest_ndk"
                    echo -e "${YELLOW}Found NDK at: ${ANDROID_NDK_HOME}${NC}"
                    break
                fi
            fi
        done

        if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
            echo -e "${RED}Error: Android NDK not found${NC}"
            echo "Please set ANDROID_NDK_HOME or NDK_HOME environment variable"
            echo "Or install the NDK via Android Studio"
            exit 1
        fi
    fi

    NDK_HOME="${ANDROID_NDK_HOME:-$NDK_HOME}"
    export NDK_HOME

    # Check if uniffi-bindgen is installed
    if ! cargo install --list | grep -q "uniffi-bindgen-cli"; then
        echo -e "${YELLOW}Installing uniffi-bindgen-cli...${NC}"
        cargo install uniffi-bindgen-cli --version 0.28
    fi

    # Check for cargo-ndk
    if ! command -v cargo-ndk &> /dev/null; then
        echo -e "${YELLOW}Installing cargo-ndk...${NC}"
        cargo install cargo-ndk
    fi

    echo -e "${GREEN}All requirements met!${NC}"
}

# Install Android targets if needed
install_targets() {
    echo -e "${YELLOW}Installing Android targets...${NC}"
    for target in "${!ANDROID_TARGETS[@]}"; do
        if ! rustup target list --installed | grep -q "$target"; then
            echo "Installing target: $target"
            rustup target add "$target"
        fi
    done
    echo -e "${GREEN}All targets installed!${NC}"
}

# Build for all Android targets
build_targets() {
    echo -e "${YELLOW}Building for Android targets...${NC}"

    for target in "${!ANDROID_TARGETS[@]}"; do
        local abi="${ANDROID_TARGETS[$target]}"
        echo "Building for $target ($abi)..."

        cargo ndk \
            --target "$target" \
            --platform "$MIN_API_LEVEL" \
            build --release --features uniffi
    done

    echo -e "${GREEN}All targets built!${NC}"
}

# Generate Kotlin bindings
generate_kotlin_bindings() {
    echo -e "${YELLOW}Generating Kotlin bindings...${NC}"

    mkdir -p "$KOTLIN_OUTPUT_DIR"

    # Use one of the built libraries to generate bindings
    local first_target
    first_target=$(echo "${!ANDROID_TARGETS[@]}" | tr ' ' '\n' | head -n1)
    local lib_path="target/${first_target}/release/${LIB_NAME}"

    if [[ -f "$lib_path" ]]; then
        cargo run --features uniffi --bin uniffi-bindgen generate \
            --config uniffi.toml \
            --library "$lib_path" \
            --language kotlin \
            --out-dir "$KOTLIN_OUTPUT_DIR" 2>/dev/null || \
        uniffi-bindgen generate \
            --config uniffi.toml \
            --library "$lib_path" \
            --language kotlin \
            --out-dir "$KOTLIN_OUTPUT_DIR"
    else
        echo -e "${RED}Error: Library not found at $lib_path${NC}"
        exit 1
    fi

    echo -e "${GREEN}Kotlin bindings generated at ${KOTLIN_OUTPUT_DIR}${NC}"
}

# Organize libraries into jniLibs structure
organize_jni_libs() {
    echo -e "${YELLOW}Organizing JNI libraries...${NC}"

    for target in "${!ANDROID_TARGETS[@]}"; do
        local abi="${ANDROID_TARGETS[$target]}"
        local src_lib="target/${target}/release/${LIB_NAME}"
        local dst_dir="${JNI_LIBS_DIR}/${abi}"

        if [[ -f "$src_lib" ]]; then
            mkdir -p "$dst_dir"
            cp "$src_lib" "$dst_dir/"
            echo "Copied $target -> $abi"
        else
            echo -e "${YELLOW}Warning: Library not found for $target${NC}"
        fi
    done

    echo -e "${GREEN}JNI libraries organized at ${JNI_LIBS_DIR}${NC}"
}

# Create Android library module structure
create_android_library() {
    echo -e "${YELLOW}Creating Android library module structure...${NC}"

    local lib_dir="${OUTPUT_DIR}/cooklang-import-android"
    mkdir -p "${lib_dir}/src/main/kotlin"
    mkdir -p "${lib_dir}/src/main/jniLibs"

    # Copy JNI libs
    cp -R "${JNI_LIBS_DIR}"/* "${lib_dir}/src/main/jniLibs/"

    # Copy Kotlin bindings
    cp "${KOTLIN_OUTPUT_DIR}"/*.kt "${lib_dir}/src/main/kotlin/"

    # Create build.gradle.kts
    cat > "${lib_dir}/build.gradle.kts" << 'EOF'
plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.cooklang.import"
    compileSdk = 34

    defaultConfig {
        minSdk = 21

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

dependencies {
    implementation("net.java.dev.jna:jna:5.14.0@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")

    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
}
EOF

    # Create proguard rules
    cat > "${lib_dir}/proguard-rules.pro" << 'EOF'
# Keep cooklang generated code
-keep class org.cooklang.** { *; }

# Keep JNA classes
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
EOF

    # Create consumer proguard rules
    cat > "${lib_dir}/consumer-rules.pro" << 'EOF'
# Consumer proguard rules for cooklang-import
-keep class org.cooklang.** { *; }
EOF

    # Create AndroidManifest.xml
    mkdir -p "${lib_dir}/src/main"
    cat > "${lib_dir}/src/main/AndroidManifest.xml" << 'EOF'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
</manifest>
EOF

    echo -e "${GREEN}Android library module created at ${lib_dir}${NC}"
}

# Create AAR package info
create_package_info() {
    echo -e "${YELLOW}Creating package info...${NC}"

    cat > "${OUTPUT_DIR}/README.md" << EOF
# CooklangImport Android Library

This directory contains the Android bindings for cooklang-import.

## Contents

- \`cooklang-import-android/\` - Android library module ready to import
- \`jniLibs/\` - Native libraries for each architecture
- \`kotlin/\` - Generated Kotlin bindings

## Supported Architectures

- arm64-v8a (64-bit ARM)
- armeabi-v7a (32-bit ARM)
- x86_64 (64-bit x86)
- x86 (32-bit x86)

## Integration

### Option 1: As a module

1. Copy \`cooklang-import-android\` to your project
2. Add to \`settings.gradle.kts\`:
   \`\`\`kotlin
   include(":cooklang-import-android")
   \`\`\`
3. Add dependency in your app's \`build.gradle.kts\`:
   \`\`\`kotlin
   implementation(project(":cooklang-import-android"))
   \`\`\`

### Option 2: Manual integration

1. Copy \`jniLibs/\` to \`app/src/main/jniLibs/\`
2. Copy Kotlin files from \`kotlin/\` to your source
3. Add JNA dependency:
   \`\`\`kotlin
   implementation("net.java.dev.jna:jna:5.14.0@aar")
   \`\`\`

## Usage

\`\`\`kotlin
import uniffi.cooklang_import.*

// Simple import
val cooklang = simpleImport("https://example.com/recipe")

// With configuration
val config = FfiImportConfig(
    provider = FfiLlmProvider.ANTHROPIC,
    apiKey = "your-api-key",
    model = null,
    timeoutSeconds = 30u,
    extractOnly = false
)
val result = importFromUrl("https://example.com/recipe", config)
\`\`\`

## Requirements

- Minimum SDK: 21 (Android 5.0)
- JNA library for FFI
EOF

    echo -e "${GREEN}Package info created${NC}"
}

# Main execution
main() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  cooklang-import Android Build Script  ${NC}"
    echo -e "${GREEN}========================================${NC}"

    check_requirements
    install_targets
    build_targets
    generate_kotlin_bindings
    organize_jni_libs
    create_android_library
    create_package_info

    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Build Complete!                       ${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Output files:"
    echo "  - JNI Libraries: ${JNI_LIBS_DIR}"
    echo "  - Kotlin bindings: ${KOTLIN_OUTPUT_DIR}"
    echo "  - Android module: ${OUTPUT_DIR}/cooklang-import-android"
    echo ""
    echo "See ${OUTPUT_DIR}/README.md for integration instructions."
}

main "$@"
