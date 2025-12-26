<script lang="ts">
	/**
	 * CurveChart - Line chart for visualizing curve data (depth vs value)
	 *
	 * Displays depth on the Y-axis (inverted, geological convention - depth increases downward)
	 * and value on the X-axis. Uses native SVG for rendering.
	 */
	import type { CurveDataPoint } from '$lib/types';

	interface Props {
		/** Output curve data points */
		outputData: CurveDataPoint[];
		/** Output curve mnemonic/name */
		outputMnemonic?: string;
		/** Original input curve data for comparison (optional) */
		inputData?: CurveDataPoint[];
		/** Input curve mnemonic/name */
		inputMnemonic?: string;
		/** Chart height in pixels */
		height?: number;
		/** Additional CSS classes */
		class?: string;
	}

	let {
		outputData,
		outputMnemonic = 'Output',
		inputData,
		inputMnemonic = 'Input',
		height = 400,
		class: className = ''
	}: Props = $props();

	// Chart dimensions
	const margin = { top: 20, right: 30, bottom: 40, left: 60 };
	const width = 600;

	// Derived dimensions - use $derived for reactive props
	const plotWidth = $derived(width - margin.left - margin.right);
	const plotHeight = $derived(height - margin.top - margin.bottom);

	// Compute data bounds
	const bounds = $derived.by(() => {
		const allData = [...outputData, ...(inputData || [])];
		if (allData.length === 0) {
			return { minDepth: 0, maxDepth: 100, minValue: 0, maxValue: 100 };
		}

		const depths = allData.map((d) => d.depth).filter((d) => !isNaN(d));
		const values = allData.map((d) => d.value).filter((v): v is number => v !== null && !isNaN(v));

		const minDepth = Math.min(...depths);
		const maxDepth = Math.max(...depths);
		const minValue = values.length > 0 ? Math.min(...values) : 0;
		const maxValue = values.length > 0 ? Math.max(...values) : 100;

		// Add some padding to value range
		const valueRange = maxValue - minValue || 1;
		const paddedMinValue = minValue - valueRange * 0.05;
		const paddedMaxValue = maxValue + valueRange * 0.05;

		return { minDepth, maxDepth, minValue: paddedMinValue, maxValue: paddedMaxValue };
	});

	// Scale functions (depth is Y-axis, value is X-axis)
	const depthScale = $derived((depth: number) => {
		const range = bounds.maxDepth - bounds.minDepth || 1;
		return ((depth - bounds.minDepth) / range) * plotHeight;
	});

	const valueScale = $derived((value: number) => {
		const range = bounds.maxValue - bounds.minValue || 1;
		return ((value - bounds.minValue) / range) * plotWidth;
	});

	// Generate SVG path for a dataset
	function generatePath(data: CurveDataPoint[]): string {
		const validPoints = data.filter((d) => d.value !== null && !isNaN(d.value));
		if (validPoints.length === 0) return '';

		const pathParts: string[] = [];
		let isFirstSegment = true;

		for (let i = 0; i < validPoints.length; i++) {
			const point = validPoints[i];
			const x = valueScale(point.value!);
			const y = depthScale(point.depth);

			if (isFirstSegment) {
				pathParts.push(`M ${x} ${y}`);
				isFirstSegment = false;
			} else {
				pathParts.push(`L ${x} ${y}`);
			}
		}

		return pathParts.join(' ');
	}

	// Generate tick values
	const depthTicks = $derived.by(() => {
		const range = bounds.maxDepth - bounds.minDepth;
		const tickCount = Math.min(10, Math.max(5, Math.floor(plotHeight / 40)));
		const step = range / (tickCount - 1);
		return Array.from({ length: tickCount }, (_, i) => bounds.minDepth + i * step);
	});

	const valueTicks = $derived.by(() => {
		const range = bounds.maxValue - bounds.minValue;
		const tickCount = Math.min(8, Math.max(4, Math.floor(plotWidth / 80)));
		const step = range / (tickCount - 1);
		return Array.from({ length: tickCount }, (_, i) => bounds.minValue + i * step);
	});

	// Format number for display
	function formatNumber(n: number): string {
		if (Math.abs(n) < 0.01 || Math.abs(n) >= 10000) {
			return n.toExponential(1);
		}
		return n.toFixed(2);
	}

	// Generate paths
	const outputPath = $derived(generatePath(outputData));
	const inputPath = $derived(inputData ? generatePath(inputData) : '');
</script>

<div class="curve-chart {className}">
	<svg {width} {height} class="bg-[hsl(var(--background))]">
		<g transform="translate({margin.left}, {margin.top})">
			<!-- Grid lines -->
			<g class="grid-lines">
				<!-- Horizontal grid lines (depth) -->
				{#each depthTicks as tick}
					<line
						x1="0"
						y1={depthScale(tick)}
						x2={plotWidth}
						y2={depthScale(tick)}
						stroke="hsl(var(--border))"
						stroke-opacity="0.5"
						stroke-dasharray="2,2"
					/>
				{/each}
				<!-- Vertical grid lines (value) -->
				{#each valueTicks as tick}
					<line
						x1={valueScale(tick)}
						y1="0"
						x2={valueScale(tick)}
						y2={plotHeight}
						stroke="hsl(var(--border))"
						stroke-opacity="0.5"
						stroke-dasharray="2,2"
					/>
				{/each}
			</g>

			<!-- Axes -->
			<g class="axes">
				<!-- Y-axis (depth) -->
				<line
					x1="0"
					y1="0"
					x2="0"
					y2={plotHeight}
					stroke="hsl(var(--foreground))"
					stroke-opacity="0.5"
				/>
				<!-- X-axis (value) -->
				<line
					x1="0"
					y1={plotHeight}
					x2={plotWidth}
					y2={plotHeight}
					stroke="hsl(var(--foreground))"
					stroke-opacity="0.5"
				/>
			</g>

			<!-- Axis labels -->
			<g class="axis-labels">
				<!-- Depth labels (Y-axis) -->
				{#each depthTicks as tick}
					<text
						x="-8"
						y={depthScale(tick)}
						text-anchor="end"
						dominant-baseline="middle"
						class="fill-[hsl(var(--muted-foreground))] text-[10px]"
					>
						{formatNumber(tick)}
					</text>
				{/each}
				<!-- Value labels (X-axis) -->
				{#each valueTicks as tick}
					<text
						x={valueScale(tick)}
						y={plotHeight + 15}
						text-anchor="middle"
						class="fill-[hsl(var(--muted-foreground))] text-[10px]"
					>
						{formatNumber(tick)}
					</text>
				{/each}
			</g>

			<!-- Axis titles -->
			<text
				x="-40"
				y={plotHeight / 2}
				text-anchor="middle"
				transform="rotate(-90, -40, {plotHeight / 2})"
				class="fill-[hsl(var(--foreground))] text-xs font-medium"
			>
				Depth
			</text>
			<text
				x={plotWidth / 2}
				y={plotHeight + 32}
				text-anchor="middle"
				class="fill-[hsl(var(--foreground))] text-xs font-medium"
			>
				Value
			</text>

			<!-- Input curve (if provided) -->
			{#if inputPath}
				<path
					d={inputPath}
					fill="none"
					stroke="hsl(var(--muted-foreground))"
					stroke-width="1.5"
					stroke-opacity="0.6"
				/>
			{/if}

			<!-- Output curve -->
			{#if outputPath}
				<path
					d={outputPath}
					fill="none"
					stroke="hsl(142, 76%, 36%)"
					stroke-width="2"
				/>
			{/if}
		</g>
	</svg>

	<!-- Legend -->
	<div class="mt-2 flex items-center justify-center gap-6 text-xs">
		{#if inputData && inputData.length > 0}
			<div class="flex items-center gap-2">
				<div class="h-0.5 w-4 bg-[hsl(var(--muted-foreground))] opacity-60"></div>
				<span class="text-[hsl(var(--muted-foreground))]">{inputMnemonic}</span>
			</div>
		{/if}
		<div class="flex items-center gap-2">
			<div class="h-0.5 w-4 bg-green-600"></div>
			<span class="text-green-600 dark:text-green-400">{outputMnemonic}</span>
		</div>
	</div>
</div>
