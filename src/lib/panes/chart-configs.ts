/**
 * Chart Configuration Types
 *
 * Defines configuration schemas for different chart types.
 * Each chart type has specific configuration options and
 * constraints on which curve types can be displayed.
 *
 * See DFC-chart-implementation.md Section 12 for design details.
 */

import type { ChartType } from '$lib/charts/types';
import type { CorrelationConfig } from '$lib/charts/correlation-types';
import { createDefaultCorrelationConfig } from '$lib/charts/correlation-types';

// ============================================================================
// Base Configuration Types
// ============================================================================

/**
 * Axis binding - which curve is bound to an axis
 */
export interface AxisBinding {
	/** Curve ID to display on this axis */
	curveId: string | null;
	/** Curve mnemonic (display name) */
	mnemonic?: string;
	/** Auto-scale or fixed range */
	autoScale: boolean;
	/** Fixed min value (when autoScale is false) */
	min?: number;
	/** Fixed max value (when autoScale is false) */
	max?: number;
	/** Invert axis direction */
	inverted?: boolean;
	/** Axis label override */
	label?: string;
}

/**
 * Series styling configuration
 */
export interface SeriesStyle {
	/** Line/point color */
	color: string;
	/** Line width in pixels */
	lineWidth: number;
	/** Point size in pixels (for scatter) */
	pointSize: number;
	/** Line style */
	lineStyle: 'solid' | 'dashed' | 'dotted';
	/** Show points on line chart */
	showPoints: boolean;
	/** Fill area under line */
	fillArea: boolean;
	/** Fill opacity (0-1) */
	fillOpacity: number;
}

/**
 * Common chart options shared by all chart types
 */
export interface CommonChartOptions {
	/** Chart title */
	title: string;
	/** Show legend */
	showLegend: boolean;
	/** Show grid lines */
	showGrid: boolean;
	/** Enable zoom/pan */
	enableZoom: boolean;
	/** Enable cursor crosshair */
	showCursor: boolean;
	/** Linked group for cursor sync */
	linkedGroup?: string;
}

// ============================================================================
// Chart-Specific Configurations
// ============================================================================

/**
 * Line Chart Configuration
 *
 * Standard X-Y line chart for plotting any curve against another.
 * No restrictions on curve types.
 */
export interface LineChartConfig extends CommonChartOptions {
	type: 'line';
	/** X-axis binding (typically depth or time) */
	xAxis: AxisBinding;
	/** Y-axis binding (the curve value) */
	yAxis: AxisBinding;
	/** Series styling */
	style: SeriesStyle;
	/** Interpolation method */
	interpolation: 'linear' | 'spline' | 'step';
}

/**
 * Scatter Chart Configuration
 *
 * X-Y scatter plot for correlation analysis.
 * No restrictions on curve types.
 */
export interface ScatterChartConfig extends CommonChartOptions {
	type: 'scatter';
	/** X-axis binding */
	xAxis: AxisBinding;
	/** Y-axis binding */
	yAxis: AxisBinding;
	/** Point styling */
	style: SeriesStyle;
	/** Point shape */
	pointShape: 'circle' | 'square' | 'triangle' | 'diamond';
	/** Show trend line */
	showTrendLine: boolean;
	/** Trend line type */
	trendLineType: 'linear' | 'polynomial' | 'exponential';
}

/**
 * Histogram Configuration
 *
 * Distribution histogram for a single curve.
 * No restrictions on curve types.
 */
export interface HistogramConfig extends CommonChartOptions {
	type: 'histogram';
	/** Data source curve */
	dataCurve: AxisBinding;
	/** Number of bins */
	binCount: number;
	/** Bin width override (null = auto) */
	binWidth: number | null;
	/** Show as percentage */
	showPercentage: boolean;
	/** Bar styling */
	style: SeriesStyle;
	/** Show normal distribution overlay */
	showNormalCurve: boolean;
}

/**
 * Cross Plot Configuration
 *
 * Multi-curve cross plot with optional z-axis coloring.
 * Commonly used for petrophysical analysis.
 */
export interface CrossPlotConfig extends CommonChartOptions {
	type: 'crossplot';
	/** X-axis binding */
	xAxis: AxisBinding;
	/** Y-axis binding */
	yAxis: AxisBinding;
	/** Z-axis for color coding (optional, used when colorMode is 'curve') */
	zAxis?: AxisBinding;
	/** Point styling */
	style: SeriesStyle;
	/** Color mode: 'curve' uses Z-axis curve values, 'well' uses well color */
	colorMode: 'curve' | 'well' | 'none';
	/** Color map for z-axis (used when colorMode is 'curve') */
	colorMap: 'viridis' | 'plasma' | 'rainbow' | 'grayscale';
	/** Well color (used when colorMode is 'well', auto-assigned if not set) */
	wellColor?: string;
	/** Show regression line */
	showRegression: boolean;
	/** Regression type */
	regressionType: 'linear' | 'polynomial';
	/** Polygon regions (for cutoff analysis) */
	polygonRegions?: PolygonRegion[];
}

/**
 * Well Log Track Configuration
 *
 * Single track in a well log display.
 * May have curve type restrictions (e.g., GR track only shows gamma ray).
 */
export interface WellLogTrackConfig {
	/** Track ID */
	id: string;
	/** Track title */
	title: string;
	/** Track width in pixels */
	width: number;
	/** Curves displayed in this track */
	curves: WellLogCurveConfig[];
	/** Track-level scale (linear or logarithmic) */
	scale: 'linear' | 'logarithmic';
	/** Grid divisions */
	gridDivisions: number;
	/** Allowed curve types (null = any) */
	allowedCurveTypes?: string[];
}

/**
 * Curve configuration within a well log track
 */
export interface WellLogCurveConfig {
	/** Curve ID */
	curveId: string;
	/** Curve mnemonic */
	mnemonic: string;
	/** Display color */
	color: string;
	/** Line width */
	lineWidth: number;
	/** Min scale value */
	scaleMin: number;
	/** Max scale value */
	scaleMax: number;
	/** Fill left/right/none */
	fill: 'left' | 'right' | 'none';
	/** Fill color */
	fillColor?: string;
}

/**
 * Well Log (Linked Charts) Configuration
 *
 * Single-track well log display with depth on Y-axis.
 * Displays one curve against the depth track from the same well/parquet file.
 */
export interface WellLogConfig extends CommonChartOptions {
	type: 'welllog';
	/** Curve to plot (Y-axis values, X-axis in display since depth is on Y) */
	curve: AxisBinding;
	/** Series styling */
	style: SeriesStyle;
	/** Depth range */
	depthRange: {
		min: number | null;
		max: number | null;
		autoScale: boolean;
	};
	/** Depth axis inverted (typically true for well logs - depth increases downward) */
	depthInverted: boolean;
	/** Show depth track labels */
	showDepthTrack: boolean;
}

/**
 * Polygon region for cross plot analysis
 */
export interface PolygonRegion {
	id: string;
	name: string;
	points: Array<{ x: number; y: number }>;
	color: string;
	opacity: number;
}

// ============================================================================
// Union Type for All Chart Configs
// ============================================================================

export type ChartConfiguration =
	| LineChartConfig
	| ScatterChartConfig
	| HistogramConfig
	| CrossPlotConfig
	| WellLogConfig
	| CorrelationConfig;

// ============================================================================
// Default Configurations
// ============================================================================

export const DEFAULT_SERIES_STYLE: SeriesStyle = {
	color: '#3b82f6',
	lineWidth: 2,
	pointSize: 4,
	lineStyle: 'solid',
	showPoints: false,
	fillArea: false,
	fillOpacity: 0.3,
};

export const DEFAULT_AXIS_BINDING: AxisBinding = {
	curveId: null,
	autoScale: true,
	inverted: false,
};

export const DEFAULT_COMMON_OPTIONS: CommonChartOptions = {
	title: '',
	showLegend: true,
	showGrid: true,
	enableZoom: true,
	showCursor: true,
};

export function createDefaultLineChartConfig(): LineChartConfig {
	return {
		...DEFAULT_COMMON_OPTIONS,
		type: 'line',
		xAxis: { ...DEFAULT_AXIS_BINDING },
		yAxis: { ...DEFAULT_AXIS_BINDING },
		style: { ...DEFAULT_SERIES_STYLE },
		interpolation: 'linear',
	};
}

export function createDefaultScatterChartConfig(): ScatterChartConfig {
	return {
		...DEFAULT_COMMON_OPTIONS,
		type: 'scatter',
		xAxis: { ...DEFAULT_AXIS_BINDING },
		yAxis: { ...DEFAULT_AXIS_BINDING },
		style: { ...DEFAULT_SERIES_STYLE, showPoints: true, lineWidth: 0 },
		pointShape: 'circle',
		showTrendLine: false,
		trendLineType: 'linear',
	};
}

export function createDefaultHistogramConfig(): HistogramConfig {
	return {
		...DEFAULT_COMMON_OPTIONS,
		type: 'histogram',
		dataCurve: { ...DEFAULT_AXIS_BINDING },
		binCount: 50,
		binWidth: null,
		showPercentage: false,
		style: { ...DEFAULT_SERIES_STYLE },
		showNormalCurve: false,
	};
}

export function createDefaultCrossPlotConfig(): CrossPlotConfig {
	return {
		...DEFAULT_COMMON_OPTIONS,
		type: 'crossplot',
		xAxis: { ...DEFAULT_AXIS_BINDING },
		yAxis: { ...DEFAULT_AXIS_BINDING },
		style: { ...DEFAULT_SERIES_STYLE, showPoints: true },
		colorMode: 'none',
		colorMap: 'viridis',
		showRegression: false,
		regressionType: 'linear',
	};
}

export function createDefaultWellLogConfig(): WellLogConfig {
	return {
		...DEFAULT_COMMON_OPTIONS,
		type: 'welllog',
		curve: { ...DEFAULT_AXIS_BINDING },
		style: { ...DEFAULT_SERIES_STYLE },
		depthRange: {
			min: null,
			max: null,
			autoScale: true,
		},
		depthInverted: true,
		showDepthTrack: true,
	};
}

// ============================================================================
// Curve Type Restrictions
// ============================================================================

/**
 * Define which curve types are allowed for specific chart types or tracks
 */
export const CURVE_TYPE_RESTRICTIONS: Record<string, string[] | null> = {
	// General charts allow any curve type
	'line': null,
	'scatter': null,
	'histogram': null,
	'crossplot': null,

	// Well log track types with restrictions
	'track-gr': ['GR', 'SGR', 'CGR', 'GAMMA_RAY'],
	'track-resistivity': ['RT', 'RILD', 'RILM', 'RSFL', 'RXO', 'AT90', 'AT60', 'AT30', 'AT10'],
	'track-density': ['RHOB', 'RHOZ', 'DENSITY'],
	'track-neutron': ['NPHI', 'TNPH', 'NPOR', 'NEUTRON'],
	'track-sonic': ['DT', 'DTC', 'DTS', 'DTCO', 'SONIC'],
	'track-caliper': ['CALI', 'HCAL', 'CALIPER'],
	'track-sp': ['SP', 'SPONTANEOUS_POTENTIAL'],
	'track-porosity': ['PHIE', 'PHIT', 'POROSITY'],
	'track-saturation': ['SW', 'SWT', 'WATER_SATURATION'],
	'track-permeability': ['PERM', 'K', 'PERMEABILITY'],
};

/**
 * Check if a curve type is allowed for a given restriction key
 */
export function isCurveTypeAllowed(curveType: string | null, restrictionKey: string): boolean {
	const restrictions = CURVE_TYPE_RESTRICTIONS[restrictionKey];

	// No restrictions = allow all
	if (restrictions === null) return true;

	// No curve type provided
	if (!curveType) return false;

	// Check if curve type is in the allowed list (case-insensitive)
	return restrictions.some((r) => r.toLowerCase() === curveType.toLowerCase());
}

/**
 * Get human-readable name for a chart type
 */
export function getChartTypeName(chartType: string): string {
	const names: Record<string, string> = {
		'line': 'Line Chart',
		'scatter': 'Scatter Chart',
		'histogram': 'Histogram',
		'crossplot': 'Cross Plot',
		'welllog': 'Well Log',
		'linked-charts': 'Linked Charts',
		'correlation': 'Well Correlation',
	};
	return names[chartType] ?? chartType;
}

// Re-export correlation config factory for convenience
export { createDefaultCorrelationConfig };

/**
 * Get available color presets
 */
export const COLOR_PRESETS = [
	'#3b82f6', // blue
	'#22c55e', // green
	'#ef4444', // red
	'#f59e0b', // amber
	'#8b5cf6', // violet
	'#06b6d4', // cyan
	'#ec4899', // pink
	'#84cc16', // lime
	'#f97316', // orange
	'#6366f1', // indigo
];
