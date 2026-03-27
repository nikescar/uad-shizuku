# UAD-Shizuku Flatpak

Build and test UAD-Shizuku as a Flatpak package.

## Prerequisites

- Docker (for containerized builds)
- Flatpak and flatpak-builder (for local builds)
- Built UAD-Shizuku binary in `../../artifacts/uad-shizuku-x86_64-unknown-linux-musl/`

## Building with Docker

```bash
$ docker build -t uad-shizuku-flatpak .
$ docker run -it -v /home/wj/work/uad-shizuku:/home/builder/uad-shizuku-source uad-shizuku-flatpak bash
$ make all
```

## Building Locally

```bash
$ make all          # Build complete Flatpak package
$ make test         # Install and test the built Flatpak
$ make run          # Run the installed Flatpak
$ make uninstall    # Uninstall the Flatpak
$ make clean        # Clean all build artifacts
$ make help         # Show all available targets
```

## Notes

- The app ID is `pe.nikescar.uad_shizuku`
- Requires `org.freedesktop.Platform` and `org.freedesktop.Sdk` version 24.08
- Binary must be built before creating the Flatpak package