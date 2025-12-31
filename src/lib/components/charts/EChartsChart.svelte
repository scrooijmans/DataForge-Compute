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
	import { LineChart, ScatterChart, BarChart } from 'echarts/charts';
	import {
		TitleComponent,
		TooltipComponent,
		GridComponent,
		LegendComponent,
		DataZoomComponent,
		VisualMapComponent,
		BrushComponent,
		ToolboxComponent
	} from 'echarts/components';
	import { CanvasRenderer } from 'echarts/renderers';
	import type { ChartDataFrame, ChartType, SeriesConfig } from '$lib/charts/types';
	import { segmentedCurveToSeriesData } from '$lib/charts/types';
	import type { SegmentedCurveData } from '$lib/types';
	import { chartManager, type ChartRegistrationOptions } from '$lib/charts/chart-manager';
	import { lttbDownsample, calculateSampleCount } from '$lib/charts/downsampling';
	import { cursorMode, selectionMode, getCursorStyle, type CursorMode, type SelectionMode } from '$lib/stores/chartInteraction';

	// Register ECharts components (tree-shaking friendly)
	echarts.use([
		TitleComponent,
		TooltipComponent,
		GridComponent,
		LegendComponent,
		DataZoomComponent,
		VisualMapComponent,
		BrushComponent,
		ToolboxComponent,
		LineChart,
		ScatterChart,
		BarChart,
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
		/** Data frame to render (legacy format with nulls) */
		data: ChartDataFrame | null;
		/** Segmented curve data (new format - segments without nulls) */
		segmentedData?: SegmentedCurveData | null;
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
		/** Show regression/trend line for scatter/crossplot charts */
		showRegression?: boolean;
		/** Fixed Y-axis minimum value (for shared axis across charts) */
		yAxisMin?: number;
		/** Fixed Y-axis maximum value (for shared axis across charts) */
		yAxisMax?: number;
		/** Hide Y-axis labels (when using shared depth track) */
		hideYAxis?: boolean;
		/** Fixed X-axis minimum value (for curve type standard ranges) */
		xAxisMin?: number;
		/** Fixed X-axis maximum value (for curve type standard ranges) */
		xAxisMax?: number;
		/** Use logarithmic scale for X-axis (e.g., resistivity) */
		xAxisLogScale?: boolean;
		/** Position of X-axis ('top' or 'bottom', default: 'bottom') */
		xAxisPosition?: 'top' | 'bottom';
		/** Disable downsampling - show all data points (for correlation view) */
		disableDownsampling?: boolean;
		/** Only allow Y-axis zoom (for well correlation - depth sync only) */
		yAxisOnlyZoom?: boolean;
	}

	let {
		id = `chart-${Math.random().toString(36).slice(2, 9)}`,
		data,
		segmentedData,
		type = 'line',
		title,
		height = 400,
		series: seriesConfig,
		invertY = true,
		showCursor = true,
		enableZoom = true,
		linkGroup,
		syncCursor = true,
		syncViewport = false,
		showRegression = false,
		yAxisMin,
		yAxisMax,
		hideYAxis = false,
		xAxisMin,
		xAxisMax,
		xAxisLogScale = false,
		xAxisPosition = 'bottom',
		disableDownsampling = false,
		yAxisOnlyZoom = false
	}: Props = $props();

	let container: HTMLDivElement;
	let chart: echarts.ECharts | null = null;
	let containerWidth = 800; // Default, will be updated on mount
	let isMounted = false; // Track if component is mounted and ready for chart creation
	let pendingDataUpdate = false; // Track if we have pending data to render

	// Get current cursor mode from store
	let currentCursorMode = $derived($cursorMode);
	let currentSelectionMode = $derived($selectionMode);
	let cursorStyle = $derived(getCursorStyle(currentCursorMode));

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
	 * Format large numbers in compact notation (1k, 1M, etc.)
	 * Limits display to no more than two trailing zeros
	 */
	function formatCompactNumber(value: number): string {
		const absValue = Math.abs(value);
		if (absValue >= 1000000) {
			const formatted = (value / 1000000);
			// Show one decimal if not a whole number
			return (formatted % 1 === 0 ? formatted.toFixed(0) : formatted.toFixed(1)) + 'M';
		}
		if (absValue >= 1000) {
			const formatted = (value / 1000);
			// Show one decimal if not a whole number
			return (formatted % 1 === 0 ? formatted.toFixed(0) : formatted.toFixed(1)) + 'k';
		}
		// For values under 1000, show reasonable precision
		if (absValue >= 100) {
			return value.toFixed(0);
		}
		if (absValue >= 10) {
			return value.toFixed(1);
		}
		if (absValue >= 1) {
			return value.toFixed(2);
		}
		// Very small numbers
		return value.toPrecision(3);
	}

	/**
	 * Downsample data for efficient rendering while preserving gaps
	 * Uses LTTB algorithm to preserve visual characteristics
	 * Preserves null/NaN values to prevent interpolation across missing data segments
	 *
	 * For ECharts gap handling:
	 * - Standard charts: data as [depth, value], gaps as [depth, null]
	 * - Well log charts: data as [value, depth], gaps as [null, depth]
	 * - connectNulls: false must be set on the series
	 *
	 * @param xValues - Depth values (primary axis for well log data)
	 * @param yValues - Curve values (measurement values)
	 * @param targetPoints - Target number of points after downsampling
	 * @param swapForWellLog - If true, output [value, depth] for well log display; if false, output [depth, value]
	 */
	// Type for gap markers - ECharts uses '-' string to represent gaps
	type GapMarker = '-' | null;
	type ChartDataPoint = [number, number] | [number, GapMarker] | [GapMarker, number];

	// Segment of contiguous data (no gaps)
	interface DataSegment {
		data: Array<[number, number]>;
		startDepth: number;
		endDepth: number;
	}

	/**
	 * Downsample data into segments for multi-series gap handling
	 * Returns an array of contiguous data segments, which can be rendered as separate series
	 * This approach guarantees proper gap display regardless of axis orientation
	 */
	function downsampleDataToSegments(
		xValues: Float64Array | Float32Array,
		yValues: Float64Array | Float32Array,
		targetPoints: number,
		swapForWellLog: boolean = false
	): DataSegment[] {
		const dataLength = xValues.length;

		// Find contiguous segments (regions without NaN)
		const segments: Array<{ start: number; end: number }> = [];
		let segmentStart = -1;

		for (let i = 0; i < dataLength; i++) {
			const hasValue = !Number.isNaN(yValues[i]) && !Number.isNaN(xValues[i]);
			if (hasValue) {
				if (segmentStart === -1) {
					segmentStart = i;
				}
			} else {
				if (segmentStart !== -1) {
					segments.push({ start: segmentStart, end: i });
					segmentStart = -1;
				}
			}
		}
		// Close last segment if open
		if (segmentStart !== -1) {
			segments.push({ start: segmentStart, end: dataLength });
		}

		if (segments.length === 0) {
			return [];
		}

		// Calculate points per segment proportionally
		const totalValidPoints = segments.reduce((sum, s) => sum + (s.end - s.start), 0);
		const result: DataSegment[] = [];

		for (const seg of segments) {
			const segLength = seg.end - seg.start;
			const segTargetPoints = Math.max(2, Math.round((segLength / totalValidPoints) * targetPoints));

			// Downsample this segment
			const segData = lttbDownsample(
				xValues,
				segLength,
				(i) => xValues[seg.start + i], // Depth
				(i) => yValues[seg.start + i], // Value
				segTargetPoints
			);

			// Convert to appropriate format
			const formattedData: Array<[number, number]> = [];
			if (swapForWellLog) {
				// Well log: [value, depth]
				for (const point of segData) {
					formattedData.push([point[1], point[0]]);
				}
			} else {
				// Standard: [depth, value]
				formattedData.push(...segData);
			}

			if (formattedData.length > 0) {
				result.push({
					data: formattedData,
					startDepth: xValues[seg.start],
					endDepth: xValues[seg.end - 1]
				});
			}
		}

		console.log('[downsampleDataToSegments] Created segments:', {
			segmentCount: result.length,
			segments: result.map((s, i) => ({
				index: i,
				pointCount: s.data.length,
				depthRange: [s.startDepth, s.endDepth],
				firstPoint: s.data[0],
				lastPoint: s.data[s.data.length - 1]
			}))
		});

		return result;
	}

	function downsampleData(
		xValues: Float64Array | Float32Array,
		yValues: Float64Array | Float32Array,
		targetPoints: number,
		swapForWellLog: boolean = false
	): ChartDataPoint[] {
		const dataLength = xValues.length;

		// Debug: Analyze data for gaps
		let nanXCount = 0;
		let nanYCount = 0;
		let validCount = 0;
		let firstNanYIndex = -1;
		let lastNanYIndex = -1;

		for (let i = 0; i < dataLength; i++) {
			if (Number.isNaN(xValues[i])) nanXCount++;
			if (Number.isNaN(yValues[i])) {
				nanYCount++;
				if (firstNanYIndex === -1) firstNanYIndex = i;
				lastNanYIndex = i;
			}
			if (!Number.isNaN(xValues[i]) && !Number.isNaN(yValues[i])) validCount++;
		}

		console.log('[downsampleData] Data analysis:', {
			dataLength,
			targetPoints,
			swapForWellLog,
			nanXCount,
			nanYCount,
			validCount,
			firstNanYIndex,
			lastNanYIndex,
			firstFewX: Array.from(xValues.slice(0, 5)),
			firstFewY: Array.from(yValues.slice(0, 5)),
			lastFewX: Array.from(xValues.slice(-5)),
			lastFewY: Array.from(yValues.slice(-5))
		});

		// For small datasets, skip downsampling but preserve gaps
		// Data format depends on chart type:
		// - Standard charts: [depth, value] with gaps as [depth, null]
		// - Well log charts: [value, depth] with gaps as [null, depth]
		// This ensures the "plotted" dimension has the null for gap handling
		if (dataLength <= targetPoints) {
			const result: ChartDataPoint[] = [];
			let gapMarkersAdded = 0;

			for (let i = 0; i < dataLength; i++) {
				const depthVal = xValues[i]; // Depth
				const curveVal = yValues[i]; // Curve measurement value

				// Skip if depth is invalid
				if (Number.isNaN(depthVal)) {
					continue;
				}

				if (Number.isNaN(curveVal)) {
					// Gap marker - use '-' string which ECharts recognizes as null/gap
					if (swapForWellLog) {
						// Well log: [value, depth] -> gap is ['-', depth]
						result.push(['-' as unknown as number, depthVal]);
					} else {
						// Standard: [depth, value] -> gap is [depth, '-']
						result.push([depthVal, '-' as unknown as null]);
					}
					gapMarkersAdded++;
				} else {
					if (swapForWellLog) {
						// Well log: [value, depth]
						result.push([curveVal, depthVal]);
					} else {
						// Standard: [depth, value]
						result.push([depthVal, curveVal]);
					}
				}
			}

			console.log('[downsampleData] Small dataset result:', {
				resultLength: result.length,
				gapMarkersAdded,
				swapForWellLog,
				samplePoints: result.slice(0, 3),
				hasGapMarkers: swapForWellLog
					? result.some(p => p[0] === '-' || p[0] === null)
					: result.some(p => p[1] === '-' || p[1] === null)
			});

			return result;
		}

		// For larger datasets, identify contiguous segments and downsample each
		const segments: Array<{ start: number; end: number }> = [];
		let segmentStart = -1;

		for (let i = 0; i < dataLength; i++) {
			const hasValue = !Number.isNaN(yValues[i]) && !Number.isNaN(xValues[i]);
			if (hasValue) {
				if (segmentStart === -1) {
					segmentStart = i;
				}
			} else {
				if (segmentStart !== -1) {
					segments.push({ start: segmentStart, end: i });
					segmentStart = -1;
				}
			}
		}
		// Close last segment if open
		if (segmentStart !== -1) {
			segments.push({ start: segmentStart, end: dataLength });
		}

		console.log('[downsampleData] Large dataset segments:', {
			segmentCount: segments.length,
			segments: segments.map(s => ({ start: s.start, end: s.end, length: s.end - s.start }))
		});

		// If no valid segments, return empty
		if (segments.length === 0) {
			return [];
		}

		// Calculate points per segment proportionally
		const totalValidPoints = segments.reduce((sum, s) => sum + (s.end - s.start), 0);
		const result: ChartDataPoint[] = [];

		for (let segIdx = 0; segIdx < segments.length; segIdx++) {
			const seg = segments[segIdx];
			const segLength = seg.end - seg.start;
			const segTargetPoints = Math.max(2, Math.round((segLength / totalValidPoints) * targetPoints));

			// Downsample this segment
			const segData = lttbDownsample(
				xValues,
				segLength,
				(i) => xValues[seg.start + i], // Depth
				(i) => yValues[seg.start + i], // Value
				segTargetPoints
			);

			// LTTB returns [depth, value] format, convert to appropriate format
			if (swapForWellLog) {
				// Convert to [value, depth] for well log
				for (const point of segData) {
					result.push([point[1], point[0]]); // Swap: [value, depth]
				}
			} else {
				// Keep as [depth, value] for standard charts
				result.push(...segData);
			}

			// Add gap marker between segments (if not last segment)
			if (segIdx < segments.length - 1) {
				const nextSeg = segments[segIdx + 1];
				const gapDepth = (xValues[seg.end - 1] + xValues[nextSeg.start]) / 2;

				if (swapForWellLog) {
					// Well log: [value, depth] -> gap is ['-', depth]
					console.log('[downsampleData] Adding gap marker (well log):', {
						segIdx,
						gapMarker: ['-', gapDepth]
					});
					result.push(['-' as unknown as number, gapDepth]);
				} else {
					// Standard: [depth, value] -> gap is [depth, '-']
					console.log('[downsampleData] Adding gap marker (standard):', {
						segIdx,
						gapMarker: [gapDepth, '-']
					});
					result.push([gapDepth, '-' as unknown as null]);
				}
			}
		}

		const gapCount = swapForWellLog
			? result.filter(p => p[0] === '-' || p[0] === null).length
			: result.filter(p => p[1] === '-' || p[1] === null).length;

		console.log('[downsampleData] Large dataset result:', {
			resultLength: result.length,
			swapForWellLog,
			hasGapMarkers: gapCount > 0,
			gapMarkerCount: gapCount
		});

		return result;
	}

	/**
	 * Safely get values from a field, handling both TypedArrays and regular arrays
	 */
	function getFieldValues(field: { values: Float64Array | Float32Array | number[] } | undefined): Float64Array | null {
		if (!field || !field.values) return null;
		// If already a TypedArray, return as-is
		if (field.values instanceof Float64Array || field.values instanceof Float32Array) {
			return field.values instanceof Float32Array
				? new Float64Array(field.values)
				: field.values;
		}
		// If regular array, convert to Float64Array
		if (Array.isArray(field.values)) {
			return new Float64Array(field.values);
		}
		return null;
	}

	/**
	 * Compute histogram bins from data
	 * @param values - Array of values to bin
	 * @param binCount - Number of bins
	 * @returns Array of { label: string, count: number } for each bin
	 */
	function computeHistogramBins(
		values: Float64Array,
		binCount: number = 20
	): { labels: string[]; counts: number[]; binEdges: number[] } {
		// Filter out NaN values
		const validValues: number[] = [];
		for (let i = 0; i < values.length; i++) {
			if (Number.isFinite(values[i])) {
				validValues.push(values[i]);
			}
		}

		if (validValues.length === 0) {
			return { labels: [], counts: [], binEdges: [] };
		}

		// Find min/max
		let min = validValues[0];
		let max = validValues[0];
		for (const v of validValues) {
			if (v < min) min = v;
			if (v > max) max = v;
		}

		// Handle case where all values are the same
		if (min === max) {
			return {
				labels: [min.toFixed(2)],
				counts: [validValues.length],
				binEdges: [min, max]
			};
		}

		// Create bins
		const binWidth = (max - min) / binCount;
		const counts = new Array(binCount).fill(0);
		const binEdges: number[] = [];

		for (let i = 0; i <= binCount; i++) {
			binEdges.push(min + i * binWidth);
		}

		// Count values in each bin
		for (const v of validValues) {
			let binIndex = Math.floor((v - min) / binWidth);
			// Handle edge case where value equals max
			if (binIndex >= binCount) binIndex = binCount - 1;
			counts[binIndex]++;
		}

		// Create labels for each bin (showing range)
		const labels = counts.map((_, i) => {
			const binStart = binEdges[i];
			const binEnd = binEdges[i + 1];
			return `${binStart.toFixed(2)}`;
		});

		return { labels, counts, binEdges };
	}

	/**
	 * Calculate linear regression coefficients (y = mx + b)
	 * Uses least squares method
	 * @param xValues - X axis values
	 * @param yValues - Y axis values
	 * @returns { slope, intercept, r2 } - regression coefficients and R-squared
	 */
	function calculateLinearRegression(
		xValues: Float64Array | Float32Array,
		yValues: Float64Array | Float32Array
	): { slope: number; intercept: number; r2: number; xMin: number; xMax: number } | null {
		let n = 0;
		let sumX = 0;
		let sumY = 0;
		let sumXY = 0;
		let sumXX = 0;
		let sumYY = 0;
		let xMin = Infinity;
		let xMax = -Infinity;

		// Calculate sums, filtering out NaN values
		for (let i = 0; i < xValues.length; i++) {
			const x = xValues[i];
			const y = yValues[i];
			if (Number.isFinite(x) && Number.isFinite(y)) {
				n++;
				sumX += x;
				sumY += y;
				sumXY += x * y;
				sumXX += x * x;
				sumYY += y * y;
				if (x < xMin) xMin = x;
				if (x > xMax) xMax = x;
			}
		}

		// Need at least 2 points for regression
		if (n < 2) return null;

		// Calculate slope and intercept
		const denominator = n * sumXX - sumX * sumX;
		if (Math.abs(denominator) < 1e-10) return null; // Avoid division by zero

		const slope = (n * sumXY - sumX * sumY) / denominator;
		const intercept = (sumY - slope * sumX) / n;

		// Calculate R-squared
		const meanY = sumY / n;
		let ssTot = 0;
		let ssRes = 0;
		for (let i = 0; i < xValues.length; i++) {
			const x = xValues[i];
			const y = yValues[i];
			if (Number.isFinite(x) && Number.isFinite(y)) {
				const predicted = slope * x + intercept;
				ssRes += (y - predicted) ** 2;
				ssTot += (y - meanY) ** 2;
			}
		}
		const r2 = ssTot > 0 ? 1 - ssRes / ssTot : 0;

		return { slope, intercept, r2, xMin, xMax };
	}

	/**
	 * Build histogram-specific ECharts options
	 */
	function buildHistogramOptions(
		values: Float64Array,
		fieldName: string,
		binCount: number = 20
	): echarts.EChartsCoreOption {
		const { labels, counts } = computeHistogramBins(values, binCount);

		const seriesOverride = seriesConfig?.find((s) => s.field === fieldName || s.field === '');
		const barColor = seriesOverride?.color ?? defaultColors[0];

		return {
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

			tooltip: {
				trigger: 'axis',
				axisPointer: {
					type: 'shadow'
				},
				backgroundColor: themeColors.popover,
				borderColor: themeColors.border,
				textStyle: { color: themeColors.popoverForeground, fontSize: 12 },
				formatter: (params: unknown) => {
					const p = Array.isArray(params) ? params[0] : params;
					if (!p || typeof p !== 'object' || !('name' in p) || !('value' in p)) return '';
					return `${fieldName}: ${(p as { name: string }).name}<br/>Count: ${(p as { value: number }).value}`;
				}
			},

			grid: {
				left: 60,
				right: 30,
				top: title ? 50 : 30,
				bottom: 60,
				containLabel: false
			},

			xAxis: {
				type: 'category',
				data: labels,
				name: fieldName,
				nameLocation: 'middle',
				nameGap: 40,
				axisLine: { show: true, lineStyle: { color: themeColors.border } },
				axisLabel: {
					color: themeColors.mutedForeground,
					fontSize: 10,
					rotate: 45,
					interval: Math.floor(labels.length / 10) // Show fewer labels for readability
				},
				axisTick: {
					alignWithLabel: true
				}
			},

			yAxis: {
				type: 'value',
				name: 'Count',
				nameLocation: 'middle',
				nameGap: 45,
				axisLine: { show: true, lineStyle: { color: themeColors.border } },
				axisLabel: { color: themeColors.mutedForeground, fontSize: 11 },
				splitLine: { show: true, lineStyle: { color: themeColors.border, opacity: 0.5 } }
			},

			series: [
				{
					name: fieldName,
					type: 'bar',
					barWidth: '90%',
					data: counts,
					itemStyle: {
						color: barColor
					}
				}
			],

			animation: false
		};
	}

	/**
	 * Build ECharts option from segmented curve data (new architecture)
	 * This is the preferred path - segments come pre-extracted from Rust backend
	 */
	function buildOptionsFromSegments(): echarts.EChartsCoreOption | null {
		if (!segmentedData || segmentedData.segments.length === 0) {
			return null;
		}

		console.log('[buildOptionsFromSegments] Using segmented data:', {
			curveId: segmentedData.curve_id,
			mnemonic: segmentedData.mnemonic,
			segmentCount: segmentedData.segments.length,
			totalPoints: segmentedData.total_points,
			depthRange: segmentedData.depth_range
		});

		// Check if this is a welllog chart type - always display as depth-inverted
		const isWellLog = type === 'welllog';
		const shouldInvertY = isWellLog && invertY;

		// Get series data from segments
		// When disableDownsampling is true, use Infinity to show all points
		const targetPoints = disableDownsampling
			? Infinity
			: calculateSampleCount(containerWidth, 2) / Math.max(1, segmentedData.segments.length);

		const seriesSegments = segmentedCurveToSeriesData(segmentedData, {
			swapForWellLog: shouldInvertY,
			targetPointsPerSegment: targetPoints
		});

		// Theme colors for ECharts
		const themeColorsLocal = {
			border: '#e5e7eb',
			mutedForeground: '#6b7280',
			foreground: '#111827',
			primary: '#3b82f6',
			popover: '#ffffff',
			popoverForeground: '#111827'
		};

		// Find series override
		const seriesOverride = seriesConfig?.find((s) => s.field === segmentedData.mnemonic || s.field === '');
		const seriesColor = seriesOverride?.color ?? defaultColors[0];

		// Configure axis styles
		const axisStyle = {
			type: 'value' as const,
			nameLocation: 'middle' as const,
			min: 'dataMin' as const,
			max: 'dataMax' as const,
			scale: true,
			axisLine: { show: true, lineStyle: { color: themeColorsLocal.border } },
			axisLabel: { color: themeColorsLocal.mutedForeground, fontSize: 11 },
			splitLine: { show: true, lineStyle: { color: themeColorsLocal.border, opacity: 0.5 } }
		};

		// Depth axis config - use fixed bounds if provided
		const depthAxisConfig = {
			...axisStyle,
			name: hideYAxis ? '' : 'Depth',
			nameGap: 40,
			// Use fixed min/max if provided (for shared depth axis)
			min: yAxisMin !== undefined ? yAxisMin : ('dataMin' as const),
			max: yAxisMax !== undefined ? yAxisMax : ('dataMax' as const),
			// Hide axis labels and ticks if hideYAxis is true
			axisLabel: hideYAxis
				? { show: false }
				: { color: themeColorsLocal.mutedForeground, fontSize: 11 },
			axisTick: hideYAxis ? { show: false } : undefined,
			splitLine: hideYAxis
				? { show: false }
				: { show: true, lineStyle: { color: themeColorsLocal.border, opacity: 0.5 } }
		};

		// Value axis config (X-axis in well log mode)
		const valueAxisConfig = {
			...axisStyle,
			type: xAxisLogScale ? ('log' as const) : ('value' as const),
			name: segmentedData.mnemonic + (segmentedData.unit ? ` (${segmentedData.unit})` : ''),
			nameGap: xAxisPosition === 'top' ? 25 : 50,
			nameLocation: 'middle' as const,
			min: xAxisMin !== undefined ? xAxisMin : ('dataMin' as const),
			max: xAxisMax !== undefined ? xAxisMax : ('dataMax' as const),
			// Position at top for well log correlation view
			position: xAxisPosition === 'top' ? ('top' as const) : ('bottom' as const),
			// Format axis labels with compact notation (1k, 1M)
			axisLabel: {
				color: themeColorsLocal.mutedForeground,
				fontSize: 10,
				formatter: formatCompactNumber
			}
		};

		// Build series from segments - each segment becomes a separate line series
		const seriesArray = seriesSegments.map((seg, idx) => ({
			name: seg.name,
			type: 'line' as const,
			data: seg.data,
			smooth: false,
			connectNulls: false,
			symbol: 'none',
			showSymbol: false,
			lineStyle: {
				color: seriesColor,
				width: seriesOverride?.width ?? 1.5
			},
			itemStyle: { color: seriesColor },
			large: false
		}));

		console.log('[buildOptionsFromSegments] Created', seriesArray.length, 'series from segments');

		return {
			title: title
				? {
						text: title,
						left: 'center',
						top: 5,
						textStyle: {
							fontSize: 14,
							fontWeight: 600,
							color: themeColorsLocal.foreground
						}
					}
				: undefined,

			tooltip: showCursor
				? {
						trigger: 'axis',
						axisPointer: {
							type: 'line',
							lineStyle: { color: themeColorsLocal.primary, opacity: 0.5 }
						},
						backgroundColor: themeColorsLocal.popover,
						borderColor: themeColorsLocal.border,
						textStyle: { color: themeColorsLocal.popoverForeground, fontSize: 12 },
						formatter: (params: unknown) => {
							const p = Array.isArray(params) ? params[0] : params;
							if (!p || typeof p !== 'object' || !('data' in p)) return '';
							const d = p.data as number[];
							if (shouldInvertY) {
								// Well log: X = value, Y = depth
								return `${segmentedData.mnemonic}: ${d[0]?.toFixed(4) ?? '-'}<br/>Depth: ${d[1]?.toFixed(2) ?? '-'} m`;
							} else {
								// Standard: X = depth, Y = value
								return `Depth: ${d[0]?.toFixed(2) ?? '-'} m<br/>${segmentedData.mnemonic}: ${d[1]?.toFixed(4) ?? '-'}`;
							}
						}
					}
				: undefined,

			grid: {
				left: hideYAxis ? 5 : 70,
				right: 5,
				// When X-axis is at top, increase top margin for axis labels
				top: xAxisPosition === 'top' ? 45 : (title ? 50 : 30),
				// When X-axis is at top, reduce bottom margin
				bottom: xAxisPosition === 'top' ? 5 : (enableZoom ? 60 : 40),
				containLabel: false
			},

			// Axes - for well log: X=value, Y=depth(inverted)
			xAxis: shouldInvertY
				? { ...valueAxisConfig, inverse: false }
				: { ...depthAxisConfig, inverse: false, position: xAxisPosition },
			yAxis: shouldInvertY
				? { ...depthAxisConfig, inverse: true }
				: { ...valueAxisConfig, inverse: false },

			// DataZoom configuration
			// When yAxisOnlyZoom is true (correlation view), only allow Y-axis (depth) zooming
			// X-axis (curve values) remains fixed
			dataZoom: enableZoom
				? (yAxisOnlyZoom
					? [
							// Y-axis only zoom for correlation view
							{
								type: 'inside',
								yAxisIndex: 0,
								filterMode: 'none',
								throttle: 50,
								zoomOnMouseWheel: true,
								moveOnMouseMove: currentCursorMode === 'pan',
								moveOnMouseWheel: false
							}
						]
					: [
							{
								type: 'inside',
								xAxisIndex: 0,
								filterMode: 'none',
								throttle: 50,
								zoomOnMouseWheel: currentCursorMode === 'zoom-in' || currentCursorMode === 'zoom-out' || currentCursorMode === 'pointer',
								moveOnMouseMove: currentCursorMode === 'pan',
								moveOnMouseWheel: false
							},
							{
								type: 'inside',
								yAxisIndex: 0,
								filterMode: 'none',
								throttle: 50,
								zoomOnMouseWheel: currentCursorMode === 'zoom-in' || currentCursorMode === 'zoom-out' || currentCursorMode === 'pointer',
								moveOnMouseMove: currentCursorMode === 'pan',
								moveOnMouseWheel: false
							}
						])
				: [],

			series: seriesArray,
			animation: false,
			useUTC: true
		};
	}

	/**
	 * Build ECharts option from props
	 */
	function buildOptions(): echarts.EChartsCoreOption {
		// Prefer segmented data if available (new architecture)
		const segmentedOptions = buildOptionsFromSegments();
		if (segmentedOptions) {
			return segmentedOptions;
		}

		// Fall back to legacy ChartDataFrame handling
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

		// Validate fields have values before proceeding
		if (!xField?.values || !yField?.values) {
			return {
				title: { text: title ?? 'Loading...', left: 'center' },
				xAxis: { type: 'value' },
				yAxis: { type: 'value' },
				series: []
			};
		}

		// Get values safely - handle both TypedArrays and regular arrays
		const xValues = getFieldValues(xField);
		const yValues = getFieldValues(yField);
		const zValues = zField ? getFieldValues(zField) : null;

		// Validate we have data with actual values
		if (!xValues || !yValues || xValues.length === 0 || yValues.length === 0) {
			return {
				title: { text: title ?? 'No Data', left: 'center' },
				xAxis: { type: 'value' },
				yAxis: { type: 'value' },
				series: []
			};
		}

		// Validate that the first few values are actually numbers (not undefined/NaN)
		const sampleSize = Math.min(5, xValues.length);
		let validSamples = 0;
		for (let i = 0; i < sampleSize; i++) {
			if (Number.isFinite(xValues[i]) && Number.isFinite(yValues[i])) {
				validSamples++;
			}
		}
		if (validSamples === 0) {
			return {
				title: { text: title ?? 'Invalid Data', left: 'center' },
				xAxis: { type: 'value' },
				yAxis: { type: 'value' },
				series: []
			};
		}

		// Check if this is a histogram
		if (type === 'histogram') {
			return buildHistogramOptions(yValues, yField.config?.displayName ?? yField.name);
		}

		// Check if this is a crossplot with color coding
		const isCrossplot = data.meta?.crossplot === true;
		const colorMode = data.meta?.colorMode ?? 'none';
		const wellColor = data.meta?.wellColor ?? defaultColors[0];

		// Check if this is a welllog chart type - always display as depth-inverted
		const isWellLog = type === 'welllog';

		// Detect if this is a depth-based plot or curve-vs-curve plot
		// depthInverted: true means it's a well log (depth on Y, value on X)
		// depthInverted: false means it's a curve-vs-curve plot (standard X-Y)
		// For welllog type, always treat as depth plot even if xField is not explicitly 'DEPTH'
		const isDepthPlot = isWellLog || (data.meta?.depthInverted !== false && xField.name === 'DEPTH');
		const shouldInvertY = isDepthPlot && invertY;

		// Configure axis styles - use dataMin/dataMax for automatic zoom to data extents
		const axisStyle = {
			type: 'value' as const,
			nameLocation: 'middle' as const,
			min: 'dataMin' as const,
			max: 'dataMax' as const,
			scale: true,
			axisLine: { show: true, lineStyle: { color: themeColors.border } },
			axisLabel: { color: themeColors.mutedForeground, fontSize: 11 },
			splitLine: { show: true, lineStyle: { color: themeColors.border, opacity: 0.5 } }
		};

		// Depth axis config (xField is typically DEPTH for well data)
		// Use fixed bounds if provided (for shared depth axis in correlation view)
		const depthAxisConfig = {
			...axisStyle,
			name: hideYAxis ? '' : (xField.config?.displayName ?? xField.name),
			nameGap: 40,
			// Use fixed min/max if provided (for shared depth axis)
			min: yAxisMin !== undefined ? yAxisMin : ('dataMin' as const),
			max: yAxisMax !== undefined ? yAxisMax : ('dataMax' as const),
			// Hide axis labels and ticks if hideYAxis is true
			axisLabel: hideYAxis
				? { show: false }
				: { color: themeColors.mutedForeground, fontSize: 11 },
			axisTick: hideYAxis ? { show: false } : undefined,
			splitLine: hideYAxis
				? { show: false }
				: { show: true, lineStyle: { color: themeColors.border, opacity: 0.5 } }
		};

		// Value axis config (yField is the curve measurement, X-axis in well log mode)
		const valueAxisConfig = {
			...axisStyle,
			type: xAxisLogScale ? ('log' as const) : ('value' as const),
			name: yField.config?.displayName ?? yField.name,
			nameGap: xAxisPosition === 'top' ? 25 : 50,
			nameLocation: 'middle' as const,
			min: xAxisMin !== undefined ? xAxisMin : ('dataMin' as const),
			max: xAxisMax !== undefined ? xAxisMax : ('dataMax' as const),
			// Position at top for well log correlation view
			position: xAxisPosition === 'top' ? ('top' as const) : ('bottom' as const)
		};

		// Calculate optimal sample count based on container width
		const targetPoints = calculateSampleCount(containerWidth, 2);

		// Determine if we need to swap data for well log display
		const useWellLogFormat = isDepthPlot && shouldInvertY;

		// For line/welllog charts, use segment-based approach for proper gap handling
		// For scatter/crossplot, use single series
		const isLineChart = type === 'line' || type === 'welllog';
		let dataSegments: DataSegment[] = [];
		let seriesData: Array<[number, number, number]> | null = null;

		if (isCrossplot && colorMode === 'curve' && zValues) {
			// Crossplot with Z-axis coloring: include Z values in data
			seriesData = buildCrossPlotData(xValues, yValues, zValues, targetPoints);
		} else if (isLineChart) {
			// Line charts: use segments for gap handling
			dataSegments = downsampleDataToSegments(xValues, yValues, targetPoints, useWellLogFormat);
		} else {
			// Scatter plots: use single series, no gap handling needed
			const flatData = downsampleData(xValues, yValues, targetPoints, useWellLogFormat);
			// Convert to segments format for consistency
			dataSegments = [{
				data: flatData.filter((p): p is [number, number] =>
					typeof p[0] === 'number' && typeof p[1] === 'number'
				),
				startDepth: xValues[0],
				endDepth: xValues[xValues.length - 1]
			}];
		}

		console.log('[buildOptions] Series data built:', {
			chartType: type,
			isWellLog,
			isDepthPlot,
			useWellLogFormat,
			isLineChart,
			isCrossplot,
			xFieldName: xField.name,
			yFieldName: yField.name,
			segmentCount: dataSegments.length,
			segmentDetails: dataSegments.map((s, i) => ({
				index: i,
				pointCount: s.data.length,
				depthRange: [s.startDepth, s.endDepth],
				firstPoint: s.data[0],
				lastPoint: s.data[s.data.length - 1]
			})),
			crossplotDataLength: seriesData?.length
		});

		// Find series override - match by field name, or use first entry if it has empty field (global override)
		const seriesOverride = seriesConfig?.find((s) => s.field === yField.name || s.field === '');
		const seriesColor = isCrossplot && colorMode === 'well'
			? wellColor
			: (seriesOverride?.color ?? yField.config?.color ?? defaultColors[0]);

		// Build visualMap for crossplot Z-axis coloring
		let visualMapConfig: echarts.EChartsCoreOption['visualMap'] = undefined;
		if (isCrossplot && colorMode === 'curve' && zValues && zValues.length > 0) {
			// Get min/max of Z values for continuous color mapping
			let zMin = Infinity;
			let zMax = -Infinity;
			for (let i = 0; i < zValues.length; i++) {
				const v = zValues[i];
				if (!Number.isNaN(v)) {
					if (v < zMin) zMin = v;
					if (v > zMax) zMax = v;
				}
			}

			// Guard against invalid min/max (all NaN values)
			if (!Number.isFinite(zMin) || !Number.isFinite(zMax)) {
				zMin = 0;
				zMax = 1;
			}

			// Get color map from data metadata, default to viridis
			const colorMapName = data.meta?.colorMap ?? 'viridis';
			const colors = COLOR_MAPS[colorMapName] ?? COLOR_MAPS.viridis;

			const zFieldName = zField?.name ?? 'Z';

			visualMapConfig = {
				type: 'continuous',
				min: zMin,
				max: zMax,
				dimension: 2, // Z-axis is the 3rd dimension (index 2)
				inRange: {
					color: colors
				},
				text: [zFieldName, ''],
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
							} else if (isWellLog || (isDepthPlot && shouldInvertY)) {
								// Well log: X = value, Y = depth
								const depthLabel = xField.name === 'DEPTH' ? 'Depth' : xField.name;
								return `${yField.name}: ${d[0]?.toFixed(4) ?? '-'}<br/>${depthLabel}: ${d[1]?.toFixed(2) ?? '-'} m`;
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
				left: hideYAxis ? 5 : 70,
				right: (isCrossplot && colorMode === 'curve') ? 100 : 5,
				// When X-axis is at top, increase top margin for axis labels
				top: xAxisPosition === 'top' ? 45 : (title ? 50 : 30),
				// When X-axis is at top, reduce bottom margin
				bottom: xAxisPosition === 'top' ? 5 : (enableZoom ? 60 : 40),
				containLabel: false
			},

			// Axes configuration
			// For well log (shouldInvertY): data is [value, depth], so X=value, Y=depth (inverted)
			// For standard charts: data is [depth, value], so X=depth, Y=value
			xAxis: shouldInvertY
				? { ...valueAxisConfig, inverse: false } // Well log: X-axis shows value (not inverted)
				: { ...depthAxisConfig, inverse: false, position: xAxisPosition }, // Standard: X-axis shows depth
			yAxis: shouldInvertY
				? { ...depthAxisConfig, inverse: true } // Well log: Y-axis shows depth (inverted - increases downward)
				: { ...valueAxisConfig, inverse: false }, // Standard: Y-axis shows value

			// DataZoom for pan/zoom - configure based on cursor mode
			// When yAxisOnlyZoom is true (correlation view), only allow Y-axis (depth) zooming
			dataZoom: enableZoom
				? (yAxisOnlyZoom
					? [
							// Y-axis only zoom for correlation view
							{
								type: 'inside',
								yAxisIndex: 0,
								filterMode: 'none',
								throttle: 50,
								zoomOnMouseWheel: true,
								moveOnMouseMove: currentCursorMode === 'pan',
								moveOnMouseWheel: false
							}
						]
					: [
							{
								type: 'inside',
								xAxisIndex: 0,
								filterMode: 'none',
								throttle: 50,
								// Zoom only enabled when cursor mode is zoom-in or zoom-out
								zoomOnMouseWheel: currentCursorMode === 'zoom-in' || currentCursorMode === 'zoom-out' || currentCursorMode === 'pointer',
								// Move only enabled when cursor mode is pan
								moveOnMouseMove: currentCursorMode === 'pan',
								moveOnMouseWheel: false
							},
							{
								type: 'inside',
								yAxisIndex: 0,
								filterMode: 'none',
								throttle: 50,
								zoomOnMouseWheel: currentCursorMode === 'zoom-in' || currentCursorMode === 'zoom-out' || currentCursorMode === 'pointer',
								moveOnMouseMove: currentCursorMode === 'pan',
								moveOnMouseWheel: false
							}
						])
				: [],

			// Brush component for crossplot selection
			brush: isCrossplot && currentSelectionMode !== 'none'
				? {
						toolbox: ['rect', 'polygon', 'clear'],
						brushType: currentSelectionMode === 'rect' ? 'rect' : 'polygon',
						brushMode: 'single',
						xAxisIndex: 0,
						yAxisIndex: 0,
						brushStyle: {
							borderWidth: 2,
							color: 'rgba(59, 130, 246, 0.2)', // primary blue with low opacity
							borderColor: 'rgba(59, 130, 246, 0.8)'
						},
						outOfBrush: {
							colorAlpha: 0.3
						},
						throttleType: 'debounce',
						throttleDelay: 100
					}
				: undefined,

			// Series - optimized for performance
			// For line charts with gaps, create one series per segment
			// For scatter/crossplot, use single series
			series: (() => {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				const seriesArray: any[] = [];
				const seriesName = seriesOverride?.label ?? yField.config?.displayName ?? yField.name;

				if (isCrossplot && seriesData) {
					// Crossplot with Z-coloring: single series with all data
					seriesArray.push({
						name: seriesName,
						type: 'scatter',
						data: seriesData,
						symbol: 'circle',
						showSymbol: true,
						symbolSize: seriesOverride?.pointSize ?? yField.config?.pointSize ?? 4,
						itemStyle: {
							color: colorMode === 'curve' ? undefined : seriesColor
						},
						large: false
					});
				} else if (type === 'scatter') {
					// Scatter plot: single series from first segment
					const scatterData = dataSegments.length > 0 ? dataSegments[0].data : [];
					seriesArray.push({
						name: seriesName,
						type: 'scatter',
						data: scatterData,
						symbol: 'circle',
						showSymbol: true,
						symbolSize: seriesOverride?.pointSize ?? yField.config?.pointSize ?? 4,
						itemStyle: { color: seriesColor },
						large: false
					});
				} else {
					// Line/welllog charts: one series per segment for gap handling
					// Each segment is a separate line series - gaps appear as visual breaks
					console.log('[buildOptions] Creating line series for', dataSegments.length, 'segments');

					for (let segIdx = 0; segIdx < dataSegments.length; segIdx++) {
						const segment = dataSegments[segIdx];

						// Debug: log each segment being added
						console.log(`[buildOptions] Segment ${segIdx}:`, {
							pointCount: segment.data.length,
							depthRange: [segment.startDepth, segment.endDepth],
							firstPoint: segment.data[0],
							lastPoint: segment.data[segment.data.length - 1]
						});

						seriesArray.push({
							// Use unique name for each segment to ensure they render separately
							name: dataSegments.length > 1 ? `${seriesName} (${segIdx + 1})` : seriesName,
							type: 'line',
							data: segment.data,
							smooth: false,
							connectNulls: false,
							symbol: 'none',
							showSymbol: false,
							lineStyle: {
								color: seriesColor,
								width: seriesOverride?.width ?? yField.config?.lineWidth ?? 1.5
							},
							itemStyle: { color: seriesColor },
							sampling: undefined,
							large: false
						});
					}

					console.log('[buildOptions] Created series for segments:', {
						segmentCount: dataSegments.length,
						seriesCount: seriesArray.length,
						seriesNames: seriesArray.map(s => s.name)
					});
				}

				// Add regression line for scatter/crossplot if enabled
				if (showRegression && (type === 'scatter' || isCrossplot) && xValues && yValues) {
					const regression = calculateLinearRegression(xValues, yValues);
					if (regression) {
						const { slope, intercept, r2, xMin, xMax } = regression;
						// Create line data with two points spanning the X range
						const regressionData: [number, number][] = [
							[xMin, slope * xMin + intercept],
							[xMax, slope * xMax + intercept]
						];

						seriesArray.push({
							name: `Regression (R²=${r2.toFixed(3)})`,
							type: 'line',
							data: regressionData,
							smooth: false,
							symbol: 'none',
							showSymbol: false,
							lineStyle: {
								color: '#ef4444', // Red color for regression line
								width: 2,
								type: 'dashed'
							},
							// Regression line should not be affected by visualMap
							visualMap: false,
							// Don't show in tooltip
							tooltip: { show: false }
						});
					}
				}

				return seriesArray as echarts.EChartsCoreOption['series'];
			})(),

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
	 * This should only be called when the container is ready and has dimensions
	 */
	function createChart() {
		if (!container) return;

		// Check if container has valid dimensions
		const containerWidth = container.clientWidth;
		const containerHeight = container.clientHeight;

		if (containerWidth === 0 || containerHeight === 0) {
			// Set pending flag so we retry when resize observer fires
			pendingDataUpdate = true;
			return;
		}

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
		// Pass width and height explicitly to ensure proper sizing
		chart = echarts.init(container, undefined, {
			renderer: 'canvas',
			useDirtyRect: true, // Enable dirty rect optimization
			width: containerWidth,
			height: containerHeight
		});

		// Set options - wrap in RAF to avoid "main process" errors
		requestAnimationFrame(() => {
			if (!chart) return;
			const option = buildOptions();
			chart.setOption(option);
		});

		// Listen for brush selection events (crossplot only)
		chart.on('brushSelected', (params: unknown) => {
			const brushParams = params as {
				batch?: Array<{
					selected?: Array<{
						seriesIndex: number;
						dataIndex: number[];
					}>;
				}>;
			};

			if (!brushParams.batch || brushParams.batch.length === 0) return;

			const selected = brushParams.batch[0]?.selected;
			if (!selected || selected.length === 0) return;

			// Count total selected points across all series
			let totalSelected = 0;
			for (const series of selected) {
				totalSelected += series.dataIndex?.length ?? 0;
			}

			if (totalSelected > 0) {
				console.log(`Brush selection: ${totalSelected} points selected`);
			}
		});

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
	 * Uses requestAnimationFrame to avoid ECharts "main process" errors
	 */
	function updateData() {
		requestAnimationFrame(() => {
			if (!chart) {
				createChart();
				return;
			}

			// Update container width
			containerWidth = container?.clientWidth || 800;

			const option = buildOptions();
			// Use notMerge: true to completely replace the option (important for visualMap updates)
			chart.setOption(option, { notMerge: true, lazyUpdate: false });
		});
	}

	/**
	 * Handle resize - debounced
	 */
	let resizeTimeout: ReturnType<typeof setTimeout> | null = null;

	function handleResize() {
		if (!container) return;

		// Debounce resize to avoid excessive redraws
		if (resizeTimeout) {
			clearTimeout(resizeTimeout);
		}

		resizeTimeout = setTimeout(() => {
			// Use requestAnimationFrame to avoid ECharts "main process" errors
			requestAnimationFrame(() => {
				if (!container) return;

				const newWidth = container.clientWidth || 800;
				const newHeight = container.clientHeight || height || 400;

				// If we have a pending update and now have valid dimensions, create the chart
				if (pendingDataUpdate && !chart && newWidth > 0 && newHeight > 0) {
					pendingDataUpdate = false;
					createChart();
					return;
				}

				if (!chart) return;

				containerWidth = newWidth;
				chart.resize({
					width: newWidth,
					height: newHeight
				});
				// Re-render with new sample count based on width
				updateData();
			});
		}, 100);
	}

	// Create chart on mount
	onMount(() => {
		isMounted = true;

		// Setup resize observer FIRST - this will catch when the container gets sized
		const resizeObserver = new ResizeObserver((entries) => {
			for (const entry of entries) {
				const { width, height } = entry.contentRect;

				// If chart doesn't exist yet but we have dimensions, create it
				if (!chart && width > 0 && height > 0) {
					// Use RAF to ensure we're outside the ResizeObserver callback
					requestAnimationFrame(() => {
						if (!chart) {
							createChart();
						}
					});
				} else if (chart) {
					handleResize();
				}
			}
		});
		resizeObserver.observe(container);

		// Also try to create chart after a short delay (for cases where ResizeObserver might not fire)
		requestAnimationFrame(() => {
			requestAnimationFrame(() => {
				if (!chart && container.clientWidth > 0 && container.clientHeight > 0) {
					createChart();
				}
			});
		});

		return () => {
			isMounted = false;
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
		// Track legacy ChartDataFrame changes
		void data?.id; // Important: track data ID changes (different curve)
		void data?.name; // Track curve name changes
		void data?.length;
		void data?.fields?.map(f => f.name);
		void data?.meta?.colorMap;
		void data?.meta?.colorMode;
		void data?.meta?.wellColor;

		// Track segmented data changes (new architecture)
		void segmentedData?.curve_id;
		void segmentedData?.mnemonic;
		void segmentedData?.total_points;
		void segmentedData?.segments?.length;

		// Need at least one data source
		if (!data && !segmentedData) return;

		console.log('[EChartsChart] Data effect triggered:', {
			dataId: data?.id,
			dataName: data?.name,
			length: data?.length,
			segmentedCurveId: segmentedData?.curve_id,
			segmentedMnemonic: segmentedData?.mnemonic,
			segmentCount: segmentedData?.segments?.length,
			isMounted,
			hasChart: !!chart
		});

		// If mounted and chart exists, update it
		if (isMounted && chart) {
			updateData();
		} else if (isMounted && !chart) {
			// Chart not created yet, mark pending
			pendingDataUpdate = true;
		}
		// If not mounted yet, the onMount handler will take care of it
	});

	// React to config changes that require recreation
	$effect(() => {
		void type;
		void height;
		void invertY;
		// Access nested properties to ensure reactivity
		void seriesConfig?.map(s => `${s.field}-${s.color}-${s.width}-${s.pointSize}`);
		void title;
		void showCursor;
		void enableZoom;
		void showRegression;
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

	// React to cursor mode changes - update dataZoom behavior
	$effect(() => {
		void currentCursorMode;
		if (chart && enableZoom) {
			// Update dataZoom configuration based on cursor mode
			requestAnimationFrame(() => {
				if (!chart) return;
				chart.setOption({
					dataZoom: [
						{
							type: 'inside',
							xAxisIndex: 0,
							zoomOnMouseWheel: currentCursorMode === 'zoom-in' || currentCursorMode === 'zoom-out' || currentCursorMode === 'pointer',
							moveOnMouseMove: currentCursorMode === 'pan',
							moveOnMouseWheel: false
						},
						{
							type: 'inside',
							yAxisIndex: 0,
							zoomOnMouseWheel: currentCursorMode === 'zoom-in' || currentCursorMode === 'zoom-out' || currentCursorMode === 'pointer',
							moveOnMouseMove: currentCursorMode === 'pan',
							moveOnMouseWheel: false
						}
					]
				});
			});
		}
	});

	// React to selection mode changes - update brush behavior for crossplot
	$effect(() => {
		void currentSelectionMode;
		if (!chart) return;

		// Check if this is a crossplot
		const isCrossplot = data?.meta?.crossplot === true;
		if (!isCrossplot) return;

		requestAnimationFrame(() => {
			if (!chart) return;

			if (currentSelectionMode === 'none') {
				// Disable brush mode
				chart.dispatchAction({
					type: 'takeGlobalCursor',
					key: 'brush',
					brushOption: {
						brushType: false
					}
				});
				// Clear the brush configuration
				chart.setOption({
					brush: undefined
				}, { notMerge: false });
			} else {
				// Enable brush mode with the selected type
				const brushType = currentSelectionMode === 'rect' ? 'rect' : 'polygon';

				// Update brush configuration
				chart.setOption({
					brush: {
						toolbox: ['rect', 'polygon', 'clear'],
						brushType: brushType,
						brushMode: 'single',
						xAxisIndex: 0,
						yAxisIndex: 0,
						brushStyle: {
							borderWidth: 2,
							color: 'rgba(59, 130, 246, 0.2)',
							borderColor: 'rgba(59, 130, 246, 0.8)'
						},
						outOfBrush: {
							colorAlpha: 0.3
						},
						throttleType: 'debounce',
						throttleDelay: 100
					}
				}, { notMerge: false });

				// Activate the brush cursor
				chart.dispatchAction({
					type: 'takeGlobalCursor',
					key: 'brush',
					brushOption: {
						brushType: brushType,
						brushMode: 'single'
					}
				});
			}
		});
	});
</script>

<div class="echarts-container" style="height: {height}px; cursor: {cursorStyle};" bind:this={container}>
	{#if !data && !segmentedData}
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
