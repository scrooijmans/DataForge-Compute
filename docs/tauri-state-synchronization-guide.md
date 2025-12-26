# Tauri State Synchronization: Frontend-Backend Communication

Based on Tauri 2.9.5 documentation and best practices for synchronizing state between frontend components and backend Rust logic, especially in multi-view/window applications.

## Overview

Tauri applications have three main communication patterns:

1. **Commands (Request-Response)**: Frontend calls backend functions
2. **Events (Pub-Sub)**: Backend pushes updates to frontend
3. **State Management**: Shared state in Rust backend

## 1. Command Pattern (Request-Response)

### Frontend to Backend: `invoke()`

The primary way frontend components call backend functions is through the `invoke()` API.

**Frontend (TypeScript/JavaScript)**:
```typescript
import { invoke } from '@tauri-apps/api/core';

// Simple command call
const result = await invoke<DataForgeStatus>('get_dataforge_status');

// Command with parameters
const wells = await invoke<WellInfo[]>('list_wells', { 
  workspaceId: id 
});

// Command with complex request object
const result = await invoke<ExecuteUdfResult>('execute_udf', {
  request: {
    udf_id: currentUdfId,
    well_id: currentWellId,
    workspace_id: currentWorkspaceId,
    parameters: currentParams,
    save_result: saveResult
  }
});
```

**Backend (Rust)**:
```rust
use tauri::State;

#[tauri::command]
pub fn get_dataforge_status(
    state: State<'_, Mutex<ComputeState>>
) -> DataForgeStatus {
    let state = state.lock().expect("Failed to lock state");
    // ... access state and return result
}

#[tauri::command]
pub fn list_wells(
    workspace_id: String,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<Vec<WellInfo>, String> {
    // ... implementation
}

#[tauri::command]
pub fn execute_udf(
    request: ExecuteUdfRequest,
    state: State<'_, Mutex<ComputeState>>,
    active_executions: State<'_, ActiveExecutions>,
) -> Result<ExecuteUdfResult, String> {
    // ... implementation
}
```

**Registration**:
```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        commands::get_dataforge_status,
        commands::list_wells,
        commands::execute_udf,
        // ... more commands
    ])
```

### Characteristics

- **Synchronous**: Frontend waits for response
- **Type-safe**: TypeScript types match Rust types (with proper serialization)
- **Error handling**: Rust `Result<T, E>` maps to TypeScript `Promise<T>` or throws
- **One-to-one**: One command call = one response

**Best Use Cases**:
- User-initiated actions (button clicks, form submissions)
- Data queries that need immediate results
- Operations that require confirmation before proceeding

**Example from your codebase** (`compute.ts`):
```typescript
export async function selectCurve(id: string) {
  selectedCurveId.set(id);
  curveData.set(null);
  executionResult.set(null);
  isLoading.set(true);

  try {
    const result = await invoke<CurveData>('get_curve_data', { curveId: id });
    curveData.set(result);
  } catch (e) {
    error.set(e instanceof Error ? e.message : String(e));
  } finally {
    isLoading.set(false);
  }
}
```

## 2. Event Pattern (Pub-Sub)

### Backend to Frontend: Event Emission

Tauri provides a powerful event system for backend-to-frontend communication, enabling real-time updates and multi-window synchronization.

### Emitting Events from Rust

**Basic Event Emission**:
```rust
use tauri::{Emitter, EventTarget, Manager};

// Emit to all windows
app.emit("data-updated", &payload)?;

// Emit to specific window
app.emit_to("main", "data-updated", &payload)?;

// Emit to all app-level listeners
app.emit_to(EventTarget::app(), "data-updated", &payload)?;

// Emit to any target (all windows, webviews, etc.)
app.emit_to(EventTarget::any(), "data-updated", &payload)?;

// Emit with filter
app.emit_filter("data-updated", &payload, |target| {
    // Custom filter logic
    target.label() == "main"
})?;
```

**From a Command**:
```rust
#[tauri::command]
pub fn save_data(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    // Perform save operation
    {
        let mut state = state.lock().unwrap();
        state.data = new_data;
    }
    
    // Notify all windows of the update
    app.emit("data-saved", &new_data)?;
    
    Ok(())
}
```

**From Window Context**:
```rust
// In a command that has access to Window
#[tauri::command]
pub fn update_progress(
    window: tauri::Window,
    progress: u8,
) -> Result<(), String> {
    window.emit("progress-update", progress)?;
    Ok(())
}
```

**Progress Updates Example**:
```rust
#[tauri::command]
pub fn download_file(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        for i in 1..100 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Emit progress to all listeners
            let _ = app.emit_to(
                EventTarget::any(), 
                "download-progress", 
                i
            );
        }
        let _ = app.emit("download-complete", ());
    });
    Ok(())
}
```

### Listening to Events on Frontend

**Basic Event Listener**:
```typescript
import { listen } from '@tauri-apps/api/event';

// Listen to an event
const unlisten = await listen('data-updated', (event) => {
  console.log('Data updated:', event.payload);
  // Update your stores/components
  dataStore.set(event.payload);
});

// Clean up when done
unlisten();
```

**In Svelte Components**:
```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { executionResult } from '$lib/stores/compute';
  
  let unlisten: (() => void) | null = null;
  
  onMount(async () => {
    // Listen for execution progress updates
    unlisten = await listen<number>('execution-progress', (event) => {
      console.log('Progress:', event.payload);
      // Update UI with progress
    });
  });
  
  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });
</script>
```

**Listen to Any Event**:
```typescript
import { listen } from '@tauri-apps/api/event';

// Listen to events from any source
const unlisten = await listen('global-update', (event) => {
  // This will receive events from any window/component
  handleGlobalUpdate(event.payload);
});
```

**Once-Only Listener**:
```typescript
import { once } from '@tauri-apps/api/event';

// Listen only once
await once('initialization-complete', (event) => {
  console.log('App initialized:', event.payload);
  // This listener automatically removes itself
});
```

### Multi-Window Event Synchronization

**Scenario**: Multiple windows need to stay in sync when data changes.

**Backend (Rust)**:
```rust
#[tauri::command]
pub fn update_shared_data(
    app: tauri::AppHandle,
    state: State<'_, Mutex<SharedState>>,
    new_data: Data,
) -> Result<(), String> {
    // Update shared state
    {
        let mut state = state.lock().unwrap();
        state.data = new_data.clone();
    }
    
    // Emit to ALL windows (any target)
    app.emit_to(EventTarget::any(), "shared-data-updated", &new_data)?;
    
    Ok(())
}
```

**Frontend - Window 1**:
```typescript
// Window 1 listens and updates its local state
await listen('shared-data-updated', (event) => {
  localDataStore.set(event.payload);
  // UI automatically updates via Svelte reactivity
});
```

**Frontend - Window 2**:
```typescript
// Window 2 also listens and stays in sync
await listen('shared-data-updated', (event) => {
  localDataStore.set(event.payload);
  // Both windows now have the same data
});
```

**Targeted Events**:
```rust
// Emit to specific window only
app.emit_to("settings-window", "settings-changed", &settings)?;

// Emit to all windows except the sender
app.emit_filter("data-updated", &data, |target| {
    target.label() != sender_window_label
})?;
```

## 3. State Management Pattern

### Backend State Management

Tauri provides a built-in state management system for sharing data across commands and threads.

**Defining State**:
```rust
use std::sync::{Arc, Mutex, RwLock};
use tauri::State;

// Simple state
pub struct ComputeState {
    pub dataforge_data_dir: Option<PathBuf>,
    pub db: Option<Connection>,
    pub registry: Option<Arc<UdfRegistry>>,
    pub engine: Option<ExecutionEngine>,
}

// Thread-safe collections
pub struct ActiveExecutions {
    executions: RwLock<HashMap<String, (Arc<CancellationToken>, Arc<ProgressState>)>>,
}
```

**Registering State**:
```rust
tauri::Builder::default()
    .manage(Mutex::new(ComputeState::default()))
    .manage(ActiveExecutions::default())
    .invoke_handler(tauri::generate_handler![
        // ... commands
    ])
```

**Accessing State in Commands**:
```rust
#[tauri::command]
pub fn get_status(
    state: State<'_, Mutex<ComputeState>>
) -> DataForgeStatus {
    let state = state.lock().expect("Failed to lock state");
    // Access state safely
    if state.db.is_none() {
        return DataForgeStatus { connected: false, .. };
    }
    // ...
}

#[tauri::command]
pub fn execute_udf(
    request: ExecuteUdfRequest,
    state: State<'_, Mutex<ComputeState>>,
    active_executions: State<'_, ActiveExecutions>,
) -> Result<ExecuteUdfResult, String> {
    // Access multiple state objects
    let state = state.lock().unwrap();
    let mut executions = active_executions.executions.write().unwrap();
    
    // Use state...
}
```

**State Access Patterns**:

1. **Mutex for Single Writer**:
   ```rust
   let state = state.lock().unwrap(); // Blocks until available
   // Use state
   ```

2. **RwLock for Multiple Readers**:
   ```rust
   // Multiple readers
   let executions = active_executions.executions.read().unwrap();
   // Single writer
   let mut executions = active_executions.executions.write().unwrap();
   ```

3. **Arc for Shared Ownership**:
   ```rust
   pub registry: Option<Arc<UdfRegistry>>, // Can be cloned and shared
   ```

### Frontend State Management

**Svelte Stores** (your current pattern):
```typescript
import { writable, derived } from 'svelte/store';

// Global stores
export const executionResult = writable<ExecuteUdfResult | null>(null);
export const selectedCurveId = writable<string | null>(null);

// Derived stores
export const selectedCurve = derived(
  [curves, selectedCurveId],
  ([$curves, $selectedCurveId]) => 
    $curves.find(c => c.id === $selectedCurveId)
);
```

**Bridging Tauri Events to Stores**:
```typescript
import { listen } from '@tauri-apps/api/event';
import { executionResult } from './stores/compute';

// Listen to backend events and update stores
export async function setupEventListeners() {
  await listen<ExecuteUdfResult>('execution-complete', (event) => {
    executionResult.set(event.payload);
  });
  
  await listen('data-refreshed', () => {
    // Refresh data by calling commands
    loadWorkspaces();
    loadWells();
  });
}
```

## 4. Hybrid Patterns for Multi-View Applications

### Pattern 1: Command + Event for Long-Running Operations

**Use Case**: Long-running operations that need progress updates.

**Backend**:
```rust
#[tauri::command]
pub fn execute_long_task(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();
    
    // Spawn async task
    std::thread::spawn(move || {
        for i in 0..100 {
            std::thread::sleep(Duration::from_millis(100));
            // Emit progress events
            let _ = app.emit_to(
                EventTarget::any(),
                "task-progress",
                (task_id.clone(), i)
            );
        }
        // Emit completion
        let _ = app.emit("task-complete", &task_id);
    });
    
    Ok(task_id)
}
```

**Frontend**:
```typescript
// Start task
const taskId = await invoke<string>('execute_long_task');

// Listen for progress
await listen<[string, number]>('task-progress', (event) => {
  const [id, progress] = event.payload;
  if (id === taskId) {
    progressStore.set(progress);
  }
});

// Listen for completion
await listen<string>('task-complete', (event) => {
  if (event.payload === taskId) {
    // Handle completion
  }
});
```

### Pattern 2: Shared Backend State + Events

**Use Case**: Multiple views need to react to shared state changes.

**Backend**:
```rust
#[tauri::command]
pub fn update_shared_resource(
    app: tauri::AppHandle,
    state: State<'_, Mutex<SharedState>>,
    resource_id: String,
    data: ResourceData,
) -> Result<(), String> {
    // Update shared state
    {
        let mut state = state.lock().unwrap();
        state.resources.insert(resource_id.clone(), data.clone());
    }
    
    // Notify all windows
    app.emit_to(
        EventTarget::any(),
        "resource-updated",
        (resource_id, data)
    )?;
    
    Ok(())
}
```

**Frontend - Multiple Views**:
```typescript
// View 1: Resource List
await listen<[string, ResourceData]>('resource-updated', (event) => {
  const [id, data] = event.payload;
  resourceListStore.update(list => {
    const index = list.findIndex(r => r.id === id);
    if (index >= 0) {
      list[index] = { id, ...data };
    }
    return list;
  });
});

// View 2: Resource Details
await listen<[string, ResourceData]>('resource-updated', (event) => {
  const [id, data] = event.payload;
  if (id === selectedResourceId) {
    resourceDetailsStore.set(data);
  }
});
```

### Pattern 3: Frontend Event Bus + Tauri Events

**Your Current Pattern**: Custom event bus for frontend coordination, Tauri commands for backend communication.

**Frontend Event Bus** (`events.ts`):
```typescript
// Frontend-only event bus for cross-component communication
export const eventBus = new DataEventBus();

export function emitCurveAdded(curveId: string) {
  eventBus.emit({ 
    dataType: 'curve', 
    itemId: curveId, 
    changeKind: 'added' 
  });
}
```

**Bridging to Tauri**:
```typescript
// When backend changes data, emit Tauri event
// Backend Rust:
app.emit("curve-added", &curve_id)?;

// Frontend TypeScript:
await listen<string>('curve-added', (event) => {
  // Update stores
  curves.update(list => [...list, newCurve]);
  
  // Also emit to frontend event bus for component coordination
  emitCurveAdded(event.payload);
});
```

## 5. Best Practices

### Avoiding Stale State

1. **Use Events for Real-Time Updates**:
   ```rust
   // ✅ Good: Emit event when state changes
   {
       let mut state = state.lock().unwrap();
       state.data = new_data;
   }
   app.emit("data-updated", &new_data)?;
   ```

2. **Refresh on Window Focus**:
   ```typescript
   import { getCurrentWindow } from '@tauri-apps/api/window';
   
   const window = getCurrentWindow();
   await window.onFocusChanged(async (focused) => {
     if (focused) {
       // Refresh data when window gains focus
       await loadStatus();
     }
   });
   ```

3. **Polling for Critical State** (if events aren't feasible):
   ```typescript
   // Only if events aren't possible
   setInterval(async () => {
     const status = await invoke<DataForgeStatus>('get_dataforge_status');
     statusStore.set(status);
   }, 5000);
   ```

### Avoiding Unnecessary Re-renders

1. **Debounce Event Handlers**:
   ```typescript
   import { debounce } from 'lodash-es';
   
   const debouncedUpdate = debounce((data) => {
     store.set(data);
   }, 300);
   
   await listen('frequent-updates', (event) => {
     debouncedUpdate(event.payload);
   });
   ```

2. **Filter Events by Relevance**:
   ```typescript
   await listen('data-updated', (event) => {
     // Only update if this view cares about this data
     if (event.payload.type === currentViewType) {
       store.set(event.payload);
     }
   });
   ```

3. **Use Derived Stores**:
   ```typescript
   // Only recalculates when dependencies change
   export const filteredData = derived(
     [allData, filter],
     ([$allData, $filter]) => 
       $allData.filter(item => matchesFilter(item, $filter))
   );
   ```

### Multi-Window Synchronization

1. **Emit to All Windows for Shared State**:
   ```rust
   // When shared state changes, notify all windows
   app.emit_to(EventTarget::any(), "shared-state-changed", &state)?;
   ```

2. **Use Window Labels for Targeted Updates**:
   ```rust
   // Update only specific window
   app.emit_to("settings-window", "settings-updated", &settings)?;
   ```

3. **Coordinate Through Backend State**:
   ```rust
   // All windows read from same backend state
   #[tauri::command]
   pub fn get_shared_state(state: State<'_, Mutex<SharedState>>) -> SharedState {
       state.lock().unwrap().clone()
   }
   ```

### Error Handling

1. **Handle Event Errors Gracefully**:
   ```typescript
   try {
     await listen('data-update', (event) => {
       // Handle event
     });
   } catch (error) {
     console.error('Failed to set up event listener:', error);
     // Fallback to polling
   }
   ```

2. **Validate Event Payloads**:
   ```typescript
   await listen<DataUpdate>('data-updated', (event) => {
     // Validate payload
     if (isValidData(event.payload)) {
       store.set(event.payload);
     } else {
       console.warn('Invalid event payload:', event.payload);
     }
   });
   ```

## 6. Example: Complete Multi-Window State Sync

**Backend**:
```rust
use std::sync::Mutex;
use tauri::{State, Manager, Emitter, EventTarget};

pub struct AppState {
    pub selected_workspace: Option<String>,
    pub selected_well: Option<String>,
}

#[tauri::command]
pub fn select_workspace(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
    workspace_id: String,
) -> Result<(), String> {
    // Update state
    {
        let mut state = state.lock().unwrap();
        state.selected_workspace = Some(workspace_id.clone());
        state.selected_well = None; // Clear dependent state
    }
    
    // Notify all windows
    app.emit_to(
        EventTarget::any(),
        "workspace-selected",
        &workspace_id
    )?;
    
    Ok(())
}

#[tauri::command]
pub fn get_current_selection(
    state: State<'_, Mutex<AppState>>,
) -> AppState {
    state.lock().unwrap().clone()
}
```

**Frontend - Main Window**:
```typescript
import { listen } from '@tauri-apps/api/event';
import { selectedWorkspaceId } from './stores/compute';

// Listen for workspace selection from any window
await listen<string>('workspace-selected', (event) => {
  selectedWorkspaceId.set(event.payload);
  // UI automatically updates via Svelte reactivity
});

// User selects workspace
export async function selectWorkspace(id: string) {
  await invoke('select_workspace', { workspaceId: id });
  // Event will be emitted, all windows will update
}
```

**Frontend - Secondary Window**:
```typescript
// Also listens and stays in sync
await listen<string>('workspace-selected', (event) => {
  selectedWorkspaceId.set(event.payload);
  // This window's UI also updates automatically
});
```

## 7. Your Current Architecture

Your DataForge Compute application uses:

1. **Tauri Commands** for all backend communication (`invoke()`)
2. **Svelte Stores** for frontend state management
3. **Custom Event Bus** for frontend component coordination
4. **Backend State Management** via `State<'_, Mutex<ComputeState>>`

**Potential Enhancement**: Add Tauri events for real-time backend-to-frontend updates:

```rust
// In commands.rs, after state changes:
app.emit_to(EventTarget::any(), "curve-data-updated", &curve_id)?;
```

```typescript
// In stores/compute.ts:
await listen<string>('curve-data-updated', async (event) => {
  // Refresh curve data
  await selectCurve(event.payload);
});
```

This would enable:
- Real-time updates when backend state changes
- Multi-window synchronization
- Progress updates for long-running operations
- Automatic refresh when data changes externally

## Summary

### Communication Patterns

| Pattern | Direction | Use Case | Example |
|---------|-----------|----------|---------|
| **Commands** | Frontend → Backend | User actions, queries | `invoke('get_data')` |
| **Events** | Backend → Frontend | Real-time updates, progress | `app.emit("progress", 50)` |
| **State** | Shared (Backend) | Persistent application state | `State<'_, Mutex<AppState>>` |

### When to Use Each

- **Commands**: When frontend needs to request data or trigger actions
- **Events**: When backend needs to push updates to frontend
- **State**: When data needs to persist across command calls

### Multi-Window Best Practices

1. Use `EventTarget::any()` for broadcasts to all windows
2. Use specific window labels for targeted updates
3. Keep shared state in backend, sync via events
4. Use frontend stores + events for component coordination
5. Clean up event listeners on component unmount

Your current architecture is solid for a single-window application. Adding Tauri events would enhance it for multi-window scenarios and real-time updates.

