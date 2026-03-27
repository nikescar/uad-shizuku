.PHONY: help build build-multipass clean install test publish publish-edge publish-beta publish-candidate publish-stable login logout status exportlogin
 
# Default architecture (can be overridden with: make build ARCH=arm64)
ARCH ?= $(shell dpkg --print-architecture)
 
# Snap name from snapcraft.yaml
SNAP_NAME = uad-shizuku
 
# Get version from snapcraft.yaml or git
VERSION ?= $(shell grep '^version:' ../snapcraft.yaml | awk '{print $$2}')
 
# Snap file pattern
SNAP_FILE = $(SNAP_NAME)_$(VERSION)_$(ARCH).snap
 
help:
	@echo "UAD-SHIZUKU Snapcraft Build and Publish Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build              - Build the snap package (destructive mode, fast)"
	@echo "  build-multipass    - Build using Multipass VM (requires setup)"
	@echo "  build-all          - Build snap packages for all architectures"
	@echo "  clean              - Remove all build artifacts"
	@echo "  install            - Install the snap locally (dangerous mode)"
	@echo "  test               - Test the installed snap"
	@echo "  publish            - Publish to edge channel (default)"
	@echo "  publish-edge       - Publish to edge channel"
	@echo "  publish-beta       - Publish to beta channel"
	@echo "  publish-candidate  - Publish to candidate channel"
	@echo "  publish-stable     - Publish to stable channel"
	@echo "  login              - Login to snapcraft store"
	@echo "  logout             - Logout from snapcraft store"
	@echo "  status             - Show snap status in the store"
	@echo "  info               - Show local snap information"
	@echo ""
	@echo "Variables:"
	@echo "  ARCH=$(ARCH)"
	@echo "  VERSION=$(VERSION)"
	@echo ""
	@echo "Examples:"
	@echo "  make build              # Build for current architecture"
	@echo "  make build ARCH=arm64   # Build for arm64"
	@echo "  make install            # Install locally for testing"
	@echo "  make publish-edge       # Publish to edge channel"
	@echo "  make publish-stable     # Publish to stable channel"
 
build:
	@echo "Building snap for architecture: $(ARCH) (destructive mode)"
	@cd ../../.. && snapcraft --destructive-mode --verbose
	@echo "Build complete: $(SNAP_FILE)"
	@ls -lh ../../../*.snap || true
 
build-multipass:
	@echo "Building snap for architecture: $(ARCH) (using Multipass VM)"
	@cd ../../.. && snapcraft --verbose
	@echo "Build complete: $(SNAP_FILE)"
	@ls -lh ../../../*.snap || true
 
build-all:
	@echo "Building snap for all architectures..."
	@for arch in amd64 arm64 armhf; do \
		echo "Building for $$arch..."; \
		cd ../.. && snapcraft --verbose --target-arch=$$arch || true; \
	done
	@echo "Build complete"
	@ls -lh ../../*.snap || true
 
clean:
	@echo "Cleaning build artifacts..."
	@cd ../.. && snapcraft clean
	@rm -f ../../*.snap
	@rm -rf ../../parts ../../stage ../../prime
	@echo "Clean complete"
 
install: build
	@echo "Installing snap locally (dangerous mode)..."
	@if [ -f "../../$(SNAP_FILE)" ]; then \
		sudo snap install --dangerous "../../$(SNAP_FILE)"; \
	else \
		echo "Snap file not found: $(SNAP_FILE)"; \
		echo "Available snaps:"; \
		ls -1 ../../*.snap 2>/dev/null || echo "No snap files found"; \
		exit 1; \
	fi
	@echo "Installation complete"
 
test:
	@echo "Testing installed snap..."
	@snap list $(SNAP_NAME) || echo "Snap not installed"
	@snap connections $(SNAP_NAME) || true
	@echo ""
	@echo "To run the application:"
	@echo "  $(SNAP_NAME)"
 
login:
	@echo "Logging in to snapcraft store..."
	@snapcraft login
 
logout:
	@echo "Logging out from snapcraft store..."
	@snapcraft logout
 
status:
	@echo "Checking snap status in store..."
	@snapcraft status $(SNAP_NAME)
 
info:
	@echo "Snap information:"
	@echo "  Name: $(SNAP_NAME)"
	@echo "  Version: $(VERSION)"
	@echo "  Architecture: $(ARCH)"
	@echo ""
	@echo "Local snap files:"
	@ls -lh ../../*.snap 2>/dev/null || echo "No snap files found"
 
publish: publish-edge
 
publish-edge:
	@echo "Publishing to edge channel..."
	@if [ -f "../../$(SNAP_FILE)" ]; then \
		snapcraft upload --release=edge "../../$(SNAP_FILE)"; \
	else \
		echo "Snap file not found: $(SNAP_FILE)"; \
		echo "Available snaps:"; \
		ls -1 ../../*.snap 2>/dev/null || echo "No snap files found"; \
		echo ""; \
		echo "Run 'make build' first or specify the correct ARCH/VERSION"; \
		exit 1; \
	fi
	@echo "Published to edge channel"
 
publish-beta:
	@echo "Publishing to beta channel..."
	@if [ -f "../../$(SNAP_FILE)" ]; then \
		snapcraft upload --release=beta "../../$(SNAP_FILE)"; \
	else \
		echo "Snap file not found: $(SNAP_FILE)"; \
		echo "Run 'make build' first or specify the correct ARCH/VERSION"; \
		exit 1; \
	fi
	@echo "Published to beta channel"
 
publish-candidate:
	@echo "Publishing to candidate channel..."
	@if [ -f "../../$(SNAP_FILE)" ]; then \
		snapcraft upload --release=candidate "../../$(SNAP_FILE)"; \
	else \
		echo "Snap file not found: $(SNAP_FILE)"; \
		echo "Run 'make build' first or specify the correct ARCH/VERSION"; \
		exit 1; \
	fi
	@echo "Published to candidate channel"

publish-stable:
	@echo "⚠️  WARNING: Publishing to stable channel!"
	@printf "Are you sure? [y/N] "; \
	read REPLY; \
	case "$$REPLY" in \
		[Yy]*) \
			if [ -f "../../$(SNAP_FILE)" ]; then \
				snapcraft upload --release=stable "../../$(SNAP_FILE)"; \
				echo "Published to stable channel"; \
			else \
				echo "Snap file not found: $(SNAP_FILE)"; \
				echo "Run 'make build' first or specify the correct ARCH/VERSION"; \
				exit 1; \
			fi ;; \
		*) \
			echo "Cancelled" ;; \
	esac

exportlogin:
	snapcraft export-login --snaps=uad-shizuku --acls package_access,package_push,package_update,package_release exported.txt
