#!/usr/bin/env bash
# Snapcraft build and publish automation script
# Converted from Snap.Makefile

set -e

# Default configuration
ARCH="${ARCH:-$(dpkg --print-architecture)}"
SNAP_NAME="${SNAP_NAME:-dure}"
VERSION="${VERSION:-$(grep '^version:' snapcraft.yaml 2>/dev/null | awk '{print $2}' || echo "unknown")}"
SNAP_FILE="${SNAP_NAME}_${VERSION}_${ARCH}.snap"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
info() {
    echo -e "${GREEN}$*${NC}"
}

warn() {
    echo -e "${YELLOW}$*${NC}"
}

error() {
    echo -e "${RED}$*${NC}" >&2
}

show_help() {
    cat <<EOF
Dure Snapcraft Build and Publish Script

Usage: $(basename "$0") [COMMAND] [OPTIONS]

Commands:
  build              Build the snap package (destructive mode, fast)
  build-multipass    Build using Multipass VM (requires setup)
  build-all          Build snap packages for all architectures
  clean              Remove all build artifacts
  install            Install the snap locally (dangerous mode)
  test               Test the installed snap
  publish            Publish to edge channel (default)
  publish-edge       Publish to edge channel
  publish-beta       Publish to beta channel
  publish-candidate  Publish to candidate channel
  publish-stable     Publish to stable channel
  login              Login to snapcraft store
  logout             Logout from snapcraft store
  status             Show snap status in the store
  info               Show local snap information
  exportlogin        Export snapcraft login credentials
  help               Show this help message

Environment Variables:
  ARCH               Target architecture (default: $(dpkg --print-architecture))
  VERSION            Snap version (default: from snapcraft.yaml)
  SNAP_NAME          Snap package name (default: dure)

Current Configuration:
  ARCH=$ARCH
  VERSION=$VERSION
  SNAP_NAME=$SNAP_NAME
  SNAP_FILE=$SNAP_FILE

Examples:
  $(basename "$0") build                    # Build for current architecture
  ARCH=arm64 $(basename "$0") build         # Build for arm64
  $(basename "$0") install                  # Install locally for testing
  $(basename "$0") publish-edge             # Publish to edge channel
  $(basename "$0") publish-stable           # Publish to stable channel
EOF
}

cmd_build() {
    info "Building snap for architecture: $ARCH (destructive mode)"
    snapcraft --destructive-mode --verbose
    info "Build complete: $SNAP_FILE"
    ls -lh ./*.snap 2>/dev/null || true
}

cmd_build_multipass() {
    info "Building snap for architecture: $ARCH (using Multipass VM)"
    snapcraft --verbose
    info "Build complete: $SNAP_FILE"
    ls -lh ./*.snap 2>/dev/null || true
}

cmd_build_all() {
    info "Building snap for all architectures..."
    for arch in amd64 arm64 armhf; do
        info "Building for $arch..."
        snapcraft --verbose --target-arch="$arch" || true
    done
    info "Build complete"
    ls -lh ./*.snap 2>/dev/null || true
}

cmd_clean() {
    info "Cleaning build artifacts..."
    snapcraft clean
    rm -f ./*.snap
    rm -rf ./parts ./stage ./prime
    info "Clean complete"
}

cmd_install() {
    cmd_build
    info "Installing snap locally (dangerous mode)..."
    if [ -f "$SNAP_FILE" ]; then
        sudo snap install --dangerous "$SNAP_FILE"
        info "Installation complete"
    else
        error "Snap file not found: $SNAP_FILE"
        echo "Available snaps:"
        ls -1 ./*.snap 2>/dev/null || echo "No snap files found"
        exit 1
    fi
}

cmd_test() {
    info "Testing installed snap..."
    snap list "$SNAP_NAME" || error "Snap not installed"
    snap connections "$SNAP_NAME" || true
    echo ""
    echo "To run the application:"
    echo "  $SNAP_NAME"
}

cmd_login() {
    info "Logging in to snapcraft store..."
    snapcraft login
}

cmd_logout() {
    info "Logging out from snapcraft store..."
    snapcraft logout
}

cmd_status() {
    info "Checking snap status in store..."
    snapcraft status "$SNAP_NAME"
}

cmd_info() {
    echo "Snap information:"
    echo "  Name: $SNAP_NAME"
    echo "  Version: $VERSION"
    echo "  Architecture: $ARCH"
    echo ""
    echo "Local snap files:"
    ls -lh ./*.snap 2>/dev/null || echo "No snap files found"
}

cmd_publish_to_channel() {
    local channel="$1"
    info "Publishing to $channel channel..."
    if [ -f "$SNAP_FILE" ]; then
        snapcraft upload --release="$channel" "$SNAP_FILE"
        info "Published to $channel channel"
    else
        error "Snap file not found: $SNAP_FILE"
        echo "Available snaps:"
        ls -1 ./*.snap 2>/dev/null || echo "No snap files found"
        echo ""
        echo "Run '$(basename "$0") build' first or specify the correct ARCH/VERSION"
        exit 1
    fi
}

cmd_publish() {
    cmd_publish_to_channel "edge"
}

cmd_publish_edge() {
    cmd_publish_to_channel "edge"
}

cmd_publish_beta() {
    cmd_publish_to_channel "beta"
}

cmd_publish_candidate() {
    cmd_publish_to_channel "candidate"
}

cmd_publish_stable() {
    warn "⚠️  WARNING: Publishing to stable channel!"
    read -p "Are you sure? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if [ -f "$SNAP_FILE" ]; then
            snapcraft upload --release=stable "$SNAP_FILE"
            info "Published to stable channel"
        else
            error "Snap file not found: $SNAP_FILE"
            echo "Run '$(basename "$0") build' first or specify the correct ARCH/VERSION"
            exit 1
        fi
    else
        warn "Cancelled"
    fi
}

cmd_exportlogin() {
    info "Exporting snapcraft login credentials..."
    snapcraft export-login --snaps=dure \
        --acls package_access,package_push,package_update,package_release \
        exported.txt
    info "Credentials exported to exported.txt"
}

# Main command dispatcher
main() {
    local cmd="${1:-help}"
    shift || true

    case "$cmd" in
        build)
            cmd_build "$@"
            ;;
        build-multipass)
            cmd_build_multipass "$@"
            ;;
        build-all)
            cmd_build_all "$@"
            ;;
        clean)
            cmd_clean "$@"
            ;;
        install)
            cmd_install "$@"
            ;;
        test)
            cmd_test "$@"
            ;;
        publish)
            cmd_publish "$@"
            ;;
        publish-edge)
            cmd_publish_edge "$@"
            ;;
        publish-beta)
            cmd_publish_beta "$@"
            ;;
        publish-candidate)
            cmd_publish_candidate "$@"
            ;;
        publish-stable)
            cmd_publish_stable "$@"
            ;;
        login)
            cmd_login "$@"
            ;;
        logout)
            cmd_logout "$@"
            ;;
        status)
            cmd_status "$@"
            ;;
        info)
            cmd_info "$@"
            ;;
        exportlogin)
            cmd_exportlogin "$@"
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            error "Unknown command: $cmd"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

main "$@"
