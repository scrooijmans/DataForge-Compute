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
// Segment-Based Curve Types (OSDU-inspired architecture)
// ============================================================================

/**
 * CurveSegment - A contiguous run of valid (non-null) data points
 *
 * Key insight: Instead of storing curves as "arrays with holes",
 * we represent them as "sets of valid segments over depth".
 * This eliminates null handling complexity in the chart layer.
 *
 * Uses TypedArrays for GPU efficiency and fast rendering.
 */
export interface CurveSegment {
	/** Starting depth of this segment */
	depthStart: number;
	/** Ending depth of this segment */
	depthEnd: number;
	/** Depth values for this segment (TypedArray for performance) */
	depths: Float64Array | number[];
	/** Measurement values for this segment (TypedArray for performance) */
	values: Float64Array | number[];
	/** Number of points in this segment */
	length: number;
}

/**
 * SegmentedCurve - A curve represented as valid segments
 *
 * OSDU-inspired design: Complexity handling (null filtering, gap detection)
 * happens at access time in the Rust backend, not at ingestion.
 * Charts receive only valid data segments - rendering becomes trivial.
 *
 * Benefits:
 * - No null handling in frontend chart code
 * - Reduced data transfer (only valid points cross IPC boundary)
 * - Automatic gap display (separate series = visual gaps)
 * - Cleaner mental model: Curve = segments, not array with holes
 */
export interface SegmentedCurve {
	/** Curve identifier (e.g., "datasetId:mnemonic") */
	id: string;
	/** Curve mnemonic (e.g., "GR", "RHOB", "PHIE") */
	mnemonic: string;
	/** Unit of measurement */
	unit: string;
	/** Array of contiguous valid data segments */
	segments: CurveSegment[];
	/** Total depth range across all segments */
	depthRange: { min: number; max: number };
	/** Total valid point count (sum of all segment lengths) */
	totalPoints: number;
	/** Source information */
	source: DataFrameSource;
}

/**
 * CurveMetadata - Extracted at ingestion, stored separately from bulk data
 * Enables fast queries without loading full curve data (OSDU WellLog schema inspired)
 */
export interface CurveMetadata {
	mnemonic: string;
	unit: string;
	description?: string;

	// Depth statistics (computed at ingestion)
	topMeasuredDepth: number;
	bottomMeasuredDepth: number;
	depthStep?: number;

	// Data quality statistics
	totalSamples: number;
	validSamples: number;
	nullCount: number;
	nullPercentage: number;

	// Segment summary (computed at ingestion for fast access)
	segmentCount: number;
	largestGapDepth?: number;

	// Value statistics
	minValue?: number;
	maxValue?: number;
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Convert SegmentedCurveData (from Rust backend) to ECharts-ready series data
 *
 * Each segment becomes a separate array of [value, depth] points (for well log)
 * or [depth, value] points (for standard charts).
 *
 * This is the core function for the segment-based architecture:
 * - Segments come pre-extracted from the Rust backend (nulls already filtered)
 * - Each segment becomes a separate ECharts series
 * - Gaps between segments appear automatically as visual breaks
 */
export function segmentedCurveToSeriesData(
	segmentedData: {
		curve_id: string;
		mnemonic: string;
		unit: string | null;
		segments: Array<{
			depth_start: number;
			depth_end: number;
			depths: number[];
			values: number[];
		}>;
		depth_range: [number, number];
		total_points: number;
	},
	options: {
		swapForWellLog?: boolean;
		targetPointsPerSegment?: number;
	} = {}
): Array<{
	name: string;
	data: Array<[number, number]>;
	depthStart: number;
	depthEnd: number;
}> {
	const { swapForWellLog = true, targetPointsPerSegment } = options;
	const result: Array<{
		name: string;
		data: Array<[number, number]>;
		depthStart: number;
		depthEnd: number;
	}> = [];

	for (let segIdx = 0; segIdx < segmentedData.segments.length; segIdx++) {
		const segment = segmentedData.segments[segIdx];
		const depths = segment.depths;
		const values = segment.values;

		// Build data array for this segment
		const data: Array<[number, number]> = [];

		// Optional: downsample large segments
		const step =
			targetPointsPerSegment && depths.length > targetPointsPerSegment
				? Math.ceil(depths.length / targetPointsPerSegment)
				: 1;

		for (let i = 0; i < depths.length; i += step) {
			if (swapForWellLog) {
				// Well log format: [value, depth] - value on X, depth on Y
				data.push([values[i], depths[i]]);
			} else {
				// Standard format: [depth, value] - depth on X, value on Y
				data.push([depths[i], values[i]]);
			}
		}

		// Ensure we include the last point if we're stepping
		if (step > 1 && depths.length > 0) {
			const lastIdx = depths.length - 1;
			const lastPoint = swapForWellLog
				? [values[lastIdx], depths[lastIdx]] as [number, number]
				: [depths[lastIdx], values[lastIdx]] as [number, number];
			if (
				data.length === 0 ||
				data[data.length - 1][0] !== lastPoint[0] ||
				data[data.length - 1][1] !== lastPoint[1]
			) {
				data.push(lastPoint);
			}
		}

		result.push({
			name:
				segmentedData.segments.length > 1
					? `${segmentedData.mnemonic} (${segIdx + 1})`
					: segmentedData.mnemonic,
			data,
			depthStart: segment.depth_start,
			depthEnd: segment.depth_end
		});
	}

	return result;
}

/**
 * Convert SegmentedCurveData to ChartDataFrame for backward compatibility
 *
 * This flattens segments back into a single array with NaN gaps.
 * Use sparingly - prefer segmentedCurveToSeriesData for new code.
 */
export function segmentedCurveToFrame(
	segmentedData: {
		curve_id: string;
		mnemonic: string;
		unit: string | null;
		segments: Array<{
			depth_start: number;
			depth_end: number;
			depths: number[];
			values: number[];
		}>;
		depth_range: [number, number];
		total_points: number;
	},
	source: DataFrameSource
): ChartDataFrame {
	// Flatten all segments into single arrays with NaN gaps
	const allDepths: number[] = [];
	const allValues: number[] = [];

	for (let segIdx = 0; segIdx < segmentedData.segments.length; segIdx++) {
		const segment = segmentedData.segments[segIdx];

		// Add gap marker between segments (except before first)
		if (segIdx > 0 && allDepths.length > 0) {
			const lastDepth = allDepths[allDepths.length - 1];
			const gapDepth = (lastDepth + segment.depth_start) / 2;
			allDepths.push(gapDepth);
			allValues.push(NaN);
		}

		// Add segment data
		for (let i = 0; i < segment.depths.length; i++) {
			allDepths.push(segment.depths[i]);
			allValues.push(segment.values[i]);
		}
	}

	return {
		id: segmentedData.curve_id,
		name: segmentedData.mnemonic,
		source,
		fields: [
			{
				name: 'DEPTH',
				type: 'number',
				values: new Float64Array(allDepths),
				unit: 'm',
				config: { displayName: 'Depth' }
			},
			{
				name: segmentedData.mnemonic,
				type: 'number',
				values: new Float64Array(allValues),
				unit: segmentedData.unit ?? undefined,
				config: { displayName: segmentedData.mnemonic }
			}
		],
		length: allDepths.length,
		meta: {
			depthInverted: true,
			depthRange: {
				min: segmentedData.depth_range[0],
				max: segmentedData.depth_range[1]
			}
		}
	};
}

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

	let nullCount = 0;
	let nanCount = 0;
	let validCount = 0;
	let firstNullIndex = -1;
	let lastNullIndex = -1;

	for (let i = 0; i < data.length; i++) {
		depths[i] = data[i].depth;
		const v = data[i].value;
		if (v === null) {
			nullCount++;
			if (firstNullIndex === -1) firstNullIndex = i;
			lastNullIndex = i;
			values[i] = NaN;
		} else if (Number.isNaN(v)) {
			nanCount++;
			values[i] = NaN;
		} else {
			validCount++;
			values[i] = v;
		}
	}

	console.log('[curveDataToFrame] Data analysis for', mnemonic, ':', {
		totalPoints: data.length,
		nullCount,
		nanCount,
		validCount,
		firstNullIndex,
		lastNullIndex,
		firstFewData: data.slice(0, 5).map(d => ({ depth: d.depth, value: d.value })),
		lastFewData: data.slice(-5).map(d => ({ depth: d.depth, value: d.value })),
		depthRange: { min: depths[0], max: depths[depths.length - 1] }
	});

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
