# CLAUDE.md

## Code Style

- Tabs for indentation
- Single quotes, no trailing commas
- 100 character print width
- Prettier with svelte and tailwindcss plugins

## Project Structure

- `src/routes/` - SvelteKit file-based routing
- `src/lib/` - Shared code, importable via `$lib` alias
- `src/lib/components/` - Reusable Svelte components
- `src/lib/stores/` - Svelte stores for state management
- `src/lib/types/` - TypeScript type definitions
- `static/` - Public static files served at root
- `src-tauri/` - Rust backend code (Tauri)
- `src-tauri/src/compute/` - Compute engine and providers

## Context7

**Trigger**: User says "use context7" (for library documentation questions).

**Usage**: Add `use context7` to your prompt when asking about any library in the DataForge-Compute tech stack.

**Available Libraries**:

### Core Framework

- `svelte` - Svelte 5 framework
  - When generating Svelte examples, prefer **Svelte 5 runes** (`$state`, `$derived`, `$effect()`) and
    avoid deprecated lifecycle APIs like `onMount()` in new code. Lifecycle‑style logic that used to
    live in `onMount()` should instead be expressed via `$effect()`.
- `sveltekit` - SvelteKit framework
- `tauri` - Desktop application wrapper (Tauri 2)
- `typescript` - TypeScript language
- `vite` - Build tooling

### Styling & UI

- `tailwindcss` - Tailwind CSS 4
- `lucide` - Icon system

### Data Visualization

- `scichart` - Advanced scientific charting (WASM-powered)
  - Used for curve visualization and plotting
  - Example: `Create a SciChart LineChart with multiple series. use context7`
- `ag-grid` - Enterprise data grid
  - Used for displaying tabular data
  - Example: `Set up AG Grid with custom cell renderers. use context7`

### Backend & Data

- `tauri` - Tauri commands and backend integration
  - Example: `Create a Tauri command that processes Parquet data. use context7`

**Examples**:

```
Create a SciChart LineChart component for visualizing curve data. use context7
```

```
Set up AG Grid with custom cell renderers for displaying computation results. use context7
```

```
Create a Tauri command that reads Parquet files and returns curve data. use context7
```

```
Implement a Svelte 5 component with $state and $effect for parameter forms. use context7
```

**Documentation Storage**: All Context7-generated documentation is stored in `docs/context7/` organized
by library in subfolders. For example, SciChart documentation would be stored in
`docs/context7/scichart/scichart-linechart.md`.

**Design & Reuse Expectations**: When asking for new frontend or backend components, design them to
be reusable, composable, and aligned with software design best practices:

- Prefer generic, well-named components/stores over ad-hoc page-specific code
- Keep domain logic separate from presentation and infrastructure
- Choose file locations and names that reflect ownership and responsibility
- Follow the explicit computation model: users intentionally trigger runs

**Complete Guide**: See `docs/mcp/context7/AGENTS.md` for complete guide.

