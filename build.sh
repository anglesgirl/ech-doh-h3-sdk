#!/bin/bash
# Build script for ech-doh-h3-sdk
# Usage: ./build.sh [android|ios|all|bindings]

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_NAME="ech_doh_h3_sdk"
OUT_DIR="${PROJECT_ROOT}/target"

echo "=== ech-doh-h3-sdk Build Script ==="
echo "Project root: ${PROJECT_ROOT}"
echo "Crate name: ${CRATE_NAME}"
echo ""

# Function to install build dependencies
install_deps() {
    echo "Installing build dependencies..."
    
    # Rust targets for Android
    rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android 2>/dev/null || true
    
    # Rust targets for iOS
    rustup target add aarch64-apple-ios x86_64-apple-ios 2>/dev/null || true
    
    # cargo-ndk for Android
    cargo install cargo-ndk --locked 2>/dev/null || true
    
    # cargo-lipo for iOS universal binaries
    cargo install cargo-lipo --locked 2>/dev/null || true
    
    # uniffi-bindgen for generating bindings
    cargo install uniffi-bindgen --locked 2>/dev/null || true
    
    # NDK path (if ANDROID_NDK_HOME not set, try common locations)
    if [ -z "$ANDROID_NDK_HOME" ]; then
        if [ -d "$HOME/Android/Sdk/ndk" ]; then
            export ANDROID_NDK_HOME=$(ls -d $HOME/Android/Sdk/ndk/* | head -1)
        elif [ -d "/opt/android/ndk" ]; then
            export ANDROID_NDK_HOME=$(ls -d /opt/android/ndk/* | head -1)
        fi
    fi
    
    echo "ANDROID_NDK_HOME: ${ANDROID_NDK_HOME}"
}

# Function to generate UniFFI bindings
generate_bindings() {
    echo "Generating UniFFI bindings..."
    cd "${PROJECT_ROOT}"
    
    # Generate scaffolding
    cargo build --features uniffi_cli 2>/dev/null || cargo build --features uniffi_cli
    
    # Generate Kotlin bindings
    echo "Generating Kotlin bindings..."
    cargo run --features uniffi_cli -- uniffi-bindgen generate src/lib.rs --language kotlin --out-dir "${OUT_DIR}/bindings/kotlin" 2>/dev/null || true
    
    # Generate Swift bindings
    echo "Generating Swift bindings..."
    cargo run --features uniffi_cli -- uniffi-bindgen generate src/lib.rs --language swift --out-dir "${OUT_DIR}/bindings/swift" 2>/dev/null || true
}

# Function to build for Android
build_android() {
    echo ""
    echo "=== Building for Android ==="
    
    if [ -z "$ANDROID_NDK_HOME" ]; then
        echo "ERROR: ANDROID_NDK_HOME not set. Please set it to your NDK path."
        echo "Example: export ANDROID_NDK_HOME=/opt/android/ndk/25.2.9519653"
        return 1
    fi
    
    cd "${PROJECT_ROOT}"
    
    # Build for each architecture
    for TARGET in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
        echo "Building for ${TARGET}..."
        cargo ndk --target "${TARGET}" -o "${OUT_DIR}/android/jniLibs" build --release
    done
    
    # Generate AAR
    echo "Generating AAR..."
    mkdir -p "${OUT_DIR}/android/aar"
    
    # Create a simple AAR structure (you'd typically use Android Studio/Gradle for this)
    cat > "${OUT_DIR}/android/build.gradle" << 'EOF'
apply plugin: 'com.android.library'

android {
    namespace 'com.example.echdohh3sdk'
    compileSdk 34

    defaultConfig {
        minSdk 21
        targetSdk 34
        versionCode 1
        versionName "0.1.0"
    }

    sourceSets {
        main {
            jniLibs.srcDirs = ['jniLibs']
        }
    }
}
EOF
    
    echo "Android build complete. Output in: ${OUT_DIR}/android"
}

# Function to build for iOS
build_ios() {
    echo ""
    echo "=== Building for iOS ==="
    
    cd "${PROJECT_ROOT}"
    
    # Build for device (arm64)
    echo "Building for aarch64-apple-ios (device)..."
    cargo build --target aarch64-apple-ios --release
    
    # Build for simulator (x86_64)
    echo "Building for x86_64-apple-ios (simulator)..."
    cargo build --target x86_64-apple-ios --release
    
    # Create XCFramework
    echo "Creating XCFramework..."
    XCFRAMEWORK_PATH="${OUT_DIR}/ios/${CRATE_NAME}.xcframework"
    rm -rf "${XCFRAMEWORK_PATH}"
    
    xcodebuild -create-xcframework \
        -library "${OUT_DIR}/aarch64-apple-ios/release/lib${CRATE_NAME}.a" \
        -headers "${OUT_DIR}/bindings/swift" \
        -library "${OUT_DIR}/x86_64-apple-ios/release/lib${CRATE_NAME}.a" \
        -headers "${OUT_DIR}/bindings/swift" \
        -output "${XCFRAMEWORK_PATH}"
    
    echo "iOS build complete. XCFramework at: ${XCFRAMEWORK_PATH}"
}

# Main script logic
COMMAND="${1:-all}"

case "${COMMAND}" in
    deps)
        install_deps
        ;;
    bindings)
        generate_bindings
        ;;
    android)
        install_deps
        build_android
        ;;
    ios)
        install_deps
        build_ios
        ;;
    all)
        install_deps
        generate_bindings
        build_android
        build_ios
        ;;
    *)
        echo "Usage: $0 [deps|bindings|android|ios|all]"
        echo "  deps      - Install build dependencies"
        echo "  bindings  - Generate UniFFI bindings (Kotlin/Swift)"
        echo "  android   - Build Android .so libraries and AAR"
        echo "  ios       - Build iOS XCFramework"
        echo "  all       - Build everything (default)"
        exit 1
        ;;
esac

echo ""
echo "=== Build Complete ==="