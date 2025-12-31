<script lang="ts">
	/**
	 * WellTopOverlay - Renders horizontal well top lines across tracks
	 *
	 * Well tops are stratigraphic markers (formation boundaries) that
	 * appear as horizontal lines at specific depths.
	 */
	import type { WellTop } from '$lib/charts/correlation-types';

	interface Props {
		/** Well tops to display */
		tops: WellTop[];
		/** Depth range for positioning */
		depthRange: { min: number; max: number };
		/** Overlay height in pixels */
		height: number;
	}

	let { tops, depthRange, height }: Props = $props();

	/** Convert depth to Y position (assuming inverted Y-axis) */
	function depthToY(depth: number): number {
		const { min, max } = depthRange;
		const range = max - min;
		if (range === 0) return height / 2;
		const ratio = (depth - min) / range;
		return ratio * height;
	}

	/** Get line style for a well top */
	function getLineStyle(top: WellTop): string {
		const style = top.lineStyle ?? 'solid';
		switch (style) {
			case 'dashed':
				return '6 4';
			case 'dotted':
				return '2 2';
			default:
				return 'none';
		}
	}
</script>

<div class="well-top-overlay">
	<svg class="overlay-svg" viewBox="0 0 100 {height}" preserveAspectRatio="none">
		{#each tops as top (top.name)}
			{@const y = depthToY(top.depth)}
			{#if y >= 0 && y <= height}
				<!-- Well top line -->
				<line
					x1="0"
					y1={y}
					x2="100"
					y2={y}
					stroke={top.color}
					stroke-width={top.lineWidth ?? 1}
					stroke-dasharray={getLineStyle(top)}
				/>
			{/if}
		{/each}
	</svg>

	<!-- Labels (positioned absolutely) -->
	{#each tops as top (top.name)}
		{@const y = depthToY(top.depth)}
		{#if y >= 0 && y <= height && top.showLabel !== false}
			<div
				class="top-label"
				class:left={top.labelPosition === 'left' || !top.labelPosition}
				class:right={top.labelPosition === 'right'}
				class:center={top.labelPosition === 'center'}
				style="top: {y}px; color: {top.color}"
			>
				{top.name}
			</div>
		{/if}
	{/each}
</div>

<style>
	.well-top-overlay {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		pointer-events: none;
		z-index: 5;
	}

	.overlay-svg {
		width: 100%;
		height: 100%;
	}

	.top-label {
		position: absolute;
		font-size: 9px;
		font-weight: 500;
		background: rgba(255, 255, 255, 0.9);
		padding: 1px 4px;
		border-radius: 2px;
		white-space: nowrap;
		transform: translateY(-50%);
	}

	.top-label.left {
		left: 4px;
	}

	.top-label.right {
		right: 4px;
	}

	.top-label.center {
		left: 50%;
		transform: translate(-50%, -50%);
	}
</style>
