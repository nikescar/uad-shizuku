# App Name/Icon Rendering Fix Design

**Date:** 2026-08-16  
**Author:** Claude Sonnet 4.5  
**Status:** Design Approved

## Overview

Fix app name and icon rendering in debloat and scan tables when Google Play, F-Droid, or APKMirror renderers are enabled. Currently, users see only package IDs (e.g., `com.android.vending`) instead of app titles and icons, despite metadata being present in the database.

## Problem Statement

### Symptoms
- Renderer settings enabled: Google Play (GP: true), F-Droid (FD: true)
- Metadata exists in database: 68 entries found
- **But:** UI shows only package IDs, no app titles or icons
- Affects both desktop and mobile views
- Affects both debloat and scan tables

### Logs Analysis
```
[INFO] uad_shizuku::app_metadata_renderer: [RENDER] prepare_app_info_for_display completed: 112 packages, 0 textures loaded
[INFO] uad_shizuku::app_metadata_renderer: [RENDER] Metadata sources: 0 from ViewModel cache, 0 from SharedStore, 68 total metadata entries
[INFO] uad_shizuku::tab_debloat::view_mobile: [DEBLOAT] Got metadata for 68 packages
[INFO] uad_shizuku::dlg_mobile_list: [MOBILE_LIST] Renderer flags - GP: true, FD: true, APK: false, AP: false
```

**Key observation:** 68 metadata entries returned, but HashMap lookup in rendering code returns `None` for all packages.

## Root Cause Analysis

### The Disconnect

**Data flow:**
1. `prepare_app_info_for_display()` queries database → finds 68 entries
2. Returns `HashMap<String, (Option<TextureHandle>, String, String, Option<String>)>` with 68 entries
3. `render_package_list()` converts to `HashMap<String, (Option<TextureHandle>, String)>` → still 68 entries
4. `render_package_table_mobile()` calls `app_display_data.get(&package.pkg)` → returns `None` for ALL packages

**Hypothesis:** Package ID mismatch between HashMap keys (from database) and lookup keys (from `filtered_packages`).

### Possible Causes

1. **Case mismatch:** Database stores `"com.android.vending"` but packages use `"COM.ANDROID.VENDING"` or vice versa
2. **Whitespace:** Extra spaces or newlines in database package IDs: `"com.android.vending "`
3. **User filtering:** Database has metadata for user 0 packages only, but `filtered_packages` includes all users
4. **Empty titles:** Titles exist but are empty strings `""` (less likely, would still show package ID)

### Architecture Context

Current rendering pipeline:
```
Database (fdroid_apps, google_play_apps, apkmirror_apps)
    ↓
app_metadata_renderer::prepare_app_info_for_display()
    ↓ (68 entries)
HashMap<String, (texture, title, developer, version)>
    ↓ (convert to mobile format)
HashMap<String, (texture, title)>
    ↓ (lookup by package.pkg)
render_package_table_mobile() → LOOKUP FAILS
```

**Note:** ViewModel cache exists (`vm_state.cached_metadata`) but is currently empty. Metadata fetchers write only to database, not to ViewModel cache. This is a known architectural gap from the MVVM migration, but fixing it is out of scope for this bug fix (would be Approach A from brainstorming).

## Design

### Strategy: Debug-First Approach

**Philosophy:** Add targeted logging to identify the exact mismatch, then apply minimal fix.

**Rationale:** 
- Avoids guessing and over-engineering
- Logs will show us the exact pattern (case? whitespace? user filtering?)
- Fix will be surgical and low-risk
- Preserves current database-first architecture

### Phase 1: Enhanced Logging

**Location:** `mobile/src/tab_debloat/view_mobile.rs`, in `render_package_list()` function, after line 163 (after `app_metadata` is populated).

**Logs to add:**

1. **Sample HashMap keys** (first 5):
   ```rust
   let sample_keys: Vec<_> = app_metadata.keys().take(5).cloned().collect();
   log::info!("[DEBLOAT] Sample app_metadata keys: {:?}", sample_keys);
   ```

2. **Sample package IDs from filtered list** (first 5):
   ```rust
   let sample_packages: Vec<_> = vm_state.filtered_packages.iter()
       .take(5)
       .map(|p| p.pkg.clone())
       .collect();
   log::info!("[DEBLOAT] Sample filtered package IDs: {:?}", sample_packages);
   ```

3. **Mismatch analysis** - First 5 packages not in HashMap:
   ```rust
   let missing: Vec<_> = vm_state.filtered_packages.iter()
       .filter(|p| !app_metadata.contains_key(&p.pkg))
       .take(5)
       .map(|p| p.pkg.clone())
       .collect();
   log::warn!("[DEBLOAT] First 5 packages missing from metadata: {:?}", missing);
   ```

4. **Success rate** - How many packages have metadata:
   ```rust
   let found_count = vm_state.filtered_packages.iter()
       .filter(|p| app_metadata.contains_key(&p.pkg))
       .count();
   log::info!(
       "[DEBLOAT] Rendering metrics - Total: {}, With metadata: {}, Hit rate: {:.1}%",
       vm_state.filtered_packages.len(),
       found_count,
       (found_count as f32 / vm_state.filtered_packages.len() as f32) * 100.0
   );
   ```

**Expected output:** Logs will show:
- Are package IDs identical in format?
- Is it a case sensitivity issue? (e.g., `com.android.vending` vs `COM.ANDROID.VENDING`)
- Are ALL packages missing or just some?
- Pattern in which packages have metadata vs which don't

### Phase 2: Apply Fix (Based on Debug Findings)

Once logs identify the mismatch pattern, apply the appropriate fix:

#### Fix A: Case Normalization
**Trigger:** If keys are `"com.android.vending"` but lookups use `"COM.ANDROID.VENDING"`

**Solution:** Normalize to lowercase in both HashMap insertion and lookup.

**Files:**
- `mobile/src/app_metadata_renderer.rs`, line 215:
  ```rust
  app_data_map.insert(pkg_id.to_lowercase(), (texture, title, developer, version));
  ```
- `mobile/src/tab_debloat/view_mobile.rs`, line 158 (map function):
  ```rust
  .map(|(pkg_id, (texture, title, _developer, _version))| {
      (pkg_id.to_lowercase(), (texture.clone(), title.clone()))
  })
  ```

**Trade-off:** Adds minimal overhead (`.to_lowercase()` on package IDs), but ensures case-insensitive matching.

#### Fix B: Whitespace Trimming
**Trigger:** If database has `"com.android.vending "` (trailing space) but packages are `"com.android.vending"`

**Solution:** Trim package IDs when inserting into HashMap.

**Files:**
- `mobile/src/app_metadata_renderer.rs`, line 215:
  ```rust
  app_data_map.insert(pkg_id.trim().to_string(), (texture, title, developer, version));
  ```

**Trade-off:** Minimal overhead, defensive against data quality issues.

#### Fix C: Empty Title Handling
**Trigger:** If titles exist but are empty strings `""`

**Solution:** Treat empty titles as `None`, fall back to package ID.

**Files:**
- `mobile/src/tab_debloat/components/package_table_mobile.rs`, lines 95-99:
  ```rust
  if let Some(title) = app_title.filter(|t| !t.is_empty()) {
      ui.label(egui::RichText::new(title).strong());
      ui.label(egui::RichText::new(&package.pkg).small().weak());
  } else {
      ui.label(&package.pkg);
  }
  ```

**Trade-off:** Better UX (show package ID instead of blank), no performance impact.

#### Fix D: User Filtering Mismatch
**Trigger:** If database has user 0 packages only, but `filtered_packages` includes all users

**Solution:** Either fetch metadata for all users, or filter packages by user before rendering.

**Complexity:** Higher - requires changes to metadata fetching logic in `DebloatActor` or database queries.

**Defer:** If this is the cause, implement Fix A or B as a workaround (normalize IDs), then file separate issue for proper user filtering.

**Most likely fix:** Fix A (case normalization) or Fix B (whitespace trimming), based on Android package ID conventions (always lowercase, no spaces).

### Phase 3: Validation

**Success criteria:**
1. Hit rate >95% for packages with metadata in database
2. Users see app titles (bold) + package IDs (small gray) for packages with metadata
3. No performance regression: rendering still <300ms for 1000+ packages

**Metrics to track:**
```rust
log::info!(
    "[DEBLOAT] Rendering metrics - Total: {}, With metadata: {}, Hit rate: {:.1}%",
    total_packages,
    found_count,
    hit_rate_percentage
);
```

**Graceful degradation:**
- Packages without metadata → show package ID only (current fallback behavior)
- Empty titles → show package ID only (Fix C)
- Failed texture load → show title without icon (already handled)

### Phase 4: Cleanup (Optional)

Once fix is confirmed working, debug logs can be:
- **Option A:** Kept at `debug!` level (disabled in release builds)
- **Option B:** Promoted to `info!` level for ongoing monitoring (recommended)
- **Option C:** Removed entirely if not needed

**Recommendation:** Keep as `debug!` level - useful for future debugging, no runtime cost in release builds.

## Testing Strategy

### Manual Testing Checklist

#### Desktop View
- [ ] Enable Google Play renderer → titles/icons appear in debloat table
- [ ] Enable F-Droid renderer → titles/icons appear in debloat table
- [ ] Enable APKMirror renderer → titles/icons appear (system apps only)
- [ ] Disable all renderers → only package IDs show
- [ ] Mixed metadata: some packages have DB entries, some don't → correct rendering for each

#### Mobile View
- [ ] Same tests as desktop, in mobile dialog (narrow screen <800px)
- [ ] Touch targets still work (48px minimum per component docs)
- [ ] Scrolling performance: 100+ packages render smoothly

#### Scan Table
- [ ] Same renderer settings apply to scan table
- [ ] Consistency: debloat and scan views show same titles/icons for same packages

### Test Data Scenarios

1. **Mixed metadata:** Some packages have database entries, some don't
   - Expected: Packages with metadata show titles, others show IDs only

2. **Empty database:** Clear database, fetch fresh metadata
   - Expected: All show package IDs initially, populate as metadata fetches complete

3. **Partial metadata:** Database has title but no icon (`icon_base64` is `None`)
   - Expected: Show title without icon (text-only rendering)

### Verification Commands

```bash
# Check database has metadata
sqlite3 ~/.config/uad_shizuku/dbs/uad_shizuku.db \
  "SELECT package_id, title, length(icon_base64) FROM fdroid_apps LIMIT 5;"

# Expected output:
# com.android.vending|Google Play Store|12345
# com.google.android.gms|Google Play Services|23456

# Run with debug logging
RUST_LOG=debug cargo run 2>&1 | grep "\[DEBLOAT\]\|\[RENDER\]"

# Expected output:
# [DEBLOAT] Sample app_metadata keys: ["com.android.vending", "com.google.android.gms", ...]
# [DEBLOAT] Sample filtered package IDs: ["com.android.vending", "com.google.android.gms", ...]
# [DEBLOAT] Rendering metrics - Total: 112, With metadata: 68, Hit rate: 60.7%
```

### Performance Benchmark

```bash
# Before fix: Measure render time
# (Look for "[RENDER] prepare_app_info_for_display completed" log line)

# After fix: Ensure render time doesn't increase >10%
# Target: <300ms for 1000+ packages (per component docs)
```

**Acceptance criteria:**
- Render time increase <10% (e.g., 80ms → 88ms acceptable, 80ms → 160ms not acceptable)
- No memory leaks (run for 10 minutes, check memory usage stable)

## Error Handling

### Edge Cases

1. **Unicode package IDs:** Rare but possible (e.g., internationalized domain names)
   - Fix A (case normalization): Use `.to_lowercase()` which handles Unicode correctly
   - Fix B (whitespace trimming): `.trim()` handles Unicode whitespace

2. **Very long titles:** Truncate with ellipsis in mobile view
   - Already handled by egui text layout (automatic wrapping/clipping)

3. **Null/None titles:** Shouldn't happen (title is `String` not `Option<String>` in database model)
   - Defensive: Fix C handles empty strings as fallback

### Rollback Plan

If fix causes crashes or performance issues:
1. Debug logs can be disabled immediately (change `log::info!` to `log::debug!`)
2. Original behavior (package ID only) is the safe fallback (no metadata rendering)
3. Revert commit, file issue with logs for further investigation

**Risk:** Low - changes are minimal (logging + normalization), no architectural changes.

## Files to Modify

### Primary Changes

1. **`mobile/src/tab_debloat/view_mobile.rs`**
   - Add debug logs (4 log statements, ~15 lines)
   - Possibly add normalization in HashMap conversion (if Fix A needed)

2. **`mobile/src/app_metadata_renderer.rs`**
   - Add normalization in HashMap insertion (if Fix A needed, 1 line change)

3. **`mobile/src/tab_debloat/components/package_table_mobile.rs`**
   - Fix empty title handling (if Fix C needed, 1 line change)

### Secondary Changes (Apply Same Fix)

4. **`mobile/src/tab_scan_control.rs`**
   - Apply same fix to scan table view (consistency)
   - Similar changes to debloat tab

**Estimated diff:** 50-100 lines of logging + 5-10 lines for actual fix = ~60-110 lines total

## Implementation Plan Summary

### Phase 1: Debug Logging (30 minutes)
- Add 4 log statements to `view_mobile.rs`
- Run app, reproduce issue, collect logs
- Analyze logs to identify mismatch pattern
- **Deliverable:** Log output showing exact package ID mismatch

### Phase 2: Apply Fix (30-60 minutes)
- Based on Phase 1 findings, apply Fix A, B, C, or D
- Most likely: case normalization (Fix A) or whitespace trimming (Fix B)
- Update both HashMap insertion and lookup points
- Apply same fix to scan table for consistency
- **Deliverable:** Package IDs match, titles display in UI

### Phase 3: Validation (30 minutes)
- Run manual testing checklist (desktop + mobile + scan)
- Check metrics: hit rate >95%, render time <300ms
- Verify edge cases: empty titles, missing metadata, Unicode IDs
- **Deliverable:** All tests pass, no performance regression

### Phase 4: Cleanup (15 minutes)
- Decide on log level (keep as `debug!` recommended)
- Update comments if normalization logic is non-obvious
- Commit with clear message explaining fix
- **Deliverable:** Clean, production-ready code

**Total estimated time:** 2-2.5 hours

## Success Metrics

### Before Fix
- Hit rate: 0% (all packages show ID only)
- User experience: Poor (can't identify apps by name)
- Metadata in database: 68 entries (unused)

### After Fix
- Hit rate: >95% (for packages with metadata in DB)
- User experience: Good (app titles + icons visible)
- Performance: <10% render time increase
- No crashes or regressions

## Future Work (Out of Scope)

### Icon Rendering
**Issue:** `icon_base64` field in database is `None` for all entries (0 textures loaded).

**Root cause:** Metadata fetchers are running but not populating `icon_base64` field.

**Solution:** Update `calc_fdroid`, `calc_googleplay`, `calc_apkmirror` modules to fetch and store icon data.

**Effort:** Medium (3-5 hours) - requires API changes, base64 encoding, database updates.

**Defer:** This fix focuses on titles. Icons can be addressed in separate PR.

### ViewModel Cache Population
**Issue:** ViewModel cache (`vm_state.cached_metadata`) is always empty. Metadata fetchers write only to database.

**Root cause:** Incomplete MVVM migration. Metadata actor doesn't populate ViewModel cache.

**Solution:** Update `MetadataActor` to emit `MetadataEvent::Fetched` with app data, ViewModel updates cache.

**Effort:** High (6-8 hours) - requires actor changes, event handling, cache synchronization.

**Defer:** This is architectural improvement (Approach A from brainstorming). Current fix (Approach B) is faster and lower risk.

## Conclusion

This design provides a debug-first approach to fix app name rendering:
1. Add targeted logging to identify exact mismatch pattern
2. Apply minimal fix (likely case normalization or whitespace trimming)
3. Validate with comprehensive testing
4. Keep debug logs for future troubleshooting

**Complexity:** Small (~60-110 lines changed)  
**Risk:** Low (no architectural changes)  
**Estimated time:** 2-2.5 hours  
**Impact:** High (fixes major UX issue where apps can't be identified by name)
