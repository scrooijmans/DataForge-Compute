# Context7 Documentation

Context7 brings up-to-date, version-specific documentation and code examples directly into your AI coding assistant for all libraries in the DataForge-Compute tech stack.

## Quick Start

Add `use context7` to your prompt when asking about any library:

```
Create a SciChart LineChart component for visualizing curve data. use context7
```

```
Set up AG Grid with custom cell renderers for computation results. use context7
```

## Complete Guide

See **[AGENTS.md](../mcp/context7/AGENTS.md)** for:

- Complete tech stack overview
- Available Context7 MCP tools
- Best practices and usage patterns
- Library-specific examples

## How It Works

1. **Write your prompt naturally** - describe what you want to build
2. **Add `use context7`** to your prompt
3. **Get working code** with current APIs

Context7 grounds your LLM with up-to-date documentation, ensuring that it always writes high-quality code with current APIs.

## Documentation Storage

When Context7 retrieves library documentation, it's stored in `docs/context7/` organized by library in subfolders:

- **Structure**: `docs/context7/{library-name}/{descriptive-filename}.md`
- **Example**: SciChart documentation → `docs/context7/scichart/scichart-linechart.md`
- **Example**: Tauri documentation → `docs/context7/tauri/tauri-commands.md`
- **Example**: Svelte documentation → `docs/context7/svelte/svelte-runes.md`

This organization keeps documentation organized by library and makes it easy to find relevant documentation.

## DataForge-Compute Tech Stack

### Primary Libraries

- **SciChart** - Scientific charting for curve visualization
- **AG Grid** - Enterprise data grid for tabular data
- **SvelteKit + Svelte 5** - Frontend framework
- **Tauri 2** - Desktop application wrapper
- **Tailwind CSS 4** - Styling

### Example Queries

```
Create a SciChart LineChart with multiple series and zoom/pan. use context7
```

```
Set up AG Grid with custom cell renderers. use context7
```

```
Create a Tauri command that processes data. use context7
```

```
Implement Svelte 5 runes for parameter forms. use context7
```

## More Information

- [Context7 Official Docs](https://context7.com/docs/overview)
- [DataForge-Compute Tech Stack Guide](../mcp/context7/AGENTS.md)

