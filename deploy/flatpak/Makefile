# Makefile for building UAD-Shizuku Flatpak
APP_ID = pe.nikescar.uad_shizuku
MANIFEST = $(APP_ID).yml
BRANCH = master
REPO_DIR = repo
BUILD_DIR = .flatpak-builder
EXPORT_DIR = export

.PHONY: all prepare-repo install-deps build clean-cache update-repo copy-to-export clean

all: prepare-repo install-deps build clean-cache update-repo copy-to-export

prepare-repo:
	@echo "Preparing repository..."
	[[ -d $(REPO_DIR) ]] || ostree init --mode=archive-z2 --repo=$(REPO_DIR)
	[[ -d $(REPO_DIR)/refs/remotes ]] || mkdir -p $(REPO_DIR)/refs/remotes && touch $(REPO_DIR)/refs/remotes/.gitkeep

install-deps:
	@echo "Installing dependencies..."
	flatpak --user remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
	flatpak --user install -y flathub \
		org.freedesktop.Platform/x86_64/24.08 \
		org.freedesktop.Sdk/x86_64/24.08
	flatpak --user install -y flathub \
		org.freedesktop.Sdk.Extension.rust-stable/x86_64/24.08

build:
	@echo "Building UAD-Shizuku Flatpak..."
	flatpak-builder --force-clean --ccache --require-changes --repo=$(REPO_DIR) \
		--subject="Build of UAD-Shizuku, `date`" \
		--allow-missing-runtimes \
		${EXPORT_ARGS} app $(MANIFEST)

clean-cache:
	@echo "Cleaning build cache..."
	rm -rf $(BUILD_DIR)/build

update-repo:
	@echo "Updating repository..."
	flatpak build-update-repo --prune --prune-depth=20 $(REPO_DIR)
	echo 'gpg-verify-summary=false' >> $(REPO_DIR)/config
	rm -rf $(REPO_DIR)/.lock

copy-to-export:
	@echo "Copying to export directory..."
	rm -rf $(EXPORT_DIR) && mkdir $(EXPORT_DIR)
	cp -rf $(REPO_DIR)/ $(EXPORT_DIR)/

clean:
	@echo "Cleaning all build artifacts..."
	rm -rf $(BUILD_DIR) $(REPO_DIR) $(EXPORT_DIR) app

test:
	@echo "Testing Flatpak installation..."
	flatpak --user install --reinstall -y $(REPO_DIR) $(APP_ID)
	
run:
	@echo "Running UAD-Shizuku..."
	flatpak --user run $(APP_ID)

uninstall:
	@echo "Uninstalling UAD-Shizuku..."
	flatpak --user uninstall -y $(APP_ID)

help:
	@echo "Available targets:"
	@echo "  all          - Build complete Flatpak package"
	@echo "  build        - Build the Flatpak application"
	@echo "  clean        - Clean all build artifacts"
	@echo "  test         - Install and test the built Flatpak"
	@echo "  run          - Run the installed Flatpak"
	@echo "  uninstall    - Uninstall the Flatpak"
	@echo "  help         - Show this help message"

	