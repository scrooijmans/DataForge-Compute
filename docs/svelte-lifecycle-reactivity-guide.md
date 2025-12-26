# Svelte Component Lifecycle and Reactivity in Multi-Pane Applications

Based on Svelte 5 documentation and best practices for managing state in large, multi-pane applications like DataForge Compute.

## Component Lifecycle in Svelte 5

### Simplified Lifecycle Model

In Svelte 5, the component lifecycle consists of **only two parts**:

1. **Creation** - When the component is mounted to the DOM
2. **Destruction** - When the component is removed from the DOM

**Key Insight**: Everything in-between (state updates) is not related to the component as a whole. Only the parts that need to react to state changes are notified. The smallest unit of change is not a component—it's the **render effects** that the component sets up upon initialization.

**Important**: There is no "before update" or "after update" hook in Svelte 5. This is by design—Svelte's granular reactivity means only affected parts of the DOM update, not the entire component.

### Lifecycle Hooks

#### `onMount`

Runs once when the component is mounted to the DOM. Useful for:
- Initial data fetching
- Setting up subscriptions
- DOM measurements
- One-time initialization

```svelte
<script>
  import { onMount } from 'svelte';
  
  onMount(async () => {
    await loadStatus();
    // Cleanup function (optional)
    return () => {
      // Cleanup logic
    };
  });
</script>
```

**Note**: The cleanup function only executes if the `onMount` callback is synchronous.

#### `onDestroy`

Runs when the component is destroyed. Useful for:
- Cleaning up subscriptions
- Canceling timers
- Releasing resources

```svelte
<script>
  import { onDestroy } from 'svelte';
  
  onDestroy(() => {
    // Cleanup logic
  });
</script>
```

## Reactivity System: Runes

Svelte 5 introduces **runes**—explicit markers for reactivity that work both inside and outside components.

### `$state` - Reactive State

Creates reactive state that triggers updates when changed:

```svelte
<script>
  let count = $state(0);
  let chartDataFrame = $state<ChartDataFrame | null>(null);
  
  // Deep reactivity for objects and arrays
  let gridRowData = $state<Record<string, unknown>[]>([]);
</script>
```

**Key Features**:
- **Deep reactivity**: Objects and arrays are proxied recursively, so mutations like `array.push()` trigger updates
- **Granular updates**: Only parts of the DOM that read the changed state update
- **No wrapper needed**: Unlike Vue's `.value`, you read/write directly to the variable

**Performance Optimization**: Use `$state.raw()` for large arrays/objects you won't mutate:

```svelte
// Avoids proxy overhead for read-only data
let largeDataset = $state.raw([...hugeArray]);
```

**Important**: `$state.raw` values cannot be mutated—only reassigned:

```svelte
// ❌ Won't work
largeDataset.push(newItem);

// ✅ Correct
largeDataset = [...largeDataset, newItem];
```

### `$derived` - Computed Values

Creates derived state that automatically updates when dependencies change:

```svelte
<script>
  let count = $state(0);
  let double = $derived(count * 2);
  
  // Complex derivations
  let filteredUdfs = $derived.by(() => {
    if (!searchQuery.trim()) {
      return $udfs;
    }
    const query = searchQuery.toLowerCase();
    return $udfs.filter(udf => 
      udf.name.toLowerCase().includes(query)
    );
  });
</script>
```

**Key Features**:
- **Automatic memoization**: Only recalculates when dependencies change
- **Referential equality**: Skips updates if the derived value is referentially identical
- **No side effects**: Expressions must be side-effect-free (use `$effect` for side effects)

**Example from your codebase** (`UdfToolbox.svelte`):
```svelte
let filteredUdfs = $derived.by(() => {
  if (!searchQuery.trim()) {
    return $udfs;
  }
  const query = searchQuery.toLowerCase();
  return $udfs.filter(
    (udf) =>
      udf.name.toLowerCase().includes(query) ||
      udf.description.toLowerCase().includes(query) ||
      udf.tags.some((tag) => tag.toLowerCase().includes(query))
  );
});
```

### `$effect` - Side Effects

Runs code when dependencies change, after DOM updates:

```svelte
<script>
  let executionResult = $state(null);
  
  // Runs when executionResult changes
  $effect(() => {
    if ($executionResult?.success && $executionResult.output_data) {
      // Prepare grid data
      gridRowData = $executionResult.output_data.map(...);
    }
  });
</script>
```

**Key Features**:
- **Automatic dependency tracking**: Tracks which state/derived values are accessed
- **Runs after DOM updates**: Ensures DOM is current before executing
- **Cleanup support**: Return a function for cleanup

**Example from your codebase** (`ExecutionResult.svelte`):
```svelte
// Load DataGrid dynamically to avoid SSR issues
$effect(() => {
  if (browser && $executionResult?.success && $executionResult.output_data) {
    import('./data/DataGrid.svelte').then((module) => {
      DataGrid = module.default;
    });
  }
});

// Prepare grid data when execution result changes
$effect(() => {
  if ($executionResult?.success && $executionResult.output_data) {
    const outputMnemonic = $executionResult.output_mnemonic || 'Output';
    // ... prepare column definitions and row data
  }
});
```

**Cleanup Example**:
```svelte
$effect(() => {
  const interval = setInterval(() => {
    console.log('tick');
  }, 1000);
  
  // Cleanup runs before effect re-runs or on unmount
  return () => clearInterval(interval);
});
```

## Stores vs Runes in Multi-Pane Applications

### When to Use Stores

**Prior to Svelte 5**, stores were the go-to solution for cross-component state. **With runes, these use cases have greatly diminished**, but stores are still useful for:

1. **Cross-component state sharing** (especially when components aren't in the same component tree)
2. **Integration with non-Svelte code**
3. **State that needs to persist across route changes**
4. **Complex state machines or event-driven architectures**

### When to Use Runes

Use runes (`$state`, `$derived`, `$effect`) for:
1. **Component-local state**
2. **State shared via props/context**
3. **Derived computations within components**
4. **Side effects tied to component lifecycle**

### Hybrid Approach (Your Codebase Pattern)

Your codebase uses a **hybrid approach** that's common in Svelte 5:

- **Stores** for global application state (`compute.ts`):
  ```typescript
  export const executionResult = writable<ExecuteUdfResult | null>(null);
  export const selectedCurveId = writable<string | null>(null);
  ```

- **Runes** for component-local reactivity:
  ```svelte
  let chartDataFrame = $state<ChartDataFrame | null>(null);
  let gridRowData = $state<Record<string, unknown>[]>([]);
  ```

- **`$effect`** to react to store changes:
  ```svelte
  $effect(() => {
    if ($executionResult?.success) {
      // React to store changes
      chartDataFrame = curveDataToFrame($executionResult.output_data);
    }
  });
  ```

This pattern works well because:
- Stores provide a stable API for cross-component communication
- Runes provide fine-grained reactivity within components
- The `$` prefix on stores makes them reactive in runes mode

## Best Practices for Avoiding Stale State

### 1. Use `$derived` Instead of Manual Calculations

**❌ Bad**: Manual recalculation in effects
```svelte
let count = $state(0);
let double = 0;

$effect(() => {
  double = count * 2; // Can become stale if dependencies change
});
```

**✅ Good**: Automatic derivation
```svelte
let count = $state(0);
let double = $derived(count * 2); // Always up-to-date
```

### 2. Avoid Destructuring Reactive Values

**❌ Bad**: Destructuring breaks reactivity
```svelte
let state = $state({ count: 0, name: 'test' });
let { count, name } = state; // Not reactive!
```

**✅ Good**: Access properties directly
```svelte
let state = $state({ count: 0, name: 'test' });
// Use state.count and state.name in template
```

### 3. Use `untrack()` to Prevent Dependency Tracking

When you need to read state without creating a dependency:

```svelte
import { untrack } from 'svelte';

$effect(() => {
  // Read count without tracking it
  const currentCount = untrack(() => count);
  
  // Only tracks 'otherValue', not 'count'
  if (otherValue > 10) {
    console.log('Count was', currentCount, 'when condition became true');
  }
});
```

### 4. Avoid Writing to State in Effects (When Possible)

**❌ Problematic**: Writing to state in effects can cause update loops
```svelte
let logs = $state([]);

$effect(() => {
  if (condition) {
    logs.push('new log'); // Can cause effect_update_depth_exceeded
  }
});
```

**✅ Better**: Use normal arrays for logs, or be careful with dependencies
```svelte
let logs: string[] = []; // Not reactive

$effect(() => {
  if (condition) {
    logs.push('new log'); // Safe - logs is not reactive
  }
});
```

### 5. Use `get()` for One-Time Reads from Stores

When you need a snapshot value from a store (not reactive):

```typescript
import { get } from 'svelte/store';

export async function saveOutputCurve() {
  // Get current values (not reactive)
  const currentResult = get(executionResult);
  const currentWellId = get(selectedWellId);
  
  // Use these values in async operations
  if (!currentResult || !currentResult.success) {
    return;
  }
  // ...
}
```

### 6. Clear Dependent State When Parent State Changes

**Example from your codebase** (`compute.ts`):
```typescript
export async function selectWell(id: string) {
  selectedWellId.set(id);
  selectedCurveId.set(null);  // Clear dependent state
  curves.set([]);              // Clear dependent data
  curveData.set(null);         // Clear dependent data
  executionResult.set(null);   // Clear dependent results
  // ...
}
```

This prevents stale data from previous selections.

## Best Practices for Avoiding Unnecessary Re-renders

### 1. Use `$derived` for Expensive Computations

`$derived` automatically memoizes and only recalculates when dependencies change:

```svelte
// ✅ Only recalculates when $udfs or searchQuery changes
let filteredUdfs = $derived.by(() => {
  // Expensive filtering operation
  return $udfs.filter(/* complex logic */);
});
```

### 2. Use `$state.raw()` for Large, Read-Only Data

For large datasets that won't be mutated:

```svelte
// ✅ Avoids proxy overhead
let largeDataset = $state.raw([...hugeArray]);
```

### 3. Minimize Reactive Dependencies in Effects

Only access the state you actually need:

```svelte
// ❌ Bad: Tracks both values even if only one changes
$effect(() => {
  if ($executionResult && $curveData) {
    // Effect runs when EITHER changes
  }
});

// ✅ Better: Separate effects or use untrack
$effect(() => {
  if ($executionResult?.success) {
    const curve = untrack(() => $curveData);
    // Only tracks executionResult
  }
});
```

### 4. Use Keyed `{#each}` Blocks

Always use keys in `{#each}` to help Svelte track items efficiently:

```svelte
<!-- ✅ Good: Uses key for efficient updates -->
{#each $curves as curve (curve.id)}
  <CurveCard {curve} />
{/each}

<!-- ❌ Bad: No key, causes unnecessary re-renders -->
{#each $curves as curve}
  <CurveCard {curve} />
{/each}
```

### 5. Conditionally Render Components

Use `{#if}` blocks to prevent unnecessary component creation:

```svelte
<!-- ✅ Only creates DataGrid when needed -->
{#if DataGrid && gridRowData.length > 0}
  <DataGrid
    columnDefs={gridColumnDefs}
    rowData={gridRowData}
  />
{/if}
```

### 6. Use `$derived` Instead of Effects for Transformations

**❌ Bad**: Effect for data transformation
```svelte
let transformed = $state([]);

$effect(() => {
  transformed = $sourceData.map(transform);
});
```

**✅ Good**: Derived state
```svelte
let transformed = $derived.by(() => 
  $sourceData.map(transform)
);
```

### 7. Batch Store Updates

When updating multiple related stores, batch the updates:

```typescript
// ✅ Good: Single update cycle
export async function selectWell(id: string) {
  selectedWellId.set(id);
  selectedCurveId.set(null);
  curves.set([]);
  curveData.set(null);
  executionResult.set(null);
  // All updates happen in one cycle
}
```

## Multi-Pane Application Patterns

### Pattern 1: Centralized Store with Component-Level Reactivity

Your codebase follows this pattern:

1. **Global stores** (`compute.ts`) manage application-wide state
2. **Component-level runes** handle local transformations
3. **Effects** bridge stores to component state

```svelte
<!-- ExecutionResult.svelte -->
<script>
  // Global store
  import { executionResult } from '$lib/stores/compute';
  
  // Local reactive state
  let chartDataFrame = $state<ChartDataFrame | null>(null);
  let gridRowData = $state<Record<string, unknown>[]>([]);
  
  // Bridge store to component state
  $effect(() => {
    if ($executionResult?.success && $executionResult.output_data) {
      chartDataFrame = curveDataToFrame($executionResult.output_data);
      gridRowData = $executionResult.output_data.map(...);
    }
  });
</script>
```

### Pattern 2: Derived Stores for Computed Values

Use `derived` stores for values computed from multiple sources:

```typescript
// ✅ Computed once, shared across components
export const selectedWorkspace: Readable<WorkspaceInfo | undefined> = derived(
  [workspaces, selectedWorkspaceId],
  ([$workspaces, $selectedWorkspaceId]) =>
    $workspaces.find((w) => w.id === $selectedWorkspaceId)
);
```

### Pattern 3: Conditional Component Loading

Dynamically load heavy components only when needed:

```svelte
let DataGrid: typeof import('./data/DataGrid.svelte').default | null = $state(null);

$effect(() => {
  if (browser && $executionResult?.success && $executionResult.output_data) {
    import('./data/DataGrid.svelte').then((module) => {
      DataGrid = module.default;
    });
  }
});
```

## Common Pitfalls and Solutions

### Pitfall 1: Stale Closures in Async Operations

**Problem**: Accessing reactive state in async callbacks can capture stale values.

**Solution**: Use `get()` for stores or ensure reactive values are accessed synchronously:

```typescript
// ✅ Good: Capture current value
export async function executeUdf() {
  const currentUdfId = get(selectedUdfId);
  const currentParams = get(parameterValues);
  
  // Use captured values in async operation
  const result = await invoke('execute_udf', {
    udf_id: currentUdfId,
    parameters: currentParams
  });
}
```

### Pitfall 2: Effect Update Loops

**Problem**: Effects that write to their own dependencies cause infinite loops.

**Solution**: Use `untrack()` or restructure to avoid circular dependencies:

```svelte
// ❌ Bad: Can cause loops
let count = $state(0);
$effect(() => {
  if (count < 10) {
    count++; // Writing to dependency!
  }
});

// ✅ Better: Use untrack or separate concerns
let count = $state(0);
$effect(() => {
  const current = untrack(() => count);
  if (current < 10) {
    count = current + 1;
  }
});
```

### Pitfall 3: Unnecessary Deep Reactivity

**Problem**: Large objects/arrays don't need deep reactivity if you only replace them.

**Solution**: Use `$state.raw()` for read-only or replace-only data:

```svelte
// ✅ For data that's replaced, not mutated
let chartData = $state.raw(largeDataset);
```

## Summary

### Lifecycle
- Components have two phases: creation and destruction
- No "update" hooks—only render effects update granularly
- Use `onMount` for initialization, `onDestroy` for cleanup

### Reactivity
- `$state`: Declares reactive state (deep by default)
- `$derived`: Computed values (auto-memoized)
- `$effect`: Side effects (runs after DOM updates)

### Avoiding Stale State
1. Use `$derived` for computed values
2. Avoid destructuring reactive values
3. Use `untrack()` when needed
4. Clear dependent state when parent changes
5. Use `get()` for one-time store reads

### Avoiding Unnecessary Re-renders
1. Use `$derived` for expensive computations
2. Use `$state.raw()` for large read-only data
3. Minimize reactive dependencies
4. Use keyed `{#each}` blocks
5. Conditionally render components
6. Batch related updates

### Multi-Pane Applications
- Use stores for cross-component state
- Use runes for component-local reactivity
- Bridge stores to components with `$effect`
- Use derived stores for shared computations
- Load heavy components conditionally

Your DataForge Compute application demonstrates these patterns well, using stores for global state and runes for component-level reactivity, with effects bridging the two when needed.

