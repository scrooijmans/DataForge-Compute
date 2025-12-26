<script lang="ts">
	/**
	 * EChartsChart - High-performance chart using Apache ECharts
	 *
	 * Features:
	 * - Pre-render LTTB downsampling for instant rendering
	 * - GPU-accelerated canvas rendering
	 * - Native zoom/pan via dataZoom
	 * - Linked chart support via ChartManager
	 *
	 * Performance: Renders 100K+ points instantly by downsampling to ~2000 points
	 * while preserving visual fidelity.
	 */
	import { onMount, onDestroy } from 'svelte';
	import * as echarts from 'echarts/core';
	import { LineChart, ScatterChart } from 'echarts/charts';
	import {
		TitleComponent,
		TooltipComponent,
		GridComponent,
		LegendComponent,
		DataZoomComponent
	} from 'echarts/components';
	import { CanvasRenderer } from 'echarts/renderers';
	import type { ChartDataFrame, ChartType, SeriesConfig } from '$lib/charts/types';
	import { chartManager, type ChartRegistrationOptions } from '$lib/charts/chart-manager';
	import { lttbDownsample, calculateSampleCount } from '$lib/charts/downsampling';

	// Register ECharts components (tree-shaking friendly)
	echarts.use([
		TitleComponent,
		TooltipComponent,
		GridComponent,
		LegendComponent,
		DataZoomComponent,
		LineChart,
		ScatterChart,
		CanvasRenderer
	]);

	interface Props {
		/** Unique chart ID (required for linked charts) */
		id?: string;
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
		/** Invert Y axis (for depth charts - depth on Y, value on X) */
		invertY?: boolean;
		/** Show cursor/tooltip */
		showCursor?: boolean;
		/** Enable zoom/pan */
		enableZoom?: boolean;
		/** Link group ID for cursor/viewport synchronization */
		linkGroup?: string;
		/** Sync cursor across linked charts (default: true) */
		syncCursor?: boolean;
		/** Sync viewport (pan/zoom) across linked charts (default: false) */
		syncViewport?: boolean;
	}

	let {
		id = `chart-${Math.random().toString(36).slice(2, 9)}`,
		data,
		type = 'line',
		title,
		height = 400,
		series: seriesConfig,
		invertY = true,
		showCursor = true,
		enableZoom = true,
		linkGroup,
		syncCursor = true,
		syncViewport = false
	}: Props = $props();

	let container: HTMLDivElement;
	let chart: echarts.ECharts | null = null;
	let containerWidth = 800; // Default, will be updated on mount

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
	 * Downsample data for efficient rendering
	 * Uses LTTB algorithm to preserve visual characteristics
	 */
	function downsampleData(
		depths: Float64Array | Float32Array,
		values: Float64Array | Float32Array,
		targetPoints: number
	): [number, number][] {
		const dataLength = depths.length;

		// For small datasets, skip downsampling
		if (dataLength <= targetPoints) {
			const result: [number, number][] = [];
			for (let i = 0; i < dataLength; i++) {
				const value = values[i];
				if (!Number.isNaN(value)) {
					if (invertY) {
						result.push([value, depths[i]]);
					} else {
						result.push([depths[i], value]);
					}
				}
			}
			return result;
		}

		// Use LTTB downsampling
		if (invertY) {
			// For well log: X = value, Y = depth
			return lttbDownsample(
				values,
				dataLength,
				(i) => values[i], // X is value
				(i) => depths[i], // Y is depth
				targetPoints
			);
		} else {
			// Standard: X = depth, Y = value
			return lttbDownsample(
				depths,
				dataLength,
				(i) => depths[i], // X is depth
				(i) => values[i], // Y is value
				targetPoints
			);
		}
	}

	/**
	 * Build ECharts option from props
	 */
	function buildOptions(): echarts.EChartsCoreOption {
		if (!data || data.fields.length < 2) {
			return {
				title: { text: title ?? 'No Data', left: 'center' },
				xAxis: { type: 'value' },
				yAxis: { type: 'value' },
				series: []
			};
		}

		const xField = data.fields[0]; // DEPTH
		const yField = data.fields[1]; // Value

		// For well logs: depth on Y-axis (inverted), value on X-axis
		const depthAxisConfig = {
			type: 'value' as const,
			name: xField.config?.displayName ?? xField.name,
			nameLocation: 'middle' as const,
			nameGap: 40,
			inverse: invertY,
			axisLine: { show: true, lineStyle: { color: 'hsl(var(--border))' } },
			axisLabel: { color: 'hsl(var(--muted-foreground))', fontSize: 11 },
			splitLine: { show: true, lineStyle: { color: 'hsl(var(--border))', opacity: 0.5 } }
		};

		const valueAxisConfig = {
			type: 'value' as const,
			name: yField.config?.displayName ?? yField.name,
			nameLocation: 'middle' as const,
			nameGap: 50,
			axisLine: { show: true, lineStyle: { color: 'hsl(var(--border))' } },
			axisLabel: { color: 'hsl(var(--muted-foreground))', fontSize: 11 },
			splitLine: { show: true, lineStyle: { color: 'hsl(var(--border))', opacity: 0.5 } }
		};

		// Calculate optimal sample count based on container width
		const targetPoints = calculateSampleCount(containerWidth, 2);

		// Downsample data for performance
		const seriesData = downsampleData(xField.values, yField.values, targetPoints);

		const seriesOverride = seriesConfig?.find((s) => s.field === yField.name);
		const seriesColor = seriesOverride?.color ?? yField.config?.color ?? defaultColors[0];

		const option: echarts.EChartsCoreOption = {
			// Title
			title: title
				? {
						text: title,
						left: 'center',
						top: 5,
						textStyle: {
							fontSize: 14,
							fontWeight: 600,
							color: 'hsl(var(--foreground))'
						}
					}
				: undefined,

			// Tooltip (crosshair) - simplified for performance
			tooltip: showCursor
				? {
						trigger: 'axis',
						axisPointer: {
							type: 'line',
							lineStyle: { color: 'hsl(var(--primary))', opacity: 0.5 }
						},
						backgroundColor: 'hsl(var(--popover))',
						borderColor: 'hsl(var(--border))',
						textStyle: { color: 'hsl(var(--popover-foreground))', fontSize: 12 },
						formatter: (params: unknown) => {
							const p = Array.isArray(params) ? params[0] : params;
							if (!p || typeof p !== 'object' || !('data' in p)) return '';
							const d = p.data as [number, number];
							if (invertY) {
								return `${yField.name}: ${d[0]?.toFixed(4) ?? '-'}<br/>Depth: ${d[1]?.toFixed(2) ?? '-'} m`;
							}
							return `Depth: ${d[0]?.toFixed(2) ?? '-'} m<br/>${yField.name}: ${d[1]?.toFixed(4) ?? '-'}`;
						}
					}
				: undefined,

			// Grid (chart area)
			grid: {
				left: 70,
				right: 30,
				top: title ? 50 : 30,
				bottom: enableZoom ? 60 : 40,
				containLabel: false
			},

			// Axes - swap based on invertY
			xAxis: invertY ? valueAxisConfig : depthAxisConfig,
			yAxis: invertY ? depthAxisConfig : valueAxisConfig,

			// DataZoom for pan/zoom
			dataZoom: enableZoom
				? [
						{
							type: 'inside',
							xAxisIndex: 0,
							filterMode: 'none',
							throttle: 50
						},
						{
							type: 'inside',
							yAxisIndex: 0,
							filterMode: 'none',
							throttle: 50
						}
					]
				: [],

			// Series - optimized for performance
			series: [
				{
					name: seriesOverride?.label ?? yField.config?.displayName ?? yField.name,
					type: type === 'scatter' ? 'scatter' : 'line',
					data: seriesData,
					smooth: false,
					symbol: 'none', // No symbols for line charts - major perf boost
					showSymbol: false,
					lineStyle: {
						color: seriesColor,
						width: seriesOverride?.width ?? yField.config?.lineWidth ?? 1.5
					},
					itemStyle: {
						color: seriesColor
					},
					// Disable sampling since we pre-sample
					sampling: undefined,
					// Enable large mode for GPU acceleration
					large: true,
					largeThreshold: 500
				}
			],

			// Disable animation for instant rendering
			animation: false,

			// Use dirty rect rendering for better performance
			useUTC: true
		};

		return option;
	}

	/**
	 * Create or recreate the chart
	 */
	function createChart() {
		if (!container) return;

		// Update container width for sample calculation
		containerWidth = container.clientWidth || 800;

		// Unregister from chart manager if previously registered
		if (chart && linkGroup) {
			chartManager.unregisterChart(id);
		}

		// Dispose existing chart
		if (chart) {
			chart.dispose();
			chart = null;
		}

		// Initialize ECharts instance with performance options
		chart = echarts.init(container, undefined, {
			renderer: 'canvas',
			useDirtyRect: true // Enable dirty rect optimization
		});

		// Set options
		const option = buildOptions();
		chart.setOption(option);

		// Register with chart manager for linked interactions
		if (linkGroup && chart) {
			const registrationOptions: ChartRegistrationOptions = {
				syncCursor,
				syncViewport,
				syncAxis: invertY ? 'y' : 'x'
			};
			chartManager.registerChart(id, chart, linkGroup, registrationOptions);
		}
	}

	/**
	 * Update chart data without recreating
	 */
	function updateData() {
		if (!chart) {
			createChart();
			return;
		}

		// Update container width
		containerWidth = container?.clientWidth || 800;

		const option = buildOptions();
		chart.setOption(option, { notMerge: true, lazyUpdate: true });
	}

	/**
	 * Handle resize - debounced
	 */
	let resizeTimeout: ReturnType<typeof setTimeout> | null = null;

	function handleResize() {
		if (!chart || !container) return;

		// Debounce resize to avoid excessive redraws
		if (resizeTimeout) {
			clearTimeout(resizeTimeout);
		}

		resizeTimeout = setTimeout(() => {
			containerWidth = container.clientWidth || 800;
			chart?.resize();
			// Re-render with new sample count based on width
			updateData();
		}, 100);
	}

	// Create chart on mount
	onMount(() => {
		// Use requestAnimationFrame to ensure container is measured
		requestAnimationFrame(() => {
			createChart();
		});

		// Setup resize observer
		const resizeObserver = new ResizeObserver(() => {
			handleResize();
		});
		resizeObserver.observe(container);

		return () => {
			if (resizeTimeout) clearTimeout(resizeTimeout);
			resizeObserver.disconnect();
		};
	});

	// Cleanup on destroy
	onDestroy(() => {
		if (resizeTimeout) clearTimeout(resizeTimeout);

		// Unregister from chart manager
		if (linkGroup) {
			chartManager.unregisterChart(id);
		}

		if (chart) {
			chart.dispose();
			chart = null;
		}
	});

	// React to data changes
	$effect(() => {
		if (data) {
			updateData();
		}
	});

	// React to config changes that require recreation
	$effect(() => {
		void type;
		void height;
		void invertY;
		void seriesConfig;
		void title;
		void showCursor;
		void enableZoom;
		if (chart) {
			updateData();
		}
	});

	// React to link group changes
	$effect(() => {
		if (chart && linkGroup) {
			chartManager.unregisterChart(id);
			const registrationOptions: ChartRegistrationOptions = {
				syncCursor,
				syncViewport,
				syncAxis: invertY ? 'y' : 'x'
			};
			chartManager.registerChart(id, chart, linkGroup, registrationOptions);
		}
	});
</script>

<div class="echarts-container" style="height: {height}px" bind:this={container}>
	{#if !data}
		<div class="empty-state">
			<p>No data to display</p>
		</div>
	{/if}
</div>

<style>
	.echarts-container {
		width: 100%;
		min-height: 200px;
		background: hsl(var(--card));
		border-radius: 8px;
		overflow: hidden;
	}

	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: hsl(var(--muted-foreground));
		font-size: 14px;
	}
</style>
