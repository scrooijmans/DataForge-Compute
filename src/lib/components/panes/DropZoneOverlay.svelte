<script lang="ts">
	/**
	 * DropZoneOverlay - Visual feedback during pane drag operations
	 *
	 * Shows drop zone indicators when dragging panes:
	 * - Edge zones (left/right/top/bottom) for splitting
	 * - Center zone for swapping
	 * - Tab zone for adding as tab
	 *
	 * Design inspired by:
	 * - GoldenLayout's highlightArea and hoverArea
	 * - VS Code's split preview indicators
	 * - JupyterLab's DockPanel overlay
	 */
	import { onMount, onDestroy } from 'svelte';
	import { dragDropContext, type DropSegment, DROP_INDICATOR_Z_INDEX } from '$lib/panes/drag-drop-context';

	/** Current drag state */
	let dragState = dragDropContext.state;

	/** Overlay element for positioning */
	let overlayEl: HTMLDivElement;

	/** Current drop zone geometry */
	let dropZone = $derived.by(() => {
		if (!$dragState.isDragging || !$dragState.dropTargetId || !$dragState.dropSegment) {
			return null;
		}
		return dragDropContext.getDropZoneGeometry($dragState.dropTargetId, $dragState.dropSegment);
	});

	/** Get overlay style based on segment */
	function getOverlayStyle(segment: DropSegment): string {
		const baseOpacity = 0.15;
		switch (segment) {
			case 'left':
			case 'right':
			case 'top':
			case 'bottom':
				return `background: hsl(var(--primary) / ${baseOpacity}); border: 2px dashed hsl(var(--primary));`;
			case 'center':
				return `background: hsl(var(--accent) / ${baseOpacity}); border: 2px solid hsl(var(--primary));`;
			case 'tab':
				return `background: hsl(var(--primary) / ${baseOpacity * 0.5}); border-top: 3px solid hsl(var(--primary));`;
			default:
				return '';
		}
	}

	/** Get segment label */
	function getSegmentLabel(segment: DropSegment): string {
		switch (segment) {
			case 'left': return 'Split Left';
			case 'right': return 'Split Right';
			case 'top': return 'Split Top';
			case 'bottom': return 'Split Bottom';
			case 'center': return 'Swap';
			case 'tab': return 'Add as Tab';
			default: return '';
		}
	}
</script>

{#if dropZone}
	<div
		bind:this={overlayEl}
		class="drop-zone-overlay"
		style="
			left: {dropZone.x}px;
			top: {dropZone.y}px;
			width: {dropZone.width}px;
			height: {dropZone.height}px;
			z-index: {DROP_INDICATOR_Z_INDEX};
			{getOverlayStyle(dropZone.segment)}
		"
	>
		<div class="drop-zone-label">
			{getSegmentLabel(dropZone.segment)}
		</div>
	</div>
{/if}

<style>
	.drop-zone-overlay {
		position: fixed;
		pointer-events: none;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		transition: all 0.1s ease-out;
	}

	.drop-zone-label {
		padding: 6px 12px;
		background: hsl(var(--primary));
		color: hsl(var(--primary-foreground));
		font-size: 12px;
		font-weight: 500;
		border-radius: 4px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
	}
</style>
