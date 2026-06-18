# Next Session Handoff: SharedStore Migration & TDD

## Current Status

**Branch:** `refactor/mvvm-actor-architecture` (22 commits)  
**Build:** ✅ Successful  
**Core MVVM Migration:** ✅ Complete (Tasks 1-12, 15)  
**Remaining:** Tasks 13-14 (SharedStore removal, legacy cleanup)

## Your Mission

Complete the MVVM migration by:
1. **Testing** existing implementation (Tasks #24-25)
2. **Migrating** SharedStore incrementally with TDD (Tasks #26-30)
3. **Cleaning up** legacy code (Tasks #31-34)

## Quick Start

```bash
# Already on correct branch
git status  # Should show refactor/mvvm-actor-architecture

# See all tasks
/tasks

# Start with planning
Use brainstorming skill for Task #35
```

## Context Documents

**MUST READ before starting:**
- `docs/mvvm-migration-complete.md` - Full migration report
- `docs/mvvm-refactor-final-summary.md` - Progress summary
- `docs/tab-migration-guide.md` - Migration patterns

**Reference:**
- `mobile/src/viewmodel/` - Current ViewModel implementation
- `mobile/src/shared_store_stt.rs` - What needs to be migrated

## Task Breakdown

### Phase 1: Integration Testing (Tasks #24-25)
Test current implementation with real devices before making changes.

**Task #24:** Debloat tab testing
- Batch uninstall, disable, enable
- Event handling verification
- Progress tracking

**Task #25:** Scan tab testing  
- VirusTotal/HybridAnalysis scans
- Event handling verification
- Cancellation testing

### Phase 2: SharedStore Migration (Tasks #26-30)

**Use TDD for each task:**
1. Brainstorm design options
2. Write tests first
3. Implement incrementally
4. Verify with integration tests

**Task #26 (13.1):** Move packages to ViewModel
- Files: `viewmodel/mod.rs`, `viewmodel/debloat.rs`, tabs
- Remove: SharedStore.installed_packages
- Estimated: ~150 lines

**Task #27 (13.2):** Move scanner states to ViewModel events
- Files: `viewmodel/scan.rs`, `tab_scan_control.rs`
- Remove: SharedStore.vt_scanner_state, ha_scanner_state
- Estimated: ~100 lines

**Task #28 (13.3):** Migrate cached app metadata
- Decide: ViewModel vs local cache vs separate layer
- Files: Depends on approach chosen
- Estimated: ~150 lines

**Task #29 (13.4):** Migrate texture caches
- Textures are UI resources (egui::TextureHandle)
- Consider: Tab-local storage vs TextureCache wrapper
- Estimated: ~100 lines

**Task #30 (13.5):** Delete SharedStore files
- Delete: `shared_store.rs`, `shared_store_stt.rs`
- Update: `lib.rs`, imports in 15+ files
- Estimated: ~200 deletions, ~50 changes

### Phase 3: Legacy Code Removal (Tasks #31-34)

**After SharedStore migration complete:**

**Task #31 (14.1):** Debloat tab cleanup
- Remove fallback implementations
- Remove Arc<Mutex<>> progress tracking
- Make ViewModel required (not Option)
- Estimated: ~150 deletions

**Task #32 (14.2):** Scan tab cleanup
- Remove fallback implementations
- Remove legacy progress tracking
- Make ViewModel required
- Estimated: ~150 deletions

**Task #33 (14.3):** Apps tab cleanup
- Make ViewModel required
- Remove remaining legacy code
- Estimated: ~50 changes

**Task #34 (14.4):** Final verification
- Clean up imports and comments
- Run clippy
- Integration tests
- Update documentation

## TDD Approach

For each migration task:

```rust
// 1. Write test first
#[test]
fn test_packages_loaded_from_viewmodel() {
    let vm = ViewModel::new(ctx);
    let packages = vec![/* test data */];
    
    // Test that packages are accessible from ViewModel
    // not from SharedStore
}

// 2. Implement to make test pass
// 3. Refactor while keeping tests green
// 4. Integration test with real UI
```

## Key Design Questions

### Task #26 (Packages Migration)

**Question:** How do actors get packages?
- Option A: Pass in command (bulky, inefficient)
- Option B: Actors share ViewModel state via Arc
- Option C: Actors read from shared cache layer

**Recommend:** Discuss in brainstorming session

### Task #28 (Cached Metadata)

**Question:** Where should cached app info live?
- Option A: ViewModel state (centralized)
- Option B: Tab-local storage (isolated)
- Option C: Separate CacheService (clean separation)

**Recommend:** Consider access patterns and testing

### Task #29 (Texture Caches)

**Question:** How to handle egui::TextureHandle?
- Option A: Keep minimal SharedStore just for textures
- Option B: TextureCache wrapper service
- Option C: Tab-local texture management

**Recommend:** Consider egui lifetime requirements

## Expected Outcomes

After completing all tasks:

✅ **Zero SharedStore usage** in actors and tabs  
✅ **No legacy fallback code**  
✅ **ViewModel is source of truth**  
✅ **All tests passing**  
✅ **Clean architecture** with clear separation  
✅ **Ready to merge** to main

## Potential Challenges

1. **Circular dependencies**: ViewModel → Actor → ViewModel access
2. **Texture lifetimes**: egui::TextureHandle tied to Context
3. **Performance**: Avoid copying large datasets
4. **Thread safety**: Actors run on background thread
5. **Backward compatibility**: Don't break existing functionality

## Success Criteria

- [ ] All integration tests pass (Tasks #24-25)
- [ ] SharedStore completely removed
- [ ] No Arc<Mutex<>> progress tracking
- [ ] ViewModel is source of truth for all state
- [ ] Build successful with zero errors
- [ ] Performance not regressed
- [ ] Documentation updated

## Git Strategy

```bash
# Work on same branch
git checkout refactor/mvvm-actor-architecture

# Commit after each subtask
git commit -m "feat: Task 13.1 - move packages to ViewModel"
git commit -m "feat: Task 13.2 - move scanner states to events"
# etc.

# Final commit
git commit -m "feat: complete SharedStore migration and legacy cleanup"
```

## Commands for New Session

```bash
# Check status
git status
git log --oneline -10

# See tasks
/tasks

# Start brainstorming
/brainstorming Plan SharedStore migration with TDD approach

# Read context
cat docs/mvvm-migration-complete.md
cat mobile/src/shared_store_stt.rs
cat mobile/src/viewmodel/mod.rs
```

## Notes from Previous Session

- ViewModel uses smol async runtime ✅
- All tabs accept ViewModel parameter ✅
- Event-driven pattern established ✅
- Backward compatible fallbacks in place ✅
- Build verified successful ✅

**Key insight:** SharedStore deeply integrated - needs careful incremental migration, not big-bang replacement.

## Questions to Answer in Brainstorming

1. What's the best way for actors to access packages?
2. Should cached metadata live in ViewModel or separate layer?
3. How to handle texture caches (egui lifetime constraints)?
4. Can we avoid breaking changes during migration?
5. What's the testing strategy for each phase?

## Ready to Begin

Start with Task #35: "Plan and implement SharedStore migration with TDD"

Use brainstorming skill to design the approach, then use TDD skill for implementation.

Good luck! 🚀
