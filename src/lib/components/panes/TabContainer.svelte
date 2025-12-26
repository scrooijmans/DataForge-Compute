<script lang="ts">
	/**
	 * TabContainer - Tabbed pane container
	 *
	 * Contains multiple panes as tabs with a tab bar for switching.
	 * Only the active tab's pane is rendered (or all are rendered but hidden).
	 *
	 * Design inspired by:
	 * - VS Code EditorGroup
	 * - GoldenLayout Stack
	 * - JupyterLab TabBar
	 *
	 * See DFC-chart-implementation.md Section 12 for design details.
	 */
	import type { TabNode, PaneNode } from '$lib/panes/layout-model';
	import { workspaceManager } from '$lib/panes/workspace-manager';
	import PaneContainer from './PaneContainer.svelte';

	interface Props {
		/** Tab node data */
		tabNode: TabNode;
		/** Depth in the layout tree */
		depth?: number;
	}

	let { tabNode, depth = 0 }: Props = $props();

	/** Active pane ID from workspace manager */
	let activePaneId = workspaceManager.activePaneId;

	/**
	 * Handle tab click to activate pane
	 */
	function handleTabClick(pane: PaneNode): void {
		workspaceManager.activatePane(pane.id);
	}

	/**
	 * Handle tab close button click
	 */
	function handleTabClose(event: MouseEvent, pane: PaneNode): void {
		event.stopPropagation();
		if (pane.closable !== false) {
			workspaceManager.removePane(pane.id);
		}
	}

	/**
	 * Handle keyboard navigation in tab bar
	 */
	function handleTabKeyDown(event: KeyboardEvent, pane: PaneNode, index: number): void {
		switch (event.key) {
			case 'Enter':
			case ' ':
				event.preventDefault();
				workspaceManager.activatePane(pane.id);
				break;
			case 'ArrowLeft':
				event.preventDefault();
				if (index > 0) {
					workspaceManager.activatePane(tabNode.children[index - 1].id);
				}
				break;
			case 'ArrowRight':
				event.preventDefault();
				if (index < tabNode.children.length - 1) {
					workspaceManager.activatePane(tabNode.children[index + 1].id);
				}
				break;
			case 'Delete':
			case 'Backspace':
				if (pane.closable !== false) {
					event.preventDefault();
					workspaceManager.removePane(pane.id);
				}
				break;
		}
	}

	/**
	 * Check if a pane is the active one
	 */
	function isActive(pane: PaneNode): boolean {
		return pane.id === $activePaneId || tabNode.children[tabNode.activeIndex]?.id === pane.id;
	}

	/**
	 * Get the currently active pane
	 */
	function getActivePane(): PaneNode | null {
		// First check if any child matches activePaneId
		const activeFromManager = tabNode.children.find((c) => c.id === $activePaneId);
		if (activeFromManager) return activeFromManager;

		// Fall back to activeIndex
		return tabNode.children[tabNode.activeIndex] ?? tabNode.children[0] ?? null;
	}
</script>

<div class="tab-container" data-tab-id={tabNode.id} data-depth={depth}>
	<!-- Tab Bar -->
	<div class="tab-bar" role="tablist" aria-label="Pane tabs">
		{#each tabNode.children as pane, index (pane.id)}
			<button
				class="tab-button"
				class:active={isActive(pane)}
				role="tab"
				aria-selected={isActive(pane)}
				aria-controls={`tabpanel-${pane.id}`}
				tabindex={isActive(pane) ? 0 : -1}
				onclick={() => handleTabClick(pane)}
				onkeydown={(e) => handleTabKeyDown(e, pane, index)}
			>
				{#if pane.icon}
					<span class="tab-icon">{pane.icon}</span>
				{/if}
				<span class="tab-title">{pane.title}</span>
				{#if pane.closable !== false}
					<button
						class="tab-close"
						aria-label={`Close ${pane.title}`}
						onclick={(e) => handleTabClose(e, pane)}
					>
						<svg
							width="12"
							height="12"
							viewBox="0 0 12 12"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<path d="M2 2L10 10M10 2L2 10" />
						</svg>
					</button>
				{/if}
			</button>
		{/each}
	</div>

	<!-- Tab Content -->
	<div class="tab-content">
		{#each tabNode.children as pane (pane.id)}
			<div
				class="tab-panel"
				class:active={isActive(pane)}
				id={`tabpanel-${pane.id}`}
				role="tabpanel"
				aria-labelledby={`tab-${pane.id}`}
				aria-hidden={!isActive(pane)}
			>
				<PaneContainer {pane} visible={isActive(pane)} />
			</div>
		{/each}
	</div>
</div>

<style>
	.tab-container {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		overflow: hidden;
		background: var(--color-bg, #ffffff);
	}

	.tab-bar {
		display: flex;
		flex-wrap: nowrap;
		overflow-x: auto;
		overflow-y: hidden;
		background: var(--color-bg-secondary, #f3f4f6);
		border-bottom: 1px solid var(--color-border, #e5e7eb);
		min-height: 36px;
		flex-shrink: 0;
	}

	/* Hide scrollbar but allow scrolling */
	.tab-bar::-webkit-scrollbar {
		height: 0;
	}

	.tab-button {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 12px;
		border: none;
		background: transparent;
		cursor: pointer;
		font-size: 13px;
		color: var(--color-text-secondary, #6b7280);
		white-space: nowrap;
		transition:
			background-color 0.15s ease,
			color 0.15s ease;
		position: relative;
		min-width: 0;
		max-width: 200px;
	}

	.tab-button:hover {
		background: var(--color-bg-hover, #e5e7eb);
		color: var(--color-text, #111827);
	}

	.tab-button.active {
		background: var(--color-bg, #ffffff);
		color: var(--color-text, #111827);
		border-bottom: 2px solid var(--color-primary, #3b82f6);
	}

	.tab-button:focus-visible {
		outline: 2px solid var(--color-primary, #3b82f6);
		outline-offset: -2px;
	}

	.tab-icon {
		flex-shrink: 0;
	}

	.tab-title {
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.tab-close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 16px;
		height: 16px;
		border: none;
		background: transparent;
		border-radius: 3px;
		cursor: pointer;
		color: var(--color-text-tertiary, #9ca3af);
		flex-shrink: 0;
		margin-left: 4px;
		opacity: 0;
		transition:
			opacity 0.15s ease,
			background-color 0.15s ease;
	}

	.tab-button:hover .tab-close,
	.tab-button.active .tab-close {
		opacity: 1;
	}

	.tab-close:hover {
		background: var(--color-bg-hover, #e5e7eb);
		color: var(--color-text, #111827);
	}

	.tab-content {
		flex: 1;
		overflow: hidden;
		position: relative;
	}

	.tab-panel {
		position: absolute;
		inset: 0;
		display: none;
	}

	.tab-panel.active {
		display: block;
	}
</style>
