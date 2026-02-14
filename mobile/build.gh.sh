#!/usr/bin/env bash
# this is build script running in github actions for github releases.
set -e  # Exit immediately if a command fails
set -x  # Print commands as they execute

# https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md
# ANDROID_HOME 	/usr/local/lib/android/sdk
# ANDROID_NDK 	/usr/local/lib/android/sdk/ndk/27.3.13750724
# ANDROID_NDK_HOME 	/usr/local/lib/android/sdk/ndk/27.3.13750724
# ANDROID_NDK_LATEST_HOME 	/usr/local/lib/android/sdk/ndk/28.2.13676358
# ANDROID_NDK_ROOT 	/usr/local/lib/android/sdk/ndk/27.3.13750724
# ANDROID_SDK_ROOT 	/usr/local/lib/android/sdk

# Your current JDK is located in /usr/lib/jvm/temurin-11-jdk-amd64
export JAVA_HOME=$JAVA_HOME_17_X64

export CC_aarch64_unknown_linux_musl=clang
export AR_aarch64_unknown_linux_musl=llvm-ar
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-Clink-self-contained=yes -Clinker=rust-lld"

# copy keystore to release dir
if [[ -d "$HOME/.projects/release.keystore" ]]; then
    cp -r $HOME/.projects/release.keystore ./app/
fi

rustup target add armv7-linux-androideabi
rustup target add aarch64-linux-android
rustup target add i686-linux-android
rustup target add x86_64-linux-android
cargo install cargo-ndk

timestamp=$(date +%y%m%d%H%M)
export APPLICATION_VERSION_CODE=${timestamp:0:-1}
export APPLICATION_VERSION_NAME=$(grep -m1 "^version = " ../Cargo.toml | cut -d' ' -f3 | tr -d '"')

export RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none"
cargo ndk -t armeabi-v7a -o app/src/main/jniLibs/ build --release --lib
cargo ndk -t arm64-v8a -o app/src/main/jniLibs/ build --release --lib
cargo ndk -t x86 -o app/src/main/jniLibs/ build --release --lib
cargo ndk -t x86_64 -o app/src/main/jniLibs/ build --release --lib

# Build APK
gradle assembleRelease

# Build AAB (Android App Bundle) for Google Play
gradle bundleRelease

# Verify AAB was created
if [ ! -f "app/build/outputs/bundle/release/app-release.aab" ]; then
    echo "ERROR: AAB file was not created at app/build/outputs/bundle/release/app-release.aab"
    exit 1
fi
echo "Successfully created AAB at app/build/outputs/bundle/release/app-release.aab"
ls -lh app/build/outputs/bundle/release/app-release.aab