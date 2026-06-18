# Tab Migration Guide

This guide shows how to migrate tab operations from direct ADB/threading to the ViewModel pattern.

## Prerequisites

✅ ViewModel infrastructure complete (Tasks 1-9)
✅ All 4 actors implemented and operational  
✅ SetOptions command added to DebloatActor

## Example: Batch Uninstall Migration

### Before (Current Implementation)

```rust
// tab_debloat_control.rs:115-250
pub fn start_batch_uninstall(&mut self, pkgs: Vec<String>, ...) {
    self.batch_uninstall_state.start();
    
    // Clone data for background thread
    let progress_clone = self.batch_uninstall_progress.clone();
    let device = device.clone();
    
    // Spawn blocking thread
    std::thread::spawn(move || {
        let total = pkgs.len();
        for (i, pkg) in pkgs.iter().enumerate() {
            // Direct ADB call
            match crate::adb::uninstall_app(&pkg, &device) {
                Ok(_) => {
                    // Update SharedStore
                    let store = get_shared_store();
                    let mut packages = store.get_installed_packages();
                    packages.retain(|p| p.pkg != pkg);
                    store.set_installed_packages(packages);
                }
                Err(e) => log::error!("Failed: {}", e),
            }
            
            // Manual progress tracking with Arc<Mutex<>>
            if let Ok(mut p) = progress_clone.lock() {
                *p = Some(i as f32 / total as f32);
            }
            let store = get_shared_store();
            store.request_repaint();
        }
    });
}
```

### After (ViewModel Pattern)

```rust
// Modified signature - needs access to app.viewmodel
pub fn start_batch_uninstall(
    &mut self,
    app: &mut UadShizukuApp,
    pkgs: Vec<String>,
    device: String,
) {
    // Configure actor options first
    if let Some(ref vm) = app.viewmodel {
        let _ = vm.set_debloat_options(
            self.unsafe_app_remove,
            self.expert_app_remove
        );
        
        // Send async command - actor handles everything
        if let Err(e) = vm.batch_uninstall(pkgs, device) {
            log::error!("Failed to start batch uninstall: {}", e);
            return;
        }
        
        // State machine will be updated via events
        self.batch_uninstall_state.start();
    }
}
```

### Event Handling in Tab Update

```rust
// Add to tab's render/update function
pub fn handle_viewmodel_events(&mut self, app: &mut UadShizukuApp) {
    if let Some(ref mut vm) = app.viewmodel {
        let events = vm.poll_events(/* ctx if needed */);
        
        for event in events {
            if let ViewModelEvent::Debloat(debloat_event) = event {
                self.handle_debloat_event(debloat_event);
            }
        }
    }
}

fn handle_debloat_event(&mut self, event: DebloatEvent) {
    match event {
        DebloatEvent::BatchProgress { operation, progress, current, total } => {
            if operation == "uninstall" {
                self.batch_uninstall_state.progress = Some(progress);
                log::debug!("Uninstall progress: {}/{}", current, total);
            }
        }
        
        DebloatEvent::BatchComplete { operation, succeeded, failed } => {
            if operation == "uninstall" {
                self.batch_uninstall_state.complete();
                log::info!("Batch uninstall complete: {} succeeded, {} failed", 
                           succeeded, failed);
            }
        }
        
        DebloatEvent::Error { operation, error } => {
            log::error!("Debloat error in {}: {}", operation, error);
            if operation == "uninstall" {
                self.batch_uninstall_state.error();
            }
        }
        
        DebloatEvent::PackagesLoaded(packages) => {
            // Update local packages state
            // TODO: Eventually replaces SharedStore.get_installed_packages()
        }
        
        _ => {}
    }
}
```

### Progress Display

```rust
// In UI rendering code
if let Some(progress) = self.batch_uninstall_state.progress {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(format!("Uninstalling... {:.0}%", progress * 100.0));
    });
}
```

## Migration Checklist

For each operation to migrate:

1. **Identify the operation**
   - Find thread spawn or direct ADB call
   - Note what state it modifies
   - Identify progress tracking mechanism

2. **Use ViewModel command**
   - Replace `std::thread::spawn` with `vm.command()`
   - Remove Arc<Mutex<>> progress tracking
   - Keep state machine for UI state

3. **Handle events**
   - Add event polling in tab update
   - Match relevant events for the operation
   - Update state machine from events

4. **Remove SharedStore calls**
   - After all operations migrated
   - Use ViewModel state instead
   - Update tests

5. **Verify**
   - Test operation works
   - Check progress updates
   - Verify error handling
   - Test cancellation if applicable

## Common Patterns

### Single Operation (Enable/Disable)

```rust
// Before
match crate::adb::enable_app(&pkg, &device) {
    Ok(_) => {
        let store = get_shared_store();
        // ... update store
    }
    Err(e) => log::error!("{}", e),
}

// After
if let Some(ref vm) = app.viewmodel {
    vm.batch_enable(vec![pkg.clone()], device.clone())?;
}
// Result comes via BatchComplete event
```

### Progress Display

```rust
// Before
if let Ok(p) = self.progress.lock() {
    if let Some(val) = *p {
        ui.label(format!("{:.0}%", val * 100.0));
    }
}

// After  
if let Some(ref vm) = app.viewmodel {
    if let Some(progress) = vm.operation_progress("uninstall") {
        ui.label(format!("{:.0}%", progress.progress * 100.0));
    }
}
```

### State Machine Integration

```rust
// State machine stays in tab for UI state
self.batch_uninstall_state.start();  // UI shows "running"

// Events update state machine
match event {
    DebloatEvent::BatchComplete { .. } => {
        self.batch_uninstall_state.complete();  // UI shows "done"
    }
    DebloatEvent::Error { .. } => {
        self.batch_uninstall_state.error();  // UI shows error
    }
    _ => {}
}
```

## Benefits

- ✅ **Non-blocking UI**: Operations run on background thread
- ✅ **Centralized logic**: Business logic in actors, not spread across tabs
- ✅ **Consistent patterns**: All operations use same command/event pattern
- ✅ **Testable**: Actors can be unit tested independently
- ✅ **Progress tracking**: Built into ViewModel, no manual Arc<Mutex<>>
- ✅ **Error handling**: Centralized in actors with events
- ✅ **Prepares for SharedStore removal**: State moves to ViewModel

## Challenges

⚠️ **Async UI updates**: Tabs must poll events in update loop  
⚠️ **Function signatures change**: Need access to app.viewmodel  
⚠️ **State coordination**: UI state machine + ViewModel events  
⚠️ **Cancellation**: Not yet implemented in actors (TODO)

## Next Steps

1. Complete batch uninstall migration in debloat tab
2. Add event handling to tab update function
3. Test operation works end-to-end
4. Migrate remaining operations (disable, enable)
5. Repeat for scan and apps tabs
6. Remove SharedStore after all tabs migrated
7. Remove old threading code
8. Final testing and merge
