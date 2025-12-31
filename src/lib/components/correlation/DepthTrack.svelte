<script lang="ts">
	/**
	 * DepthTrack - Shared depth axis display (leftmost column)
	 *
	 * Displays depth scale labels that align with all curve tracks.
	 * Uses SVG for precise positioning and clean rendering.
	 */

	interface Props {
		/** Depth range to display */
		depthRange: { min: number; max: number };
		/** Track height in pixels */
		height: number;
		/** Whether depth increases downward (typical for well logs) */
		inverted?: boolean;
	}

	let { depthRange, height, inverted = true }: Props = $props();

	/** Calculate nice tick values */
	function calculateTicks(min: number, max: number, targetCount: number = 10): number[] {
		const range = max - min;
		if (range <= 0) return [min];

		// Calculate a nice step size
		const rawStep = range / targetCount;
		const magnitude = Math.pow(10, Math.floor(Math.log10(rawStep)));
		const residual = rawStep / magnitude;

		let niceStep: number;
		if (residual <= 1.5) niceStep = 1 * magnitude;
		else if (residual <= 3) niceStep = 2 * magnitude;
		else if (residual <= 7) niceStep = 5 * magnitude;
		else niceStep = 10 * magnitude;

		// Generate tick values
		const ticks: number[] = [];
		const start = Math.ceil(min / niceStep) * niceStep;
		for (let v = start; v <= max; v += niceStep) {
			ticks.push(v);
		}

		return ticks;
	}

	/** Convert depth to Y position */
	function depthToY(depth: number): number {
		const { min, max } = depthRange;
		const range = max - min;
		if (range === 0) return height / 2;

		const ratio = (depth - min) / range;
		return inverted ? ratio * height : (1 - ratio) * height;
	}

	/** Format depth value for display */
	function formatDepth(depth: number): string {
		if (Math.abs(depth) >= 1000) {
			return depth.toFixed(0);
		} else if (Math.abs(depth) >= 10) {
			return depth.toFixed(1);
		} else {
			return depth.toFixed(2);
		}
	}

	let ticks = $derived(calculateTicks(depthRange.min, depthRange.max));
</script>

<div class="depth-track">
	<!-- Header -->
	<div class="track-header">
		<span class="header-label">Depth</span>
		<span class="header-unit">(m)</span>
	</div>

	<!-- Depth scale SVG -->
	<svg class="depth-scale" viewBox="0 0 60 {height}" preserveAspectRatio="none">
		<!-- Tick marks and labels -->
		{#each ticks as tick}
			{@const y = depthToY(tick)}
			<line
				x1="50"
				y1={y}
				x2="60"
				y2={y}
				stroke="currentColor"
				stroke-width="1"
				class="tick-line"
			/>
			<text
				x="48"
				y={y}
				text-anchor="end"
				dominant-baseline="middle"
				class="tick-label"
			>
				{formatDepth(tick)}
			</text>
		{/each}

		<!-- Vertical axis line -->
		<line
			x1="59"
			y1="0"
			x2="59"
			y2={height}
			stroke="currentColor"
			stroke-width="1"
			class="axis-line"
		/>
	</svg>
</div>

<style>
	.depth-track {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		background: var(--color-bg, #ffffff);
	}

	.track-header {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4px;
		height: 36px;
		border-bottom: 1px solid var(--color-border, #e5e7eb);
		background: var(--color-bg-secondary, #f9fafb);
		flex-shrink: 0;
		box-sizing: border-box;
	}

	.header-label {
		font-size: 10px;
		font-weight: 600;
		color: var(--color-text, #111827);
	}

	.header-unit {
		font-size: 9px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.depth-scale {
		flex: 1;
		width: 100%;
		color: var(--color-text-secondary, #6b7280);
	}

	.tick-line {
		opacity: 0.5;
	}

	.tick-label {
		font-size: 9px;
		fill: currentColor;
	}

	.axis-line {
		opacity: 0.3;
	}
</style>
