<script lang="ts">
	/**
	 * WorkspaceContainer - Root workspace component
	 *
	 * The top-level component that:
	 * - Subscribes to workspace layout changes
	 * - Renders the layout tree via LayoutRenderer
	 * - Provides workspace toolbar
	 * - Handles persistence (save/restore) per workspace
	 *
	 * Layout storage is workspace-scoped: each workspace maintains its own
	 * chart layout in localStorage with key `dataforge-workspace-layout-${workspaceId}`.
	 *
	 * See DFC-chart-implementation.md Section 12 for design details.
	 */
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { workspaceManager, PaneType } from '$lib/panes/workspace-manager';
	import type { WorkspaceLayout } from '$lib/panes/layout-model';
	import LayoutRenderer from './LayoutRenderer.svelte';
	import ChartInteractionBar from '$lib/components/charts/ChartInteractionBar.svelte';
	import { selectedWorkspaceId, previousWorkspaceIdForLayout } from '$lib/stores/compute';
	import { untrack } from 'svelte';

	/**
	 * ChartLayout returned from Tauri backend
	 */
	interface ChartLayout {
		id: string;
		workspace_id: string;
		layout_json: string;
		version: number;
		sync_version: number;
		sync_status: string;
		created_at: string;
		updated_at: string;
	}

	interface Props {
		/** Whether to auto-restore layout on mount */
		autoRestore?: boolean;
		/** Whether to auto-save layout on changes */
		autoSave?: boolean;
	}

	let { autoRestore = true, autoSave = true }: Props = $props();

	/** Workspace-scoped storage key - derived from selected workspace ID */
	let storageKey = $derived(
		$selectedWorkspaceId ? `dataforge-workspace-layout-${$selectedWorkspaceId}` : null
	);

	/** Current layout from workspace manager */
	let layout = workspaceManager.layout;

	/** Events from workspace manager */
	let events = workspaceManager.events;

	/** Pane type menu open state */
	let showAddMenu = $state(false);

	/** Guard flag to prevent auto-save during layout reset/restore operations */
	let isResetting = $state(false);

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
	 * Save layout to both localStorage (fast) and backend (durable)
	 */
	function saveLayout(): void {
		if (!storageKey || !$selectedWorkspaceId) return; // No workspace selected
		try {
			const currentLayout = workspaceManager.saveLayout();
			const layoutJson = JSON.stringify(currentLayout);

			console.log('[WorkspaceContainer] saveLayout() called for workspace:', $selectedWorkspaceId);
			console.log('[WorkspaceContainer] Layout root type:', currentLayout.root.type);

			// Immediate: localStorage for fast recovery
			localStorage.setItem(storageKey, layoutJson);

			// Durable: backend database (async, non-blocking)
			invoke<ChartLayout>('save_workspace_layout', {
				workspaceId: $selectedWorkspaceId,
				layoutJson
			}).catch((err) => console.error('Backend save failed:', err));
		} catch (error) {
			console.error('Failed to save workspace layout:', error);
		}
	}

	/**
	 * Save layout for a specific workspace ID (used during workspace switching)
	 * This ensures we save the OLD workspace's layout before switching to new one.
	 */
	function saveLayoutForWorkspace(workspaceId: string): void {
		if (!workspaceId) return;
		const key = `dataforge-workspace-layout-${workspaceId}`;
		try {
			const currentLayout = workspaceManager.saveLayout();
			const layoutJson = JSON.stringify(currentLayout);

			console.log('[WorkspaceContainer] saveLayoutForWorkspace() called for workspace:', workspaceId);
			console.log('[WorkspaceContainer] Layout root type:', currentLayout.root.type);
			console.log('[WorkspaceContainer] Layout JSON length:', layoutJson.length);

			// Immediate: localStorage
			localStorage.setItem(key, layoutJson);

			// Durable: backend database (async, non-blocking)
			invoke<ChartLayout>('save_workspace_layout', {
				workspaceId,
				layoutJson
			}).catch((err) => console.error('Backend save failed:', err));
		} catch (error) {
			console.error('Failed to save workspace layout:', error);
		}
	}

	/**
	 * Restore layout from backend or localStorage (workspace-scoped)
	 * Note: Caller should set isResetting = true before calling to prevent auto-save loops
	 *
	 * Priority:
	 * 1. Try backend database first (source of truth)
	 * 2. Fall back to localStorage if backend fails or has no data
	 * 3. Reset to default if neither has data
	 */
	async function restoreLayout(): Promise<void> {
		const workspaceId = $selectedWorkspaceId;
		console.log('[WorkspaceContainer] restoreLayout() called for workspace:', workspaceId);

		if (!workspaceId) {
			// No workspace selected - reset to default
			console.log('[WorkspaceContainer] No workspace selected, resetting to default');
			workspaceManager.resetLayout();
			return;
		}

		try {
			// Try backend first (source of truth)
			console.log('[WorkspaceContainer] Trying backend for workspace:', workspaceId);
			const backendLayout = await invoke<ChartLayout | null>('get_workspace_layout', {
				workspaceId
			});

			console.log('[WorkspaceContainer] Backend returned:', backendLayout ? 'layout found' : 'null');

			if (backendLayout?.layout_json) {
				const layout = JSON.parse(backendLayout.layout_json) as WorkspaceLayout;
				console.log('[WorkspaceContainer] Restoring from backend, root type:', layout.root.type);
				workspaceManager.restoreLayout(layout);
				// Update localStorage to match backend
				if (storageKey) {
					localStorage.setItem(storageKey, backendLayout.layout_json);
				}
				return;
			}
		} catch (error) {
			console.warn('[WorkspaceContainer] Backend restore failed, falling back to localStorage:', error);
		}

		// Fall back to localStorage
		try {
			if (storageKey) {
				const saved = localStorage.getItem(storageKey);
				console.log('[WorkspaceContainer] localStorage has data:', saved ? 'yes' : 'no');
				if (saved) {
					const layout = JSON.parse(saved) as WorkspaceLayout;
					console.log('[WorkspaceContainer] Restoring from localStorage, root type:', layout.root.type);
					workspaceManager.restoreLayout(layout);
					return;
				}
			}
		} catch (error) {
			console.error('[WorkspaceContainer] Failed to restore workspace layout from localStorage:', error);
		}

		// No saved layout for this workspace - start fresh
		console.log('[WorkspaceContainer] No saved layout found, resetting to default');
		workspaceManager.resetLayout();
	}

	/**
	 * Reset layout to default (user-initiated reset via toolbar button)
	 */
	function handleResetLayout(): void {
		isResetting = true;
		workspaceManager.resetLayout();
		isResetting = false;

		// Remove the saved layout for this workspace from localStorage
		if (storageKey) {
			localStorage.removeItem(storageKey);
		}

		// Also delete from backend database
		if ($selectedWorkspaceId) {
			invoke<boolean>('delete_workspace_layout', {
				workspaceId: $selectedWorkspaceId
			}).catch((err) => console.error('Failed to delete layout from backend:', err));
		}
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

	// Auto-save on layout changes - SKIP during reset/restore operations AND workspace switches
	$effect(() => {
		// Track $events to react to layout changes
		if (!autoSave || !$events || isResetting) return;

		// CRITICAL: Check if we're in the middle of a workspace switch.
		// If selectedWorkspaceId !== previousWorkspaceIdForLayout, the workspace just changed
		// and we should NOT auto-save here - the workspace change effect will handle
		// saving the previous workspace's layout to the correct key.
		const currentWorkspaceId = $selectedWorkspaceId;
		const previousWorkspaceId = untrack(() => $previousWorkspaceIdForLayout);

		if (currentWorkspaceId !== previousWorkspaceId) {
			console.log('[WorkspaceContainer] Auto-save SKIPPED - workspace switch in progress');
			return;
		}

		console.log('[WorkspaceContainer] Auto-save triggered for workspace:', currentWorkspaceId);
		saveLayout();
	});

	// Re-restore layout when workspace changes - WITH PROPER SAVE ORDER
	// Uses persistent store (previousWorkspaceIdForLayout) instead of local variable
	// to survive component unmount/remount during page navigation.
	//
	// We use untrack() to read previousWorkspaceIdForLayout without creating a
	// reactive dependency - we only want this effect to re-run when
	// $selectedWorkspaceId changes, not when the previous ID is updated.
	$effect(() => {
		const currentWorkspaceId = $selectedWorkspaceId;
		// Read previous without tracking - we only react to currentWorkspaceId changes
		const previousWorkspaceId = untrack(() => $previousWorkspaceIdForLayout);
		console.log('[WorkspaceContainer] Workspace change effect - current:', currentWorkspaceId, 'previous:', previousWorkspaceId);

		if (currentWorkspaceId !== previousWorkspaceId) {
			console.log('[WorkspaceContainer] Workspace CHANGED from', previousWorkspaceId, 'to', currentWorkspaceId);

			// CRITICAL: Save the PREVIOUS workspace's layout BEFORE switching
			// This must happen before we update previousWorkspaceIdForLayout
			if (previousWorkspaceId && autoSave) {
				console.log('[WorkspaceContainer] Saving PREVIOUS workspace layout before switch');
				saveLayoutForWorkspace(previousWorkspaceId);
			}

			// Update the persistent store
			previousWorkspaceIdForLayout.set(currentWorkspaceId);

			// Only restore if we have a valid workspace and autoRestore is enabled
			// Set guard to prevent auto-save from triggering during restore
			if (autoRestore && currentWorkspaceId) {
				console.log('[WorkspaceContainer] Setting isResetting=true, starting restore');
				isResetting = true;
				// Use IIFE for async restore - guard state is managed around the async call
				(async () => {
					try {
						await restoreLayout();
					} finally {
						console.log('[WorkspaceContainer] Restore complete, setting isResetting=false');
						isResetting = false;
					}
				})();
			}
		}
	});

	onMount(() => {
		// Restore layout on mount - set guard to prevent auto-save during restore
		if (autoRestore) {
			isResetting = true;
			// Async restore with guard management
			(async () => {
				try {
					await restoreLayout();
				} finally {
					isResetting = false;
				}
			})();
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

	<!-- Chart Interaction Bar - Always visible, shows cursor mode tools -->
	<ChartInteractionBar />

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
