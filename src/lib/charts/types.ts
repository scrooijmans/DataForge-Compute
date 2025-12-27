/**
 * Chart System Types
 *
 * Core type definitions for the high-performance charting system.
 * See DFC-chart-implementation.md for architectural details.
 */

// ============================================================================
// Chart Types
// ============================================================================

export type ChartType = 'line' | 'scatter' | 'histogram' | 'crossplot' | 'welllog';

// ============================================================================
// Data Frame Types (Grafana-inspired)
// ============================================================================

/**
 * ChartDataFrame - Unified data structure for all chart data
 * Uses columnar layout optimized for GPU transfer
 */
export interface ChartDataFrame {
	/** Unique identifier */
	id: string;
	/** Human-readable name */
	name: string;
	/** Source information */
	source: DataFrameSource;
	/** Array of fields (columns) */
	fields: ChartField[];
	/** Total row count */
	length: number;
	/** Optional metadata */
	meta?: DataFrameMeta;
}

/**
 * ChartField - A single column of data using TypedArrays for GPU efficiency
 */
export interface ChartField {
	/** Field name (e.g., "DEPTH", "GR", "RHOB") */
	name: string;
	/** Data type */
	type: FieldType;
	/** Values - MUST be TypedArray for performance */
	values: Float64Array | Float32Array;
	/** Display configuration */
	config?: FieldConfig;
	/** Unit of measurement */
	unit?: string;
}

export type FieldType = 'number' | 'time' | 'string';

export interface FieldConfig {
	displayName?: string;
	min?: number;
	max?: number;
	color?: string;
	lineWidth?: number;
	pointSize?: number;
}

export interface DataFrameSource {
	type: 'well_curve' | 'computed' | 'imported';
	wellId?: string;
	curveId?: string;
	computationId?: string;
}

export interface DataFrameMeta {
	preferredChartType?: ChartType;
	depthInverted?: boolean;
	depthRange?: { min: number; max: number };
	/** Mark as crossplot data */
	crossplot?: boolean;
	/** Color mode for crossplot: 'curve' uses Z-axis, 'well' uses well color, 'none' for uniform color */
	colorMode?: 'curve' | 'well' | 'none';
	/** Well color for crossplot when colorMode is 'well' */
	wellColor?: string;
	/** Z-axis curve mnemonic for crossplot color dimension */
	zMnemonic?: string;
	/** Color map name for crossplot Z-axis coloring */
	colorMap?: 'viridis' | 'plasma' | 'rainbow' | 'grayscale';
}

// ============================================================================
// Chart Configuration
// ============================================================================

export interface ChartConfig {
	/** Unique chart ID */
	id: string;
	/** Chart type */
	type: ChartType;
	/** Chart title */
	title?: string;
	/** X-axis configuration */
	xAxis: AxisConfig;
	/** Y-axis configurations */
	yAxes: AxisConfig[];
	/** Series configurations */
	series: SeriesConfig[];
	/** Interaction settings */
	interaction: InteractionConfig;
}

export interface AxisConfig {
	/** Field name this axis displays */
	field: string;
	/** Axis label */
	label?: string;
	/** Is axis inverted (depth increases downward) */
	inverted?: boolean;
	/** Fixed min value */
	min?: number;
	/** Fixed max value */
	max?: number;
	/** Grid lines visible */
	grid?: boolean;
}

export interface SeriesConfig {
	/** Field name for this series */
	field: string;
	/** Display label */
	label?: string;
	/** Line/point color */
	color?: string;
	/** Line width */
	width?: number;
	/** Point size (scatter) */
	pointSize?: number;
	/** Visibility */
	visible?: boolean;
}

export interface InteractionConfig {
	/** Enable pan */
	pan?: boolean;
	/** Enable zoom */
	zoom?: boolean;
	/** Show crosshair */
	cursor?: boolean;
	/** Enable selection */
	select?: boolean;
	/** Link group for cursor sync */
	linkedGroup?: string;
}

// ============================================================================
// Chart Events
// ============================================================================

export interface Viewport {
	xMin: number;
	xMax: number;
	yMin: number;
	yMax: number;
}

export interface CursorPosition {
	x: number;
	y: number;
	dataIndex?: number;
}

// ============================================================================
// Linked Chart Types
// ============================================================================

/**
 * ChartTrack - A single track in a linked well log display
 */
export interface ChartTrack {
	id: string;
	data: ChartDataFrame;
	title: string;
	color?: string;
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Convert array of CurveDataPoint to ChartDataFrame
 */
export function curveDataToFrame(
	data: Array<{ depth: number; value: number | null }>,
	mnemonic: string,
	source: DataFrameSource
): ChartDataFrame {
	const depths = new Float64Array(data.length);
	const values = new Float64Array(data.length);

	for (let i = 0; i < data.length; i++) {
		depths[i] = data[i].depth;
		const v = data[i].value;
		values[i] = v !== null && !Number.isNaN(v) ? v : NaN;
	}

	return {
		id: `${source.type}:${source.curveId ?? source.computationId ?? 'unknown'}`,
		name: mnemonic,
		source,
		fields: [
			{
				name: 'DEPTH',
				type: 'number',
				values: depths,
				unit: 'm',
				config: { displayName: 'Depth' }
			},
			{
				name: mnemonic,
				type: 'number',
				values,
				config: { displayName: mnemonic }
			}
		],
		length: data.length,
		meta: {
			depthInverted: true,
			depthRange: {
				min: depths[0],
				max: depths[depths.length - 1]
			}
		}
	};
}
