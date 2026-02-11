#!/usr/bin/env bash
# this is a build script for fdroid system.
# gitlab-runner 18.3.0~pre  registry.gitlab.com/fdroid/fdroidserver:buildserver-bookworm

# extrepo enable debian_official
# [[ $(dpkg -l|grep libssl-dev|grep -c "libssl-dev") -lt 1 ]] && apt install libssl-dev squashfs-tools
# [[ $(which x|grep -c "/x") -lt 1 ]] && cargo install xbuild
[[ ! -d "$HOME/.local" ]] && mkdir -p $HOME/.local $HOME/.android

# https://static.rust-lang.org/dist/rust-1.88.0-x86_64-unknown-linux-gnu.tar.xz
# tar -xvf rust-1.88.0-x86_64-unknown-linux-gnu.tar.xz
# mv rust-1.88.0-x86_64-unknown-linux-gnu $HOME/.local/

if [[ ! -d "$HOME/.local/jdk-24.0.1/bin" ]]; then
    echo "Installing JDK 24..."
    durl="https://download.java.net/java/GA/jdk24.0.1/24a58e0e276943138bf3e963e6291ac2/9/GPL/openjdk-24.0.1_linux-x64_bin.tar.gz"
    pushd "$HOME/Downloads"
    wget --directory-prefix="$HOME/Downloads" "${durl}" 2>&1 1>/dev/null
    tar -zxvf openjdk-24.0.1_linux-x64_bin.tar.gz 2>&1 1>/dev/null
    mv jdk-24.0.1/ $HOME/.local/
    popd
fi

if [[ ! -d "$HOME/.local/LLVM-20.1.0-Linux-X64/bin" ]]; then
    echo "Installing LLVM20..."
    durl="https://github.com/llvm/llvm-project/releases/download/llvmorg-20.1.0/LLVM-20.1.0-Linux-X64.tar.xz"
    pushd "$HOME/Downloads"
    wget --directory-prefix="$HOME/Downloads" "${durl}" 2>&1 1>/dev/null
    tar -xvf LLVM-20.1.0-Linux-X64.tar.xz 2>&1 1>/dev/null
    mv LLVM-20.1.0-Linux-X64 $HOME/.local/
    popd
fi

if [[ ! -d "$HOME/.android/platform-tools" ]]; then
    echo "Installing android platform-tools..."
    durl="https://dl.google.com/android/repository/platform-tools-latest-linux.zip"
    pushd "$HOME/Downloads"
    wget --directory-prefix="$HOME/Downloads" "${durl}" 2>&1 1>/dev/null
    unzip platform-tools-latest-linux.zip 2>&1 1>/dev/null
    mv platform-tools $HOME/.android
    popd
fi

if [[ ! -d "$HOME/.local/kotlinc/bin" ]]; then
    echo "Installing android kotlin compiler..."
    durl="https://github.com/JetBrains/kotlin/releases/download/v2.2.0/kotlin-compiler-2.2.0.zip"
    pushd "$HOME/Downloads"
    wget --directory-prefix="$HOME/Downloads" "${durl}" 2>&1 1>/dev/null
    tar -zxvf kotlin-native-prebuilt-linux-x86_64-2.2.0.tar.gz 2>&1 1>/dev/null
    mv kotlin-native-prebuilt-linux-x86_64-2.2.0 $HOME/.local
    popd
fi

if [[ ! -d "$HOME/.local/kotlin-native-prebuilt-linux-x86_64-2.2.0/bin" ]]; then
    echo "Installing android kotlin native..."
    durl="https://github.com/JetBrains/kotlin/releases/download/v2.2.0/kotlin-native-prebuilt-linux-x86_64-2.2.0.tar.gz"
    pushd "$HOME/Downloads"
    wget --directory-prefix="$HOME/Downloads" "${durl}" 2>&1 1>/dev/null
    unzip kotlin-compiler-2.2.0.zip 2>&1 1>/dev/null
    mv kotlinc $HOME/.local/
    popd
fi

if [[ ! -d "$HOME/.local/gradle-9.0.0" ]]; then
    echo "Installing android gradle..."
    durl="https://services.gradle.org/distributions/gradle-9.0.0-bin.zip"
    pushd "$HOME/Downloads"
    wget --directory-prefix="$HOME/Downloads" "${durl}" 2>&1 1>/dev/null
    unzip gradle-9.0.0-bin.zip 2>&1 1>/dev/null
    mv gradle-9.0.0 $HOME/.local
    popd
fi

if [[ ! -d "$HOME/.android/ndk/android-ndk-r28c" ]]; then
    echo "Installing android ndk..."
    durl="https://dl.google.com/android/repository/android-ndk-r28c-linux.zip"
    pushd "$HOME/Downloads"
    wget --directory-prefix="$HOME/Downloads" "${durl}" 2>&1 1>/dev/null
    unzip android-ndk-r28c-linux.zip 2>&1 1>/dev/null
    mv android-ndk-r28c $HOME/.android/ndk
    popd
fi

# if latest folder name in $HOME/.android/ndk is not equal to $HOME/.cache/x/Android.ndk/latest contents
if [[ $(cat $HOME/.cache/x/Android.ndk/latest) != "android-ndk-r28c" ]]; then
    bash android.ndk.sh
fi

: <<'END_COMMENT'
export PATH=$PATH:$HOME/.local/jdk-24.0.1/bin
export PATH=$PATH:$HOME/.local/LLVM-20.1.0-Linux-X64/bin
export PATH=$PATH:$HOME/.android/platform-tools
export PATH=$PATH:$HOME/.local/kotlin-native-prebuilt-linux-x86_64-2.2.0/bin
export PATH=$PATH:$HOME/.local/kotlinc/bin
export PATH=$PATH:$HOME/.local/gradle-9.0.0/bin

export JAVA_HOME=$HOME/.local/jdk-24.0.1
export ANDROID_HOME=$HOME/.android
export ANDROID_NDK_HOME=$HOME/.android/ndk/android-ndk-r28c/
export NDK_HOME=$HOME/.android/ndk/android-ndk-r28c/
export ANDROID_NDK_ROOT=$HOME/.android/ndk
export ANDROID_CMDLINE_TOOLS=$HOME/.android/cmdline-tools/latest/bin
export ANDROID_TOOLS=$HOME/.android/tools/bin
export ANDROID_PLATFORM_TOOLS=$HOME/.android/platform-tools
END_COMMENT

[[ $(grep -c "jdk-24.0.1" $HOME/.bashrc) -lt 1 ]] && echo "export PATH=\$PATH:\$HOME/.local/jdk-24.0.1/bin" >> $HOME/.bashrc
[[ $(grep -c "LLVM-20.1.0-Linux-X64" $HOME/.bashrc) -lt 1 ]] && echo "export PATH=\$PATH:\$HOME/.local/LLVM-20.1.0-Linux-X64/bin" >> $HOME/.bashrc
[[ $(grep -c "platform-tools" $HOME/.bashrc) -lt 1 ]] && echo "export PATH=\$PATH:\$HOME/.android/platform-tools" >> $HOME/.bashrc
[[ $(grep -c "kotlin-native-prebuilt-linux-x86_64-2.2.0" $HOME/.bashrc) -lt 1 ]] && echo "export PATH=\$PATH:\$HOME/.local/kotlin-native-prebuilt-linux-x86_64-2.2.0/bin" >> $HOME/.bashrc
[[ $(grep -c "kotlinc" $HOME/.bashrc) -lt 1 ]] && echo "export PATH=\$PATH:\$HOME/.local/kotlinc/bin" >> $HOME/.bashrc
[[ $(grep -c "gradle-9.0.0" $HOME/.bashrc) -lt 1 ]] && echo "export PATH=\$PATH:\$HOME/.local/gradle-9.0.0/bin" >> $HOME/.bashrc

# https://github.com/aws/aws-lc-rs/blob/main/aws-lc-rs/README.md#Build
# russh -> cmake  >> apt install cmake

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
gradle build
gradle assembleRelease

# adb commands
# adb devices
# adb install app/bulid/outputs/apk/release/app-release.apk
# adb uninstall pe.nikescar.uad_shizuku
# adb shell am start -n pe.nikescar.uad_shizuku/.MainActivity

# logcat commands
# adb logcat -c
# adb logcat -v time -s *:V > fullcat.log
# adb logcat -s UAD-Shizuku > uadcat.log