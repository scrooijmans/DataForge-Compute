<script lang="ts">
	/**
	 * UPlotChart - High-performance chart using uPlot
	 *
	 * Renders 100K+ points at 60fps using WebGL-accelerated Canvas.
	 * Replaces SVG-based CurveChart to solve UI freezing issues.
	 */
	import { onMount, onDestroy } from 'svelte';
	import uPlot from 'uplot';
	import 'uplot/dist/uPlot.min.css';
	import type { ChartDataFrame, ChartType, SeriesConfig } from '$lib/charts/types';

	interface Props {
		/** Data frame to render */
		data: ChartDataFrame | null;
		/** Chart type */
		type?: ChartType;
		/** Chart title */
		title?: string;
		/** Chart height in pixels */
		height?: number;
		/** Series configuration overrides */
		series?: SeriesConfig[];
		/** Invert Y axis (for depth charts) */
		invertY?: boolean;
		/** Show cursor crosshair */
		showCursor?: boolean;
	}

	let {
		data,
		type = 'line',
		title,
		height = 400,
		series: seriesConfig,
		invertY = true,
		showCursor = true
	}: Props = $props();

	let container: HTMLDivElement;
	let chart: uPlot | null = null;

	// Default colors for series
	const defaultColors = [
		'#3b82f6', // blue
		'#22c55e', // green
		'#ef4444', // red
		'#f59e0b', // amber
		'#8b5cf6', // violet
		'#06b6d4' // cyan
	];

	/**
	 * Build uPlot options from props
	 */
	function buildOptions(width: number): uPlot.Options {
		const opts: uPlot.Options = {
			width,
			height,
			title,
			class: 'uplot-chart',
			scales: {
				x: {
					time: false,
					dir: invertY ? -1 : 1
				},
				y: {
					auto: true
				}
			},
			axes: [
				{
					label: data?.fields[0]?.config?.displayName ?? 'Depth',
					grid: { show: true, stroke: 'rgba(128,128,128,0.2)' },
					stroke: 'hsl(var(--foreground))',
					font: '12px system-ui',
					labelFont: '12px system-ui',
					ticks: { stroke: 'rgba(128,128,128,0.3)' }
				},
				{
					label: data?.fields[1]?.config?.displayName ?? 'Value',
					grid: { show: true, stroke: 'rgba(128,128,128,0.2)' },
					stroke: 'hsl(var(--foreground))',
					font: '12px system-ui',
					labelFont: '12px system-ui',
					ticks: { stroke: 'rgba(128,128,128,0.3)' }
				}
			],
			series: buildSeriesConfig(),
			cursor: {
				show: showCursor,
				points: {
					show: type === 'scatter',
					size: 8,
					fill: 'white',
					stroke: defaultColors[0]
				},
				focus: {
					prox: 30
				}
			},
			legend: {
				show: true
			}
		};

		return opts;
	}

	/**
	 * Build series configuration for uPlot
	 */
	function buildSeriesConfig(): uPlot.Series[] {
		const result: uPlot.Series[] = [
			{} // X axis series (no rendering config needed)
		];

		if (!data) return result;

		// Skip first field (X axis), configure rest
		for (let i = 1; i < data.fields.length; i++) {
			const field = data.fields[i];
			const override = seriesConfig?.find((s) => s.field === field.name);
			const color = override?.color ?? field.config?.color ?? defaultColors[(i - 1) % defaultColors.length];

			result.push({
				label: override?.label ?? field.config?.displayName ?? field.name,
				stroke: color,
				width: override?.width ?? field.config?.lineWidth ?? 1.5,
				points: {
					show: type === 'scatter',
					size: override?.pointSize ?? field.config?.pointSize ?? 4,
					fill: color
				},
				show: override?.visible ?? true
			});
		}

		return result;
	}

	/**
	 * Convert ChartDataFrame to uPlot data format
	 */
	function toUPlotData(frame: ChartDataFrame): uPlot.AlignedData {
		const result: number[][] = [];

		for (const field of frame.fields) {
			// Convert TypedArray to regular array for uPlot compatibility
			result.push(Array.from(field.values));
		}

		return result as uPlot.AlignedData;
	}

	/**
	 * Create or recreate the chart
	 */
	function createChart() {
		if (!container) return;

		// Destroy existing chart
		if (chart) {
			chart.destroy();
			chart = null;
		}

		const width = container.clientWidth || 800;
		const opts = buildOptions(width);

		// Create with empty data if no data provided
		const uplotData: uPlot.AlignedData = data ? toUPlotData(data) : [[0], [0]];

		chart = new uPlot(opts, uplotData, container);
	}

	/**
	 * Update chart data without recreating
	 */
	function updateData() {
		if (!chart || !data) return;

		const uplotData = toUPlotData(data);
		chart.setData(uplotData, false); // false = don't reset scales
	}

	/**
	 * Handle resize
	 */
	function handleResize() {
		if (!chart || !container) return;
		chart.setSize({ width: container.clientWidth, height });
	}

	// Create chart on mount
	onMount(() => {
		createChart();

		// Setup resize observer
		const resizeObserver = new ResizeObserver(() => {
			handleResize();
		});
		resizeObserver.observe(container);

		return () => {
			resizeObserver.disconnect();
		};
	});

	// Cleanup on destroy
	onDestroy(() => {
		if (chart) {
			chart.destroy();
			chart = null;
		}
	});

	// React to data changes
	$effect(() => {
		if (data) {
			if (chart) {
				updateData();
			} else {
				createChart();
			}
		}
	});

	// React to config changes that require recreation
	$effect(() => {
		// These changes require chart recreation
		void type;
		void height;
		void invertY;
		void seriesConfig;
		createChart();
	});
</script>

<div class="uplot-container" bind:this={container}>
	{#if !data}
		<div class="empty-state">
			<p>No data to display</p>
		</div>
	{/if}
</div>

<style>
	.uplot-container {
		width: 100%;
		min-height: 200px;
		background: hsl(var(--card));
		border-radius: 8px;
		overflow: hidden;
	}

	.uplot-container :global(.uplot) {
		font-family: inherit;
	}

	.uplot-container :global(.u-title) {
		font-size: 14px;
		font-weight: 600;
		color: hsl(var(--foreground));
	}

	.uplot-container :global(.u-legend) {
		font-size: 12px;
		padding: 8px;
	}

	.uplot-container :global(.u-legend .u-series) {
		padding: 2px 8px;
	}

	.uplot-container :global(.u-legend .u-marker) {
		width: 12px;
		height: 3px;
		border-radius: 1px;
	}

	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 200px;
		color: hsl(var(--muted-foreground));
		font-size: 14px;
	}
</style>
