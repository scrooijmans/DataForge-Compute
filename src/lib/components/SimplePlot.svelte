<script lang="ts">
	/**
	 * Simple Plot View - Basic curve visualization
	 *
	 * Renders 1-2 curves vs depth using SVG. This is a minimal
	 * implementation for the MVP - a more sophisticated charting
	 * library would be used in production.
	 */
	import type { CurveData } from '$lib/types';

	interface PlotCurve {
		data: CurveData;
		color: string;
		label?: string;
	}

	interface Props {
		curves: PlotCurve[];
		height?: number;
		showGrid?: boolean;
		depthLabel?: string;
	}

	let { curves, height = 400, showGrid = true, depthLabel = 'Depth' }: Props = $props();

	// SVG dimensions
	const marginLeft = 60;
	const marginRight = 20;
	const marginTop = 20;
	const marginBottom = 30;
	const width = 300;

	// Calculate combined ranges
	let ranges = $derived.by(() => {
		if (curves.length === 0) return null;

		let depthMin = Infinity;
		let depthMax = -Infinity;
		let valueMin = Infinity;
		let valueMax = -Infinity;

		for (const curve of curves) {
			for (const point of curve.data.data) {
				depthMin = Math.min(depthMin, point.depth);
				depthMax = Math.max(depthMax, point.depth);
				if (point.value !== null) {
					valueMin = Math.min(valueMin, point.value);
					valueMax = Math.max(valueMax, point.value);
				}
			}
		}

		// Add some padding to value range
		const valueRange = valueMax - valueMin;
		valueMin -= valueRange * 0.05;
		valueMax += valueRange * 0.05;

		return {
			depthMin,
			depthMax,
			depthRange: depthMax - depthMin,
			valueMin,
			valueMax,
			valueRange: valueMax - valueMin
		};
	});

	// Scale functions
	function scaleX(value: number): number {
		if (!ranges) return marginLeft;
		return marginLeft + ((value - ranges.valueMin) / ranges.valueRange) * (width - marginLeft - marginRight);
	}

	function scaleY(depth: number): number {
		if (!ranges) return marginTop;
		return marginTop + ((depth - ranges.depthMin) / ranges.depthRange) * (height - marginTop - marginBottom);
	}

	// Generate path for a curve
	function generatePath(data: CurveData): string {
		const points: string[] = [];
		let inPath = false;

		for (const point of data.data) {
			if (point.value !== null) {
				const x = scaleX(point.value);
				const y = scaleY(point.depth);
				if (!inPath) {
					points.push(`M ${x} ${y}`);
					inPath = true;
				} else {
					points.push(`L ${x} ${y}`);
				}
			} else {
				inPath = false;
			}
		}

		return points.join(' ');
	}

	// Generate grid lines
	function generateGridLines(): { depth: number[]; value: number[] } {
		if (!ranges) return { depth: [], value: [] };

		const depthLines: number[] = [];
		const valueLines: number[] = [];

		// Generate 5 depth lines
		const depthStep = ranges.depthRange / 4;
		for (let i = 0; i <= 4; i++) {
			depthLines.push(ranges.depthMin + i * depthStep);
		}

		// Generate 5 value lines
		const valueStep = ranges.valueRange / 4;
		for (let i = 0; i <= 4; i++) {
			valueLines.push(ranges.valueMin + i * valueStep);
		}

		return { depth: depthLines, value: valueLines };
	}

	let gridLines = $derived(generateGridLines());

	function formatValue(v: number): string {
		if (Math.abs(v) < 0.01 || Math.abs(v) >= 10000) {
			return v.toExponential(1);
		}
		return v.toFixed(1);
	}
</script>

<div class="rounded-lg border bg-[hsl(var(--card))]">
	<!-- Header -->
	<div class="border-b p-3">
		<div class="flex items-center justify-between">
			<div>
				<h3 class="text-sm font-semibold">Curve Plot</h3>
				<p class="text-xs text-[hsl(var(--muted-foreground))]">
					{curves.length === 0 ? 'No curves loaded' : curves.map((c) => c.data.mnemonic).join(', ')}
				</p>
			</div>
			{#if curves.length > 0}
				<div class="flex gap-2">
					{#each curves as curve}
						<div class="flex items-center gap-1">
							<div class="h-2 w-4 rounded" style="background-color: {curve.color}"></div>
							<span class="text-xs">{curve.label || curve.data.mnemonic}</span>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</div>

	<div class="p-3">
		{#if curves.length === 0 || !ranges}
			<div class="flex items-center justify-center" style="height: {height}px">
				<div class="text-center">
					<svg
						class="mx-auto mb-4 h-12 w-12 text-[hsl(var(--muted-foreground))] opacity-30"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="1.5"
							d="M7 12l3-3 3 3 4-4M8 21l4-4 4 4M3 4h18M4 4v16"
						/>
					</svg>
					<p class="text-sm font-medium">No Data to Plot</p>
					<p class="mt-1 text-xs text-[hsl(var(--muted-foreground))]">
						Load curve data to see the plot
					</p>
				</div>
			</div>
		{:else}
			<svg
				viewBox="0 0 {width} {height}"
				class="w-full"
				style="max-height: {height}px"
				preserveAspectRatio="xMidYMid meet"
			>
				<!-- Grid -->
				{#if showGrid}
					<g class="grid" stroke="currentColor" stroke-opacity="0.1">
						<!-- Horizontal grid lines (depth) -->
						{#each gridLines.depth as d}
							<line x1={marginLeft} y1={scaleY(d)} x2={width - marginRight} y2={scaleY(d)} />
						{/each}
						<!-- Vertical grid lines (value) -->
						{#each gridLines.value as v}
							<line x1={scaleX(v)} y1={marginTop} x2={scaleX(v)} y2={height - marginBottom} />
						{/each}
					</g>
				{/if}

				<!-- Axes -->
				<g class="axes" stroke="currentColor" stroke-opacity="0.5">
					<!-- Y axis (depth) -->
					<line
						x1={marginLeft}
						y1={marginTop}
						x2={marginLeft}
						y2={height - marginBottom}
						stroke-width="1"
					/>
					<!-- X axis (value) -->
					<line
						x1={marginLeft}
						y1={height - marginBottom}
						x2={width - marginRight}
						y2={height - marginBottom}
						stroke-width="1"
					/>
				</g>

				<!-- Depth labels -->
				<g class="depth-labels" fill="currentColor" font-size="10" text-anchor="end">
					{#each gridLines.depth as d}
						<text x={marginLeft - 5} y={scaleY(d) + 3}>{d.toFixed(0)}</text>
					{/each}
				</g>

				<!-- Value labels -->
				<g class="value-labels" fill="currentColor" font-size="10" text-anchor="middle">
					{#each gridLines.value as v, i}
						{#if i % 2 === 0}
							<text x={scaleX(v)} y={height - marginBottom + 15}>{formatValue(v)}</text>
						{/if}
					{/each}
				</g>

				<!-- Axis labels -->
				<text
					x={marginLeft / 2}
					y={height / 2}
					fill="currentColor"
					font-size="11"
					text-anchor="middle"
					transform="rotate(-90, {marginLeft / 2}, {height / 2})"
				>
					{depthLabel}
				</text>

				<!-- Curves -->
				{#each curves as curve}
					<path
						d={generatePath(curve.data)}
						fill="none"
						stroke={curve.color}
						stroke-width="1.5"
						stroke-linecap="round"
						stroke-linejoin="round"
					/>
				{/each}
			</svg>
		{/if}
	</div>
</div>
