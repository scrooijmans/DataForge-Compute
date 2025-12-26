<script lang="ts">
	/**
	 * SplitContainer - Resizable split panel container
	 *
	 * Divides space between children with draggable resize handles.
	 * Supports both horizontal and vertical orientations.
	 *
	 * Design inspired by:
	 * - VS Code SplitView
	 * - Lumino SplitPanel
	 * - GoldenLayout RowOrColumn
	 *
	 * See DFC-chart-implementation.md Section 12 for design details.
	 */
	import { onMount } from 'svelte';
	import type { SplitNode } from '$lib/panes/layout-model';
	import { workspaceManager } from '$lib/panes/workspace-manager';
	import LayoutRenderer from './LayoutRenderer.svelte';

	interface Props {
		/** Split node data */
		splitNode: SplitNode;
		/** Depth in the layout tree */
		depth?: number;
	}

	let { splitNode, depth = 0 }: Props = $props();

	/** Container element ref */
	let containerEl: HTMLDivElement;

	/** Currently dragging resize handle index */
	let draggingIndex: number | null = $state(null);

	/** Starting position for drag */
	let dragStartPos = 0;

	/** Starting sizes for drag */
	let dragStartSizes: number[] = [];

	/** Minimum size for each pane in pixels */
	const MIN_SIZE_PX = 50;

	/**
	 * Calculate flex basis for each child based on sizes array
	 */
	function getChildStyle(index: number): string {
		const size = splitNode.sizes[index] ?? 1 / splitNode.children.length;
		return `flex: ${size} 1 0%; min-${splitNode.orientation === 'horizontal' ? 'width' : 'height'}: ${MIN_SIZE_PX}px;`;
	}

	/**
	 * Handle mouse down on resize handle
	 */
	function handleResizeStart(event: MouseEvent, index: number): void {
		event.preventDefault();
		draggingIndex = index;
		dragStartPos = splitNode.orientation === 'horizontal' ? event.clientX : event.clientY;
		dragStartSizes = [...splitNode.sizes];

		// Add global listeners
		window.addEventListener('mousemove', handleResizeMove);
		window.addEventListener('mouseup', handleResizeEnd);

		// Add dragging class to body for cursor
		document.body.classList.add('split-dragging');
	}

	/**
	 * Handle mouse move during resize
	 */
	function handleResizeMove(event: MouseEvent): void {
		if (draggingIndex === null || !containerEl) return;

		const currentPos = splitNode.orientation === 'horizontal' ? event.clientX : event.clientY;
		const containerSize =
			splitNode.orientation === 'horizontal' ? containerEl.offsetWidth : containerEl.offsetHeight;

		// Calculate delta as fraction of container
		const delta = (currentPos - dragStartPos) / containerSize;

		// Calculate new sizes
		const newSizes = [...dragStartSizes];
		const leftIndex = draggingIndex;
		const rightIndex = draggingIndex + 1;

		// Adjust sizes while respecting minimums
		const minFraction = MIN_SIZE_PX / containerSize;

		let newLeftSize = dragStartSizes[leftIndex] + delta;
		let newRightSize = dragStartSizes[rightIndex] - delta;

		// Clamp to minimum
		if (newLeftSize < minFraction) {
			newLeftSize = minFraction;
			newRightSize = dragStartSizes[leftIndex] + dragStartSizes[rightIndex] - minFraction;
		} else if (newRightSize < minFraction) {
			newRightSize = minFraction;
			newLeftSize = dragStartSizes[leftIndex] + dragStartSizes[rightIndex] - minFraction;
		}

		newSizes[leftIndex] = newLeftSize;
		newSizes[rightIndex] = newRightSize;

		// Update workspace manager
		workspaceManager.resizeSplit(splitNode.id, newSizes);
	}

	/**
	 * Handle mouse up to end resize
	 */
	function handleResizeEnd(): void {
		draggingIndex = null;
		window.removeEventListener('mousemove', handleResizeMove);
		window.removeEventListener('mouseup', handleResizeEnd);
		document.body.classList.remove('split-dragging');
	}

	// Cleanup on unmount
	onMount(() => {
		return () => {
			window.removeEventListener('mousemove', handleResizeMove);
			window.removeEventListener('mouseup', handleResizeEnd);
			document.body.classList.remove('split-dragging');
		};
	});
</script>

<div
	class="split-container split-{splitNode.orientation}"
	bind:this={containerEl}
	data-split-id={splitNode.id}
	data-depth={depth}
>
	{#each splitNode.children as child, index (child.id)}
		<div class="split-child" style={getChildStyle(index)}>
			<LayoutRenderer node={child} depth={depth + 1} />
		</div>

		{#if index < splitNode.children.length - 1}
			<div
				class="split-handle split-handle-{splitNode.orientation}"
				class:dragging={draggingIndex === index}
				role="separator"
				aria-orientation={splitNode.orientation === 'horizontal' ? 'vertical' : 'horizontal'}
				tabindex="0"
				onmousedown={(e) => handleResizeStart(e, index)}
			>
				<div class="split-handle-inner"></div>
			</div>
		{/if}
	{/each}
</div>

<style>
	.split-container {
		display: flex;
		width: 100%;
		height: 100%;
		overflow: hidden;
	}

	.split-horizontal {
		flex-direction: row;
	}

	.split-vertical {
		flex-direction: column;
	}

	.split-child {
		overflow: hidden;
		position: relative;
	}

	.split-handle {
		flex: 0 0 auto;
		background: var(--color-border, #e5e7eb);
		transition: background-color 0.15s ease;
		position: relative;
		z-index: 10;
	}

	.split-handle:hover,
	.split-handle.dragging {
		background: var(--color-primary, #3b82f6);
	}

	.split-handle-horizontal {
		width: 4px;
		cursor: col-resize;
	}

	.split-handle-vertical {
		height: 4px;
		cursor: row-resize;
	}

	.split-handle-inner {
		position: absolute;
		background: transparent;
	}

	.split-handle-horizontal .split-handle-inner {
		left: -4px;
		right: -4px;
		top: 0;
		bottom: 0;
	}

	.split-handle-vertical .split-handle-inner {
		top: -4px;
		bottom: -4px;
		left: 0;
		right: 0;
	}

	/* Focus styles for accessibility */
	.split-handle:focus {
		outline: 2px solid var(--color-primary, #3b82f6);
		outline-offset: -2px;
	}

	/* Global cursor styles when dragging */
	:global(body.split-dragging) {
		cursor: col-resize !important;
		user-select: none !important;
	}

	:global(body.split-dragging .split-vertical .split-handle) {
		cursor: row-resize !important;
	}
</style>
