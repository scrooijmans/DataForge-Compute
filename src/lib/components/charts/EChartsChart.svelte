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
		DataZoomComponent,
		VisualMapComponent
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
		VisualMapComponent,
		LineChart,
		ScatterChart,
		CanvasRenderer
	]);

	// Color maps for crossplot visualMap
	const COLOR_MAPS: Record<string, string[]> = {
		viridis: ['#440154', '#482878', '#3e4989', '#31688e', '#26828e', '#1f9e89', '#35b779', '#6ece58', '#b5de2b', '#fde725'],
		plasma: ['#0d0887', '#46039f', '#7201a8', '#9c179e', '#bd3786', '#d8576b', '#ed7953', '#fb9f3a', '#fdca26', '#f0f921'],
		rainbow: ['#9400D3', '#4B0082', '#0000FF', '#00FF00', '#FFFF00', '#FF7F00', '#FF0000'],
		grayscale: ['#000000', '#333333', '#666666', '#999999', '#cccccc', '#ffffff']
	};

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

	// Theme colors for ECharts (canvas cannot use CSS variables, must be hex/rgb)
	// These should match the CSS theme colors
	const themeColors = {
		border: '#e5e7eb',
		mutedForeground: '#6b7280',
		foreground: '#111827',
		primary: '#3b82f6',
		popover: '#ffffff',
		popoverForeground: '#111827'
	};

	/**
	 * Downsample data for efficient rendering
	 * Uses LTTB algorithm to preserve visual characteristics
	 * @param xValues - X axis values (either depths or curve values)
	 * @param yValues - Y axis values (curve values)
	 * @param targetPoints - Target number of points after downsampling
	 * @param standardXY - If true, use standard X-Y orientation; if false, use well log orientation (value on X, depth on Y)
	 */
	function downsampleData(
		xValues: Float64Array | Float32Array,
		yValues: Float64Array | Float32Array,
		targetPoints: number,
		standardXY: boolean = false
	): [number, number][] {
		const dataLength = xValues.length;

		// For small datasets, skip downsampling
		if (dataLength <= targetPoints) {
			const result: [number, number][] = [];
			for (let i = 0; i < dataLength; i++) {
				const xVal = xValues[i];
				const yVal = yValues[i];
				// Skip NaN values
				if (!Number.isNaN(xVal) && !Number.isNaN(yVal)) {
					if (standardXY || !invertY) {
						// Standard X-Y plot (curve vs curve or standard line chart)
						result.push([xVal, yVal]);
					} else {
						// Well log orientation: value on X, depth on Y
						result.push([yVal, xVal]);
					}
				}
			}
			return result;
		}

		// Use LTTB downsampling
		if (standardXY || !invertY) {
			// Standard X-Y: X = xValues, Y = yValues
			return lttbDownsample(
				xValues,
				dataLength,
				(i) => xValues[i],
				(i) => yValues[i],
				targetPoints
			);
		} else {
			// Well log: X = value (yValues), Y = depth (xValues)
			return lttbDownsample(
				yValues,
				dataLength,
				(i) => yValues[i], // X is value
				(i) => xValues[i], // Y is depth
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

		const xField = data.fields[0];
		const yField = data.fields[1];
		const zField = data.fields.length > 2 ? data.fields[2] : null;

		// Check if this is a crossplot with color coding
		const isCrossplot = data.meta?.crossplot === true;
		const colorMode = data.meta?.colorMode ?? 'none';
		const wellColor = data.meta?.wellColor ?? defaultColors[0];

		console.log('[EChartsChart] buildOptions:', {
			isCrossplot,
			colorMode,
			colorMap: data.meta?.colorMap,
			zField: zField?.name,
			fieldsCount: data.fields.length
		});

		// Detect if this is a depth-based plot or curve-vs-curve plot
		// depthInverted: true means it's a well log (depth on Y, value on X)
		// depthInverted: false means it's a curve-vs-curve plot (standard X-Y)
		const isDepthPlot = data.meta?.depthInverted !== false && xField.name === 'DEPTH';
		const shouldInvertY = isDepthPlot && invertY;

		// Configure X axis
		const xAxisConfig = {
			type: 'value' as const,
			name: xField.config?.displayName ?? xField.name,
			nameLocation: 'middle' as const,
			nameGap: 40,
			inverse: false, // X axis is never inverted
			axisLine: { show: true, lineStyle: { color: themeColors.border } },
			axisLabel: { color: themeColors.mutedForeground, fontSize: 11 },
			splitLine: { show: true, lineStyle: { color: themeColors.border, opacity: 0.5 } }
		};

		// Configure Y axis
		const yAxisConfig = {
			type: 'value' as const,
			name: yField.config?.displayName ?? yField.name,
			nameLocation: 'middle' as const,
			nameGap: 50,
			inverse: shouldInvertY, // Only invert if it's a depth plot
			axisLine: { show: true, lineStyle: { color: themeColors.border } },
			axisLabel: { color: themeColors.mutedForeground, fontSize: 11 },
			splitLine: { show: true, lineStyle: { color: themeColors.border, opacity: 0.5 } }
		};

		// Calculate optimal sample count based on container width
		const targetPoints = calculateSampleCount(containerWidth, 2);

		// Build series data based on whether we have Z-axis for coloring
		let seriesData: Array<[number, number] | [number, number, number]>;

		if (isCrossplot && colorMode === 'curve' && zField) {
			// Crossplot with Z-axis coloring: include Z values in data
			seriesData = buildCrossPlotData(xField.values, yField.values, zField.values, targetPoints);
			console.log('[EChartsChart] Built 3D crossplot data, sample:', seriesData.slice(0, 3));
		} else {
			// Standard 2D data
			seriesData = downsampleData(xField.values, yField.values, targetPoints, !isDepthPlot);
		}

		// Find series override - match by field name, or use first entry if it has empty field (global override)
		const seriesOverride = seriesConfig?.find((s) => s.field === yField.name || s.field === '');
		const seriesColor = isCrossplot && colorMode === 'well'
			? wellColor
			: (seriesOverride?.color ?? yField.config?.color ?? defaultColors[0]);

		// Build visualMap for crossplot Z-axis coloring
		let visualMapConfig: echarts.EChartsCoreOption['visualMap'] = undefined;
		if (isCrossplot && colorMode === 'curve' && zField) {
			// Get min/max of Z values for continuous color mapping
			let zMin = Infinity;
			let zMax = -Infinity;
			for (let i = 0; i < zField.values.length; i++) {
				const v = zField.values[i];
				if (!Number.isNaN(v)) {
					if (v < zMin) zMin = v;
					if (v > zMax) zMax = v;
				}
			}

			// Get color map from data metadata, default to viridis
			const colorMapName = data.meta?.colorMap ?? 'viridis';
			const colors = COLOR_MAPS[colorMapName] ?? COLOR_MAPS.viridis;

			console.log('[EChartsChart] visualMap config:', {
				colorMapName,
				colors,
				zMin,
				zMax,
				zFieldName: zField.name
			});

			visualMapConfig = {
				type: 'continuous',
				min: zMin,
				max: zMax,
				dimension: 2, // Z-axis is the 3rd dimension (index 2)
				inRange: {
					color: colors
				},
				text: [zField.name, ''],
				textStyle: { color: themeColors.mutedForeground, fontSize: 11 },
				right: 10,
				top: 'middle',
				calculable: true
			};
		}

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
							color: themeColors.foreground
						}
					}
				: undefined,

			// Tooltip (crosshair) - simplified for performance
			tooltip: showCursor
				? {
						trigger: isCrossplot ? 'item' : 'axis',
						axisPointer: {
							type: 'line',
							lineStyle: { color: themeColors.primary, opacity: 0.5 }
						},
						backgroundColor: themeColors.popover,
						borderColor: themeColors.border,
						textStyle: { color: themeColors.popoverForeground, fontSize: 12 },
						formatter: (params: unknown) => {
							const p = Array.isArray(params) ? params[0] : params;
							if (!p || typeof p !== 'object' || !('data' in p)) return '';
							const d = p.data as number[];

							if (isCrossplot) {
								// Crossplot: show X, Y, and optionally Z
								let tooltip = `${xField.name}: ${d[0]?.toFixed(4) ?? '-'}<br/>${yField.name}: ${d[1]?.toFixed(4) ?? '-'}`;
								if (colorMode === 'curve' && zField && d.length > 2) {
									tooltip += `<br/>${zField.name}: ${d[2]?.toFixed(4) ?? '-'}`;
								}
								return tooltip;
							} else if (isDepthPlot && shouldInvertY) {
								// Well log: X = value, Y = depth
								return `${yField.name}: ${d[0]?.toFixed(4) ?? '-'}<br/>Depth: ${d[1]?.toFixed(2) ?? '-'} m`;
							} else if (isDepthPlot) {
								// Standard depth plot: X = depth, Y = value
								return `Depth: ${d[0]?.toFixed(2) ?? '-'} m<br/>${yField.name}: ${d[1]?.toFixed(4) ?? '-'}`;
							} else {
								// Curve vs curve: X = xField, Y = yField
								return `${xField.name}: ${d[0]?.toFixed(4) ?? '-'}<br/>${yField.name}: ${d[1]?.toFixed(4) ?? '-'}`;
							}
						}
					}
				: undefined,

			// VisualMap for crossplot coloring
			visualMap: visualMapConfig,

			// Grid (chart area) - add right margin for visualMap if crossplot with curve coloring
			grid: {
				left: 70,
				right: (isCrossplot && colorMode === 'curve') ? 100 : 30,
				top: title ? 50 : 30,
				bottom: enableZoom ? 60 : 40,
				containLabel: false
			},

			// Axes configuration
			// For depth plots with invertY: swap axes (value on X, depth on Y)
			// For curve-vs-curve or standard: use standard X-Y orientation
			xAxis: (isDepthPlot && shouldInvertY) ? yAxisConfig : xAxisConfig,
			yAxis: (isDepthPlot && shouldInvertY) ? { ...xAxisConfig, inverse: true } : yAxisConfig,

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
					type: (type === 'scatter' || isCrossplot) ? 'scatter' : 'line',
					data: seriesData,
					smooth: false,
					// For scatter/crossplot: show symbols; for line charts: hide them for performance
					symbol: (type === 'scatter' || isCrossplot) ? 'circle' : 'none',
					showSymbol: (type === 'scatter' || isCrossplot),
					symbolSize: (type === 'scatter' || isCrossplot) ? 4 : 0,
					lineStyle: {
						color: seriesColor,
						width: seriesOverride?.width ?? yField.config?.lineWidth ?? 1.5
					},
					itemStyle: {
						// Only set color if not using visualMap
						color: (isCrossplot && colorMode === 'curve') ? undefined : seriesColor
					},
					// Disable sampling since we pre-sample
					sampling: undefined,
					// Enable large mode for GPU acceleration (but disable for visualMap coloring as it's incompatible)
					large: !(isCrossplot && colorMode === 'curve'),
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
	 * Build crossplot data with Z-axis values for color coding
	 */
	function buildCrossPlotData(
		xValues: Float64Array | Float32Array,
		yValues: Float64Array | Float32Array,
		zValues: Float64Array | Float32Array,
		targetPoints: number
	): Array<[number, number, number]> {
		const dataLength = xValues.length;
		const result: Array<[number, number, number]> = [];

		// For small datasets, use all points
		if (dataLength <= targetPoints) {
			for (let i = 0; i < dataLength; i++) {
				const x = xValues[i];
				const y = yValues[i];
				const z = zValues[i];
				if (!Number.isNaN(x) && !Number.isNaN(y) && !Number.isNaN(z)) {
					result.push([x, y, z]);
				}
			}
			return result;
		}

		// Simple uniform sampling for crossplot (preserves Z distribution)
		const step = dataLength / targetPoints;
		for (let i = 0; i < targetPoints; i++) {
			const idx = Math.floor(i * step);
			const x = xValues[idx];
			const y = yValues[idx];
			const z = zValues[idx];
			if (!Number.isNaN(x) && !Number.isNaN(y) && !Number.isNaN(z)) {
				result.push([x, y, z]);
			}
		}

		return result;
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
		console.log('[EChartsChart] updateData setOption with visualMap:', option.visualMap ? 'yes' : 'no', option.visualMap);
		// Use notMerge: true to completely replace the option (important for visualMap updates)
		// Remove lazyUpdate to ensure immediate rendering
		chart.setOption(option, { notMerge: true, lazyUpdate: false });
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

	// React to data changes - including metadata for crossplot coloring
	$effect(() => {
		// Access these properties to ensure reactivity when they change
		const _length = data?.length;
		const _fields = data?.fields?.map(f => f.name).join(', ');
		const _colorMap = data?.meta?.colorMap;
		const _colorMode = data?.meta?.colorMode;
		const _wellColor = data?.meta?.wellColor;

		console.log('[EChartsChart] Data effect triggered, data:', data ? `${_length} points, fields: ${_fields}, colorMap: ${_colorMap}, colorMode: ${_colorMode}` : 'null');
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
