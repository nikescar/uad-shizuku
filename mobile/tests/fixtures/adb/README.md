# Mock ADB fixtures

This directory holds real `adb`/`pm`/`dumpsys` output captured from a
physical device (Pixel 8a, Android 16, arm64-v8a), for use by
`mobile/src/bin/mock_adb.rs` — a fake `adb` binary that lets the rest of the
app and its tests exercise `mobile/src/adb.rs` without a real device
attached.

The device's real serial number was replaced with the placeholder
`MOCKDEVICE01` in `devices_l.txt` (it did not otherwise appear in captured
shell output). Everything else here is unmodified real output.

## Files

| File | Real command captured |
|---|---|
| `devices_l.txt` | `adb devices -l` |
| `pm_list_users.txt` | `adb shell pm list users` |
| `pm_list_packages_f.txt` | `adb shell pm list packages -f` |
| `dumpsys_package_packages.txt` | `adb shell dumpsys package packages` (full system dump, ~1.9MB) |
| `dumpsys_package_io.heckel.ntfy.txt` | `adb shell dumpsys package io.heckel.ntfy` |
| `dumpsys_package_com.google.android.systemui.overlay.pixelbatteryhealthconfig.txt` | `adb shell dumpsys package <pkg>` for a `SYSTEM`-flagged overlay package |
| `dumpsys_package_not_found.txt` | `adb shell dumpsys package <uninstalled pkg>` |
| `getprop_abilist.txt` | `adb shell getprop ro.product.cpu.abilist` |
| `getprop_version.txt` | `adb shell getprop ro.build.version.release` |
| `find_sample.txt` | `adb shell find <codePath> -type f` (reference only — not read at runtime) |
| `sha256sum_sample.txt` | `adb shell sha256sum <apk path>` (reference only — not read at runtime) |

`find_sample.txt` and `sha256sum_sample.txt` are kept for documentation of
what was captured; `mock_adb` hardcodes their one real value (see
`NTFY_APK_PATH` / `NTFY_APK_SHA256` in `mock_adb.rs`) rather than reading
these files, since they only ever cover a single path.

## What's mocked vs. synthesized

- `dumpsys package <pkg>` returns the real captured dump for
  `io.heckel.ntfy` and the overlay package above, the "not found" fixture
  for the literal package name `com.mockadb.not.installed`, and otherwise
  **synthesizes** a well-formed (but fake) per-package dump for any other
  package name — so callers can query arbitrary test package names and still
  get parseable output.
- `sha256sum` returns the one real captured hash for the `io.heckel.ntfy`
  APK path, and a deterministic (but non-cryptographic) 64-hex-char
  stand-in hash for any other path.
- `pm uninstall` / `pm disable-user` / `pm enable` / `cmd package
  install-existing` / `adb install` / `adb uninstall` were **not** run
  against the real device (they mutate device state) — their responses are
  well-known real `adb`/`pm` output strings, not captures. The mock is
  stateless: calling these never changes what a later `dumpsys`/`pm list`
  call reports.
- `dumpsys usagestats -history` was not captured (the Usage tab is a stub
  per `CLAUDE.md`) — the mock returns a placeholder line.
- `adb pull` writes a small dummy file to the requested local destination
  (not real APK bytes) so callers that check for the file's existence
  succeed.

## Using the mock

1. Build it: `cargo build --bin mock_adb` (from the workspace root; output
   lands in `target/debug/mock_adb`).
2. Put a copy/symlink of it named `adb` (or `adb.exe` on Windows) in a
   directory, and prepend that directory to `PATH` so it's found before any
   real `adb`:

   ```bash
   mkdir -p /tmp/mock-adb-bin
   ln -sf "$(pwd)/target/debug/mock_adb" /tmp/mock-adb-bin/adb
   PATH="/tmp/mock-adb-bin:$PATH" cargo run -p uad-shizuku
   ```

   On Windows, copy (don't symlink) `mock_adb.exe` to `adb.exe`.
3. Use device serial `MOCKDEVICE01` (what `adb devices -l` reports) when a
   serial is needed.

`mobile/tests/integration/adb_mock_test.rs` does this automatically for its
own test process.
