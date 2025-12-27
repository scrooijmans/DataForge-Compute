<script lang="ts">
	/**
	 * WorkspaceContainer - Root workspace component
	 *
	 * The top-level component that:
	 * - Subscribes to workspace layout changes
	 * - Renders the layout tree via LayoutRenderer
	 * - Provides workspace toolbar
	 * - Handles persistence (save/restore)
	 *
	 * See DFC-chart-implementation.md Section 12 for design details.
	 */
	import { onMount } from 'svelte';
	import { workspaceManager, PaneType } from '$lib/panes/workspace-manager';
	import type { WorkspaceLayout } from '$lib/panes/layout-model';
	import LayoutRenderer from './LayoutRenderer.svelte';

	interface Props {
		/** Storage key for layout persistence */
		storageKey?: string;
		/** Whether to auto-restore layout on mount */
		autoRestore?: boolean;
		/** Whether to auto-save layout on changes */
		autoSave?: boolean;
	}

	let { storageKey = 'dataforge-workspace-layout', autoRestore = true, autoSave = true }: Props =
		$props();

	/** Current layout from workspace manager */
	let layout = workspaceManager.layout;

	/** Events from workspace manager */
	let events = workspaceManager.events;

	/** Pane type menu open state */
	let showAddMenu = $state(false);

	/** Available pane types for adding */
	const paneTypes = [
		{ type: PaneType.LineChart, label: 'Line Chart' },
		{ type: PaneType.ScatterChart, label: 'Scatter Chart' },
		{ type: PaneType.Histogram, label: 'Histogram' },
		{ type: PaneType.WellLog, label: 'Well Log' },
		{ type: PaneType.LinkedCharts, label: 'Linked Charts' },
		{ type: PaneType.DataGrid, label: 'Data Grid' },
	];

	/** Check if workspace is empty (no panes or only empty pane) */
	let isEmpty = $derived.by(() => {
		if (!$layout) return true;
		const root = $layout.root;
		// Empty if root is a single empty pane
		if (root.type === 'pane' && root.paneType === PaneType.Empty) return true;
		return false;
	});

	/**
	 * Save layout to localStorage
	 */
	function saveLayout(): void {
		try {
			const currentLayout = workspaceManager.saveLayout();
			localStorage.setItem(storageKey, JSON.stringify(currentLayout));
		} catch (error) {
			console.error('Failed to save workspace layout:', error);
		}
	}

	/**
	 * Restore layout from localStorage
	 */
	function restoreLayout(): void {
		try {
			const saved = localStorage.getItem(storageKey);
			if (saved) {
				const layout = JSON.parse(saved) as WorkspaceLayout;
				workspaceManager.restoreLayout(layout);
			}
		} catch (error) {
			console.error('Failed to restore workspace layout:', error);
			// Reset to default on error
			workspaceManager.resetLayout();
		}
	}

	/**
	 * Reset layout to default
	 */
	function handleResetLayout(): void {
		workspaceManager.resetLayout();
		localStorage.removeItem(storageKey);
	}

	/**
	 * Add a new pane of the specified type
	 */
	function handleAddPane(paneType: PaneType): void {
		workspaceManager.addPane(paneType, {});
		showAddMenu = false;
	}

	/**
	 * Toggle add menu
	 */
	function toggleAddMenu(): void {
		showAddMenu = !showAddMenu;
	}

	/**
	 * Close add menu when clicking outside
	 */
	function handleClickOutside(event: MouseEvent): void {
		const target = event.target as HTMLElement;
		if (!target.closest('.add-menu-container')) {
			showAddMenu = false;
		}
	}

	// Auto-save on layout changes
	$effect(() => {
		if (autoSave && $events) {
			saveLayout();
		}
	});

	onMount(() => {
		// Restore layout on mount
		if (autoRestore) {
			restoreLayout();
		}

		// Add click outside listener
		document.addEventListener('click', handleClickOutside);

		return () => {
			document.removeEventListener('click', handleClickOutside);
		};
	});
</script>

<div class="workspace-container">
	<!-- Workspace Toolbar -->
	<div class="workspace-toolbar">
		<div class="toolbar-left">
			<span class="toolbar-title">Workspace</span>
		</div>

		<div class="toolbar-right">
			<!-- Add Pane Button -->
			<div class="add-menu-container">
				<button
					class="toolbar-button"
					aria-label="Add pane"
					aria-haspopup="true"
					aria-expanded={showAddMenu}
					onclick={toggleAddMenu}
				>
					<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
						<path d="M8 2v12M2 8h12" stroke="currentColor" stroke-width="2" fill="none" />
					</svg>
					<span>Add Pane</span>
				</button>

				{#if showAddMenu}
					<div class="add-menu" role="menu">
						{#each paneTypes as { type, label }}
							<button
								class="add-menu-item"
								role="menuitem"
								onclick={() => handleAddPane(type)}
							>
								<span>{label}</span>
							</button>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Reset Button -->
			<button class="toolbar-button toolbar-button-secondary" aria-label="Reset layout" onclick={handleResetLayout}>
				<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
					<path d="M2 7a5 5 0 1 1 1.5 3.5" />
					<path d="M2 11V7h4" />
				</svg>
				<span>Reset</span>
			</button>
		</div>
	</div>

	<!-- Layout Content -->
	<div class="workspace-content">
		{#if isEmpty}
			<!-- Empty workspace placeholder -->
			<div class="workspace-empty">
				<svg width="48" height="48" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.5">
					<rect x="6" y="6" width="36" height="36" rx="4" opacity="0.3" />
					<path d="M24 16v16M16 24h16" opacity="0.5" />
				</svg>
				<h3>No Charts Open</h3>
				<p>Click <strong>Add Pane</strong> to add a chart to your workspace</p>
			</div>
		{:else if $layout}
			<LayoutRenderer node={$layout.root} />
		{/if}
	</div>
</div>

<style>
	.workspace-container {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		overflow: hidden;
		background: var(--color-bg-tertiary, #f3f4f6);
	}

	.workspace-toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 44px;
		padding: 0 12px;
		background: var(--color-bg, #ffffff);
		border-bottom: 1px solid var(--color-border, #e5e7eb);
		flex-shrink: 0;
	}

	.toolbar-left {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.toolbar-title {
		font-size: 14px;
		font-weight: 600;
		color: var(--color-text, #111827);
	}

	.toolbar-right {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.toolbar-button {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		border: none;
		background: var(--color-primary, #3b82f6);
		color: white;
		font-size: 13px;
		font-weight: 500;
		border-radius: 6px;
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			transform 0.1s ease;
	}

	.toolbar-button:hover {
		background: var(--color-primary-hover, #2563eb);
	}

	.toolbar-button:active {
		transform: scale(0.98);
	}

	.toolbar-button:focus-visible {
		outline: 2px solid var(--color-primary, #3b82f6);
		outline-offset: 2px;
	}

	.toolbar-button-secondary {
		background: var(--color-bg-secondary, #f3f4f6);
		color: var(--color-text, #111827);
	}

	.toolbar-button-secondary:hover {
		background: var(--color-bg-hover, #e5e7eb);
	}

	.add-menu-container {
		position: relative;
	}

	.add-menu {
		position: absolute;
		top: 100%;
		right: 0;
		margin-top: 4px;
		min-width: 180px;
		background: var(--color-bg, #ffffff);
		border: 1px solid var(--color-border, #e5e7eb);
		border-radius: 8px;
		box-shadow:
			0 4px 6px -1px rgb(0 0 0 / 0.1),
			0 2px 4px -2px rgb(0 0 0 / 0.1);
		padding: 4px;
		z-index: 100;
	}

	.add-menu-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 8px 12px;
		border: none;
		background: transparent;
		font-size: 13px;
		color: var(--color-text, #111827);
		text-align: left;
		border-radius: 6px;
		cursor: pointer;
		transition: background-color 0.15s ease;
	}

	.add-menu-item:hover {
		background: var(--color-bg-hover, #f3f4f6);
	}

	.add-menu-item:focus-visible {
		outline: 2px solid var(--color-primary, #3b82f6);
		outline-offset: -2px;
	}

	.workspace-content {
		flex: 1;
		overflow: hidden;
		padding: 4px;
	}

	.workspace-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-tertiary, #9ca3af);
		text-align: center;
		padding: 24px;
	}

	.workspace-empty svg {
		margin-bottom: 16px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.workspace-empty h3 {
		margin: 0 0 8px 0;
		font-size: 16px;
		font-weight: 600;
		color: var(--color-text-secondary, #6b7280);
	}

	.workspace-empty p {
		margin: 0;
		font-size: 14px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.workspace-empty strong {
		color: var(--color-text, #111827);
	}
</style>
