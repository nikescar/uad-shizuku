---
title: Development
---

# Development

This guide covers setting up a development environment for UAD-Shizuku, including building the desktop application, Android APK, and documentation.

## Prerequisites

Before you begin, ensure you have the following installed:

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | Latest stable | Core language for the application |
| Git | Any recent | Source code management |
| ADB | Latest | Android device communication |

### Installing Rust

Rust is required to build UAD-Shizuku. Install it using rustup:

**Windows:**
Download and run the installer from https://rustup.rs/

**macOS/Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify the installation:
```bash
rustc --version
cargo --version
```

### Recommended IDE Setup

For the best development experience, we recommend:

- **Visual Studio Code** with the following extensions:
  - rust-analyzer (Rust language support)
  - Even Better TOML (Cargo.toml editing)
  - CodeLLDB (Debugging)

- **RustRover** (JetBrains IDE for Rust)

---

## Project Structure

```
uad-shizuku/
├── src/              # Desktop application source code
├── mobile/           # Android build files and scripts
│   ├── app/          # Android app module
│   ├── build.sh      # Android build script
│   └── qemu.sh       # QEMU container management
├── docs/             # Documentation (this site)
├── resources/        # Static assets and resources
└── Cargo.toml        # Rust project configuration
```

---

## Building Desktop Application

The desktop application can be built for Windows, macOS, and Linux.

### Clone the Repository

```bash
git clone https://github.com/nikescar/uad-shizuku
cd uad-shizuku
```

### Development Build

For quick iteration during development:

```bash
cargo build
```

The debug binary will be at `target/debug/uad-shizuku` (or `uad-shizuku.exe` on Windows).

### Release Build

For optimized production builds:

```bash
cargo build --release
```

The release binary will be at `target/release/uad-shizuku`.

### Running the Application

```bash
# Development
cargo run

# Release mode
cargo run --release
```

### Cross-Compilation Targets

UAD-Shizuku supports building for multiple targets:

| Target | Triple |
|--------|--------|
| Windows x64 | `x86_64-pc-windows-msvc` |
| macOS x64 | `x86_64-apple-darwin` |
| macOS ARM64 | `aarch64-apple-darwin` |
| Linux x64 | `x86_64-unknown-linux-musl` |
| Linux ARM64 | `aarch64-linux-android` |

To build for a specific target:
```bash
rustup target add <target-triple>
cargo build --release --target <target-triple>
```

---

## Building Android APK

The Android version is built using a QEMU-based Alpine Linux container to ensure consistent builds across different host systems.

### Prerequisites for Android Build

- QEMU installed on your system
- SSH client
- rsync

### Build Steps

```bash
cd mobile

# Step 1: Initialize the QEMU guest container (first time only)
./qemu.sh init

# Step 2: Start the QEMU guest
./qemu.sh run

# Step 3: Sync project files to the container
./qemu.sh syncto

# Step 4: SSH into the Alpine Linux container
./qemu.sh ssh

# Step 5: Build inside the container
cd /opt/uad-shizuku/mobile && ./build.sh

# Step 6: Exit SSH and sync built files back
exit
./qemu.sh syncfrom

# Step 7: Find the built APKs
ls app/build/outputs/apk/debug/
ls app/build/outputs/apk/release/
```

### Installing Debug Build

```bash
adb install app/build/outputs/apk/debug/app-arm64-v8a-debug.apk
```

### QEMU Script Commands

| Command | Description |
|---------|-------------|
| `./qemu.sh init` | Initialize and set up the build container |
| `./qemu.sh run` | Start the QEMU virtual machine |
| `./qemu.sh stop` | Stop the running QEMU instance |
| `./qemu.sh ssh` | SSH into the running container |
| `./qemu.sh syncto` | Copy project files to the container |
| `./qemu.sh syncfrom` | Copy built files from the container |

---

## Building Documentation

The documentation site is built using mdx-sitegen-solidbase.

### Prerequisites

- Node.js (for npx)
- darkhttpd (optional, for local preview)

### Build Documentation

```bash
# Build documentation from project root
npx github:nikescar/mdx-sitegen-solidbase

# Preview locally
darkhttpd .solidbase/.output/public
```

The built site will be in `.solidbase/.output/public/`.

### Documentation Structure

```
docs/
├── index.md           # Home page
├── installation.md    # Installation guide
├── development.md     # This file
├── usage.md           # Usage guide
└── kr/                # Korean translations
    └── docs/
        ├── index.md
        ├── installation.md
        ├── development.md
        └── usage.md
```

---

## Development Workflow

### Making Changes

1. Create a new branch for your feature or fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes and test locally:
   ```bash
   cargo build
   cargo run
   ```

3. Commit your changes with a descriptive message:
   ```bash
   git add .
   git commit -m "feat: add your feature description"
   ```

4. Push and create a pull request:
   ```bash
   git push origin feature/your-feature-name
   ```

### Code Formatting

Ensure your code follows Rust formatting standards:

```bash
cargo fmt
```

### Linting

Check for common issues:

```bash
cargo clippy
```

---

## Troubleshooting

### Build Errors

**Missing dependencies:**
```bash
# Debian/Ubuntu
sudo apt install build-essential pkg-config libssl-dev

# Fedora
sudo dnf install gcc pkg-config openssl-devel

# macOS (with Homebrew)
brew install openssl pkg-config
```

### QEMU Issues

**QEMU not starting:**
- Ensure QEMU is installed: `qemu-system-x86_64 --version`
- Check available disk space
- Review QEMU logs for specific errors

**Sync failures:**
- Ensure the QEMU guest is running
- Check network connectivity between host and guest
