# Makefile for cooklang-import
# Supports building for iOS, Android, and native platforms

.PHONY: all build test lint clean ios android bindings help install-deps

# Default target
all: build

# Build the library (native)
build:
	cargo build --release

# Build with UniFFI feature
build-uniffi:
	cargo build --release --features uniffi

# Run tests
test:
	cargo test

# Run tests with UniFFI feature
test-uniffi:
	cargo test --features uniffi

# Run lints
lint:
	cargo fmt --check
	cargo clippy -- -D warnings

# Format code
fmt:
	cargo fmt

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/ios target/android

# Clean only mobile build artifacts
clean-mobile:
	rm -rf target/ios target/android

# === iOS Builds ===

# Build for iOS (requires macOS)
ios:
	@chmod +x scripts/build-ios.sh
	@scripts/build-ios.sh

# Build iOS release artifacts
ios-release: ios
	@echo "iOS release artifacts ready in target/ios/"

# === Android Builds ===

# Build for Android
android:
	@chmod +x scripts/build-android.sh
	@scripts/build-android.sh

# Build Android release artifacts
android-release: android
	@echo "Android release artifacts ready in target/android/"

# === All Mobile Platforms ===

# Build for all mobile platforms
mobile: ios android
	@echo "All mobile builds complete!"

# === Bindings Generation ===

# Generate Swift bindings only (without full iOS build)
bindings-swift: build-uniffi
	@mkdir -p target/bindings/swift
	uniffi-bindgen generate \
		--config uniffi.toml \
		--library target/release/libcooklang_import.dylib \
		--language swift \
		--out-dir target/bindings/swift || \
	uniffi-bindgen generate \
		--config uniffi.toml \
		--library target/release/libcooklang_import.so \
		--language swift \
		--out-dir target/bindings/swift

# Generate Kotlin bindings only (without full Android build)
bindings-kotlin: build-uniffi
	@mkdir -p target/bindings/kotlin
	uniffi-bindgen generate \
		--library target/release/libcooklang_import.so \
		--language kotlin \
		--out-dir target/bindings/kotlin || \
	uniffi-bindgen generate \
		--library target/release/libcooklang_import.dylib \
		--language kotlin \
		--out-dir target/bindings/kotlin

# Generate Python bindings
bindings-python: build-uniffi
	@mkdir -p target/bindings/python
	uniffi-bindgen generate \
		--library target/release/libcooklang_import.so \
		--language python \
		--out-dir target/bindings/python || \
	uniffi-bindgen generate \
		--library target/release/libcooklang_import.dylib \
		--language python \
		--out-dir target/bindings/python

# Generate Ruby bindings
bindings-ruby: build-uniffi
	@mkdir -p target/bindings/ruby
	uniffi-bindgen generate \
		--library target/release/libcooklang_import.so \
		--language ruby \
		--out-dir target/bindings/ruby || \
	uniffi-bindgen generate \
		--library target/release/libcooklang_import.dylib \
		--language ruby \
		--out-dir target/bindings/ruby

# Generate all bindings
bindings: bindings-swift bindings-kotlin bindings-python bindings-ruby
	@echo "All bindings generated in target/bindings/"

# === Dependencies ===

# Install development dependencies
install-deps:
	rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios || true
	rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android || true
	@echo "uniffi-bindgen is built as part of this project"
	cargo install cargo-ndk || true

# === Release ===

# Create a release build with all artifacts
release: build-uniffi
	@echo "Release build complete!"
	@echo "Library: target/release/libcooklang_import.*"

# === Documentation ===

# Generate documentation
docs:
	cargo doc --no-deps --features uniffi
	@echo "Documentation generated at target/doc/cooklang_import/"

# === Help ===

help:
	@echo "cooklang-import build system"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Native targets:"
	@echo "  build         - Build native library (release)"
	@echo "  build-uniffi  - Build with UniFFI feature enabled"
	@echo "  test          - Run tests"
	@echo "  test-uniffi   - Run tests with UniFFI feature"
	@echo "  lint          - Run lints (fmt check + clippy)"
	@echo "  fmt           - Format code"
	@echo "  docs          - Generate documentation"
	@echo "  clean         - Clean all build artifacts"
	@echo ""
	@echo "Mobile targets:"
	@echo "  ios           - Build iOS XCFramework + Swift bindings"
	@echo "  android       - Build Android AAR + Kotlin bindings"
	@echo "  mobile        - Build for both iOS and Android"
	@echo "  clean-mobile  - Clean only mobile artifacts"
	@echo ""
	@echo "Bindings targets:"
	@echo "  bindings-swift   - Generate Swift bindings only"
	@echo "  bindings-kotlin  - Generate Kotlin bindings only"
	@echo "  bindings-python  - Generate Python bindings"
	@echo "  bindings-ruby    - Generate Ruby bindings"
	@echo "  bindings         - Generate all bindings"
	@echo ""
	@echo "Setup:"
	@echo "  install-deps  - Install required tools and targets"
	@echo ""
	@echo "Release:"
	@echo "  release       - Create release build"
	@echo "  ios-release   - Create iOS release artifacts"
	@echo "  android-release - Create Android release artifacts"
