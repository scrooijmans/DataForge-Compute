<script lang="ts">
	/**
	 * ChartConfigPanel - Configuration panel for chart settings
	 *
	 * Displays different configuration options based on the chart type:
	 * - Line/Scatter: X/Y axis curve selection, styling
	 * - Histogram: Data curve selection, bin count
	 * - Cross Plot: X/Y/Z axis selection, color map
	 * - Well Log: Track configuration, depth range
	 */
	import type { CurveInfo, WellInfo } from '$lib/types';
	import type { PaneNode } from '$lib/panes/layout-model';
	import { PaneType } from '$lib/panes/layout-model';
	import type {
		ChartConfiguration,
		LineChartConfig,
		ScatterChartConfig,
		HistogramConfig,
		CrossPlotConfig,
		WellLogConfig,
		AxisBinding,
		SeriesStyle,
	} from '$lib/panes/chart-configs';
	import {
		createDefaultLineChartConfig,
		createDefaultScatterChartConfig,
		createDefaultHistogramConfig,
		createDefaultCrossPlotConfig,
		createDefaultWellLogConfig,
		COLOR_PRESETS,
		getChartTypeName,
	} from '$lib/panes/chart-configs';
	import CurveSelector from './CurveSelector.svelte';
	import { loadCurveData, loadSegmentedCurveData } from '$lib/stores/dataStore';
	import { curveDataToFrame, type ChartDataFrame } from '$lib/charts/types';
	import type { SegmentedCurveData } from '$lib/types';

	interface Props {
		/** Selected pane node */
		pane: PaneNode;
		/** Current chart configuration */
		config: ChartConfiguration | null;
		/** Available wells */
		wells: WellInfo[];
		/** Available curves for the selected well */
		curves: CurveInfo[];
		/** Selected well info */
		well: WellInfo | null;
		/** Callback when well selection changes */
		onWellChange?: (wellId: string) => void;
		/** Callback when configuration changes */
		onConfigChange: (config: ChartConfiguration) => void;
		/** Callback when chart data changes (legacy ChartDataFrame format) */
		onDataChange?: (data: ChartDataFrame | null) => void;
		/** Callback when segmented chart data changes (new segment-based format) */
		onSegmentedDataChange?: (data: SegmentedCurveData | null) => void;
	}

	let { pane, config, wells, curves, well, onWellChange, onConfigChange, onDataChange, onSegmentedDataChange }: Props = $props();

	/** Loading state for curve data */
	let isLoadingData = $state(false);

	/** Initialize config if not present */
	let chartConfig = $derived.by(() => {
		if (config) return config;
		// Create default config based on pane type
		switch (pane.paneType) {
			case PaneType.LineChart:
				return createDefaultLineChartConfig();
			case PaneType.ScatterChart:
				return createDefaultScatterChartConfig();
			case PaneType.Histogram:
				return createDefaultHistogramConfig();
			case PaneType.CrossPlot:
				return createDefaultCrossPlotConfig();
			case PaneType.WellLog:
			case PaneType.LinkedCharts:
				return createDefaultWellLogConfig();
			default:
				return createDefaultLineChartConfig();
		}
	});

	/**
	 * Update a specific field in the config
	 */
	function updateConfig<K extends keyof ChartConfiguration>(
		key: K,
		value: ChartConfiguration[K]
	): void {
		onConfigChange({ ...chartConfig, [key]: value } as ChartConfiguration);
	}

	/**
	 * Update axis binding and load curve data if applicable
	 */
	async function updateAxis(axisKey: 'xAxis' | 'yAxis' | 'zAxis' | 'dataCurve', binding: AxisBinding): Promise<void> {
		console.log('[ChartConfigPanel] updateAxis called:', { axisKey, binding });
		const newConfig = { ...chartConfig, [axisKey]: binding } as ChartConfiguration;
		onConfigChange(newConfig);

		// Trigger data loading when axis binding changes
		await loadChartData(newConfig);
	}

	/**
	 * Load chart data based on axis bindings
	 * For scatter charts with two curves: find overlapping depths and pair values
	 * For line charts: plot Y values against depth
	 * For crossplots: find overlapping depths for X, Y, and optionally Z axes
	 */
	async function loadChartData(currentConfig: ChartConfiguration): Promise<void> {
		console.log('[ChartConfigPanel] loadChartData called, config type:', currentConfig.type);

		// Handle crossplot separately
		if (currentConfig.type === 'crossplot') {
			await loadCrossPlotData(currentConfig as CrossPlotConfig);
			return;
		}

		// Only handle line/scatter charts for now
		if (currentConfig.type !== 'line' && currentConfig.type !== 'scatter') {
			console.log('[ChartConfigPanel] Skipping - not a line/scatter chart');
			return;
		}

		const lineConfig = currentConfig as LineChartConfig | ScatterChartConfig;
		const xAxisCurveId = lineConfig.xAxis?.curveId;
		const yAxisCurveId = lineConfig.yAxis?.curveId;
		console.log('[ChartConfigPanel] xAxisCurveId:', xAxisCurveId, 'yAxisCurveId:', yAxisCurveId);

		// Need Y axis curve to plot data
		if (!yAxisCurveId) {
			console.log('[ChartConfigPanel] No Y axis curve selected, clearing data');
			onDataChange?.(null);
			return;
		}

		isLoadingData = true;

		try {
			// Load Y-axis curve data
			console.log('[ChartConfigPanel] Loading Y-axis curve data for:', yAxisCurveId);
			const yData = await loadCurveData(yAxisCurveId);
			console.log('[ChartConfigPanel] Y curve data loaded:', yData ? `${yData.data.length} points` : 'null');

			if (!yData || yData.data.length === 0) {
				console.log('[ChartConfigPanel] No Y data, clearing');
				onDataChange?.(null);
				return;
			}

			// If X axis has a curve selected, create a curve-vs-curve plot
			if (xAxisCurveId) {
				console.log('[ChartConfigPanel] Loading X-axis curve data for:', xAxisCurveId);
				const xData = await loadCurveData(xAxisCurveId);
				console.log('[ChartConfigPanel] X curve data loaded:', xData ? `${xData.data.length} points` : 'null');

				if (xData && xData.data.length > 0) {
					// Create lookup map for X data by depth (using tolerance for floating point comparison)
					const xByDepth = new Map<number, number | null>();
					for (const point of xData.data) {
						// Round depth to 4 decimal places for consistent matching
						const roundedDepth = Math.round(point.depth * 10000) / 10000;
						xByDepth.set(roundedDepth, point.value);
					}

					// Find overlapping depths and pair the values
					const pairedData: Array<{ depth: number; xValue: number; yValue: number }> = [];
					for (const yPoint of yData.data) {
						const roundedDepth = Math.round(yPoint.depth * 10000) / 10000;
						const xValue = xByDepth.get(roundedDepth);

						// Only include points where both X and Y have valid values at the same depth
						if (xValue !== undefined && xValue !== null && yPoint.value !== null) {
							pairedData.push({
								depth: yPoint.depth,
								xValue: xValue,
								yValue: yPoint.value
							});
						}
					}

					console.log('[ChartConfigPanel] Paired data points:', pairedData.length, 'from', yData.data.length, 'Y points and', xData.data.length, 'X points');

					if (pairedData.length > 0) {
						// Create frame for curve-vs-curve plot
						const frame = createCurveVsCurveFrame(
							pairedData,
							xData.mnemonic,
							yData.mnemonic,
							xData.unit,
							yData.unit,
							well?.id
						);
						console.log('[ChartConfigPanel] Created curve-vs-curve frame:', { id: frame.id, length: frame.length });
						onDataChange?.(frame);
					} else {
						console.log('[ChartConfigPanel] No overlapping depth points found');
						onDataChange?.(null);
					}
					return;
				}
			}

			// No X curve or X curve failed - fall back to Y vs Depth plot
			const frame = curveDataToFrame(yData.data, yData.mnemonic, {
				type: 'well_curve',
				wellId: well?.id,
				curveId: yAxisCurveId
			});
			console.log('[ChartConfigPanel] Created Y vs Depth frame:', { id: frame.id, length: frame.length });

			// Add unit info if available
			if (yData.unit && frame.fields[1]) {
				frame.fields[1].unit = yData.unit;
			}

			onDataChange?.(frame);
		} catch (error) {
			console.error('[ChartConfigPanel] Failed to load curve data:', error);
			onDataChange?.(null);
		} finally {
			isLoadingData = false;
		}
	}

	/**
	 * Load crossplot data with optional Z-axis for color coding
	 * Finds overlapping depth indices across all selected curves
	 */
	async function loadCrossPlotData(crossConfig: CrossPlotConfig): Promise<void> {
		const xAxisCurveId = crossConfig.xAxis?.curveId;
		const yAxisCurveId = crossConfig.yAxis?.curveId;
		const zAxisCurveId = crossConfig.colorMode === 'curve' ? crossConfig.zAxis?.curveId : null;

		console.log('[ChartConfigPanel] loadCrossPlotData:', { xAxisCurveId, yAxisCurveId, zAxisCurveId, colorMode: crossConfig.colorMode });

		// Need both X and Y axes to plot crossplot
		if (!xAxisCurveId || !yAxisCurveId) {
			console.log('[ChartConfigPanel] Missing X or Y axis curve for crossplot');
			onDataChange?.(null);
			return;
		}

		isLoadingData = true;

		try {
			// Load X and Y curve data
			const [xData, yData] = await Promise.all([
				loadCurveData(xAxisCurveId),
				loadCurveData(yAxisCurveId)
			]);

			if (!xData || !yData || xData.data.length === 0 || yData.data.length === 0) {
				console.log('[ChartConfigPanel] Missing X or Y data for crossplot');
				onDataChange?.(null);
				return;
			}

			// Optionally load Z-axis data for color coding
			let zData = null;
			if (zAxisCurveId) {
				zData = await loadCurveData(zAxisCurveId);
				console.log('[ChartConfigPanel] Z curve data loaded:', zData ? `${zData.data.length} points` : 'null');
			}

			// Create lookup maps by depth
			const xByDepth = new Map<number, number | null>();
			for (const point of xData.data) {
				const roundedDepth = Math.round(point.depth * 10000) / 10000;
				xByDepth.set(roundedDepth, point.value);
			}

			const zByDepth = new Map<number, number | null>();
			if (zData) {
				for (const point of zData.data) {
					const roundedDepth = Math.round(point.depth * 10000) / 10000;
					zByDepth.set(roundedDepth, point.value);
				}
			}

			// Find overlapping depths and gather all values
			const crossPlotData: Array<{ depth: number; xValue: number; yValue: number; zValue?: number }> = [];

			for (const yPoint of yData.data) {
				const roundedDepth = Math.round(yPoint.depth * 10000) / 10000;
				const xValue = xByDepth.get(roundedDepth);

				// Need valid X and Y values at this depth
				if (xValue === undefined || xValue === null || yPoint.value === null) {
					continue;
				}

				// If Z-axis is selected, also need valid Z value
				if (zAxisCurveId) {
					const zValue = zByDepth.get(roundedDepth);
					if (zValue === undefined || zValue === null) {
						continue;
					}
					crossPlotData.push({
						depth: yPoint.depth,
						xValue,
						yValue: yPoint.value,
						zValue
					});
				} else {
					crossPlotData.push({
						depth: yPoint.depth,
						xValue,
						yValue: yPoint.value
					});
				}
			}

			console.log('[ChartConfigPanel] CrossPlot data points:', crossPlotData.length);

			if (crossPlotData.length > 0) {
				console.log('[ChartConfigPanel] Creating frame with colorMode:', crossConfig.colorMode, 'colorMap:', crossConfig.colorMap);
				const frame = createCrossPlotFrame(
					crossPlotData,
					xData.mnemonic,
					yData.mnemonic,
					zData?.mnemonic,
					xData.unit,
					yData.unit,
					zData?.unit,
					crossConfig.colorMode,
					crossConfig.colorMap,
					crossConfig.wellColor ?? getRandomWellColor(),
					well?.id
				);
				console.log('[ChartConfigPanel] Created crossplot frame:', { id: frame.id, length: frame.length, fields: frame.fields.map(f => f.name), meta: frame.meta });
				onDataChange?.(frame);
			} else {
				console.log('[ChartConfigPanel] No overlapping depth points found for crossplot');
				onDataChange?.(null);
			}
		} catch (error) {
			console.error('[ChartConfigPanel] Failed to load crossplot data:', error);
			onDataChange?.(null);
		} finally {
			isLoadingData = false;
		}
	}

	/**
	 * Generate a random color for well coloring
	 */
	function getRandomWellColor(): string {
		const colors = ['#3b82f6', '#22c55e', '#ef4444', '#f59e0b', '#8b5cf6', '#06b6d4', '#ec4899', '#84cc16'];
		return colors[Math.floor(Math.random() * colors.length)];
	}

	/**
	 * Create a ChartDataFrame for curve-vs-curve plotting (e.g., GR vs Resistivity)
	 */
	function createCurveVsCurveFrame(
		data: Array<{ depth: number; xValue: number; yValue: number }>,
		xMnemonic: string,
		yMnemonic: string,
		xUnit: string | null,
		yUnit: string | null,
		wellId?: string
	): import('$lib/charts/types').ChartDataFrame {
		const depths = new Float64Array(data.length);
		const xValues = new Float64Array(data.length);
		const yValues = new Float64Array(data.length);

		for (let i = 0; i < data.length; i++) {
			depths[i] = data[i].depth;
			xValues[i] = data[i].xValue;
			yValues[i] = data[i].yValue;
		}

		return {
			id: `crossplot:${xMnemonic}-${yMnemonic}`,
			name: `${xMnemonic} vs ${yMnemonic}`,
			source: {
				type: 'well_curve',
				wellId
			},
			fields: [
				{
					name: xMnemonic,
					type: 'number',
					values: xValues,
					unit: xUnit ?? undefined,
					config: { displayName: xMnemonic }
				},
				{
					name: yMnemonic,
					type: 'number',
					values: yValues,
					unit: yUnit ?? undefined,
					config: { displayName: yMnemonic }
				}
			],
			length: data.length,
			meta: {
				depthInverted: false, // Not a depth plot
				depthRange: {
					min: depths[0],
					max: depths[depths.length - 1]
				}
			}
		};
	}

	/**
	 * Create a ChartDataFrame for crossplot with optional Z-axis coloring
	 */
	function createCrossPlotFrame(
		data: Array<{ depth: number; xValue: number; yValue: number; zValue?: number }>,
		xMnemonic: string,
		yMnemonic: string,
		zMnemonic: string | undefined,
		xUnit: string | null,
		yUnit: string | null,
		zUnit: string | null | undefined,
		colorMode: 'curve' | 'well' | 'none',
		colorMap: 'viridis' | 'plasma' | 'rainbow' | 'grayscale',
		wellColor: string,
		wellId?: string
	): import('$lib/charts/types').ChartDataFrame {
		const depths = new Float64Array(data.length);
		const xValues = new Float64Array(data.length);
		const yValues = new Float64Array(data.length);

		for (let i = 0; i < data.length; i++) {
			depths[i] = data[i].depth;
			xValues[i] = data[i].xValue;
			yValues[i] = data[i].yValue;
		}

		const fields: import('$lib/charts/types').ChartField[] = [
			{
				name: xMnemonic,
				type: 'number',
				values: xValues,
				unit: xUnit ?? undefined,
				config: { displayName: xMnemonic }
			},
			{
				name: yMnemonic,
				type: 'number',
				values: yValues,
				unit: yUnit ?? undefined,
				config: { displayName: yMnemonic }
			}
		];

		// Add Z-axis field if coloring by curve
		if (colorMode === 'curve' && zMnemonic) {
			const zValues = new Float64Array(data.length);
			for (let i = 0; i < data.length; i++) {
				zValues[i] = data[i].zValue ?? NaN;
			}
			fields.push({
				name: zMnemonic,
				type: 'number',
				values: zValues,
				unit: zUnit ?? undefined,
				config: { displayName: zMnemonic }
			});
		}

		return {
			id: `crossplot:${xMnemonic}-${yMnemonic}${zMnemonic ? `-${zMnemonic}` : ''}`,
			name: `${xMnemonic} vs ${yMnemonic}${zMnemonic ? ` (${zMnemonic})` : ''}`,
			source: {
				type: 'well_curve',
				wellId
			},
			fields,
			length: data.length,
			meta: {
				depthInverted: false, // Not a depth plot - crossplot uses standard X-Y orientation
				crossplot: true, // Mark as crossplot for EChartsChart to detect
				colorMode, // How to color the points
				colorMap, // Color map for Z-axis coloring
				wellColor, // Color to use if colorMode is 'well'
				zMnemonic, // Name of the Z-axis curve for color dimension
				depthRange: {
					min: depths[0],
					max: depths[depths.length - 1]
				}
			}
		};
	}

	/**
	 * Update style property
	 */
	function updateStyle<K extends keyof SeriesStyle>(key: K, value: SeriesStyle[K]): void {
		const currentConfig = chartConfig as LineChartConfig | ScatterChartConfig | HistogramConfig | WellLogConfig;
		onConfigChange({
			...currentConfig,
			style: { ...currentConfig.style, [key]: value },
		} as ChartConfiguration);
	}

	/**
	 * Update well log curve binding and load data
	 */
	async function updateWellLogCurve(binding: AxisBinding): Promise<void> {
		console.log('[ChartConfigPanel] updateWellLogCurve called:', binding);
		const wellLogConfig = chartConfig as WellLogConfig;
		const newConfig = { ...wellLogConfig, curve: binding };
		onConfigChange(newConfig);

		// Load the well log data
		await loadWellLogData(newConfig);
	}

	/**
	 * Load well log data - curve plotted against depth
	 * Creates a ChartDataFrame with depth on the first field (will be Y-axis)
	 * and curve values on the second field (will be X-axis in well log display)
	 *
	 * NEW: Also loads segmented curve data (OSDU-inspired architecture)
	 * Segmented data eliminates null handling - each segment is contiguous valid data
	 */
	async function loadWellLogData(currentConfig: WellLogConfig): Promise<void> {
		const curveId = currentConfig.curve?.curveId;

		console.log('[ChartConfigPanel] loadWellLogData:', { curveId });

		if (!curveId) {
			console.log('[ChartConfigPanel] No curve selected for well log');
			onDataChange?.(null);
			onSegmentedDataChange?.(null);
			return;
		}

		isLoadingData = true;

		try {
			// Load both legacy and segmented data in parallel
			const [curveData, segmentedData] = await Promise.all([
				loadCurveData(curveId),
				loadSegmentedCurveData(curveId)
			]);

			// Handle segmented data (new architecture - preferred path)
			if (segmentedData && segmentedData.segments.length > 0) {
				console.log('[ChartConfigPanel] Loaded segmented data:', {
					curveId: segmentedData.curve_id,
					mnemonic: segmentedData.mnemonic,
					segmentCount: segmentedData.segments.length,
					totalPoints: segmentedData.total_points,
					depthRange: segmentedData.depth_range
				});
				onSegmentedDataChange?.(segmentedData);
			} else {
				onSegmentedDataChange?.(null);
			}

			// Handle legacy data (backward compatibility)
			if (!curveData || curveData.data.length === 0) {
				console.log('[ChartConfigPanel] No curve data for well log');
				onDataChange?.(null);
				return;
			}

			// Create frame with depth as first field (Y-axis in well log) and curve as second field (X-axis)
			const frame = curveDataToFrame(curveData.data, curveData.mnemonic, {
				type: 'well_curve',
				wellId: well?.id,
				curveId: curveId
			});

			// Add unit info if available
			if (curveData.unit && frame.fields[1]) {
				frame.fields[1].unit = curveData.unit;
			}

			// Set metadata for well log display
			frame.meta = {
				...frame.meta,
				depthInverted: currentConfig.depthInverted,
				preferredChartType: 'welllog',
			};

			console.log('[ChartConfigPanel] Created well log frame:', { id: frame.id, length: frame.length });
			onDataChange?.(frame);
		} catch (error) {
			console.error('[ChartConfigPanel] Failed to load well log data:', error);
			onDataChange?.(null);
			onSegmentedDataChange?.(null);
		} finally {
			isLoadingData = false;
		}
	}
</script>

<div class="chart-config-panel">
	<div class="panel-header">
		<h3 class="panel-title">{getChartTypeName(chartConfig.type)} Settings</h3>
		<span class="pane-title">{pane.title}</span>
	</div>

	<div class="panel-content">
		<!-- Well Selection - Always show this first -->
		<div class="config-section">
			<h4 class="section-title">Data Source</h4>

			<div class="field-group">
				<label class="field-label" for="well-selector">Well</label>
				<select
					id="well-selector"
					class="field-select"
					value={well?.id ?? ''}
					onchange={(e) => onWellChange?.(e.currentTarget.value)}
				>
					<option value="">Select a well...</option>
					{#each wells as w (w.id)}
						<option value={w.id}>{w.name} ({w.curve_count} curves)</option>
					{/each}
				</select>
			</div>

			{#if !well}
				<div class="well-hint">
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
					</svg>
					<span>Select a well to choose curves for plotting</span>
				</div>
			{/if}
		</div>

		{#if well}
			<!-- Common Options -->
			<div class="config-section">
				<h4 class="section-title">General</h4>

				<div class="field-group">
					<label class="field-label" for="chart-title">Title</label>
					<input
						id="chart-title"
						type="text"
						class="field-input"
						value={chartConfig.title}
						placeholder="Chart title..."
						oninput={(e) => updateConfig('title', e.currentTarget.value)}
					/>
				</div>

				<div class="field-row">
					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={chartConfig.showLegend}
							onchange={(e) => updateConfig('showLegend', e.currentTarget.checked)}
						/>
						<span>Show Legend</span>
					</label>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={chartConfig.showGrid}
							onchange={(e) => updateConfig('showGrid', e.currentTarget.checked)}
						/>
						<span>Show Grid</span>
					</label>
				</div>

				<div class="field-row">
					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={chartConfig.enableZoom}
							onchange={(e) => updateConfig('enableZoom', e.currentTarget.checked)}
						/>
						<span>Enable Zoom</span>
					</label>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={chartConfig.showCursor}
							onchange={(e) => updateConfig('showCursor', e.currentTarget.checked)}
						/>
						<span>Show Cursor</span>
					</label>
				</div>
			</div>

			<!-- Line Chart Options -->
			{#if chartConfig.type === 'line'}
				{@const lineConfig = chartConfig as LineChartConfig}
				<div class="config-section">
					<h4 class="section-title">Data Binding</h4>

					<CurveSelector
						label="X Axis"
						binding={lineConfig.xAxis}
						{curves}
						{well}
						required
						onChange={(binding) => updateAxis('xAxis', binding)}
					/>

					<CurveSelector
						label="Y Axis"
						binding={lineConfig.yAxis}
						{curves}
						{well}
						required
						onChange={(binding) => updateAxis('yAxis', binding)}
					/>
				</div>

				<div class="config-section">
					<h4 class="section-title">Style</h4>

					<div class="field-group">
						<label class="field-label">Interpolation</label>
						<select
							class="field-select"
							value={lineConfig.interpolation}
							onchange={(e) =>
								onConfigChange({ ...lineConfig, interpolation: e.currentTarget.value as 'linear' | 'spline' | 'step' })}
						>
							<option value="linear">Linear</option>
							<option value="spline">Spline</option>
							<option value="step">Step</option>
						</select>
					</div>

					<div class="field-group">
						<label class="field-label">Color</label>
						<div class="color-presets">
							{#each COLOR_PRESETS as color}
								<button
									type="button"
									class="color-preset"
									class:selected={lineConfig.style.color === color}
									style="background-color: {color}"
									onclick={() => updateStyle('color', color)}
									aria-label="Select color {color}"
								></button>
							{/each}
						</div>
					</div>

					<div class="field-row">
						<div class="field-group">
							<label class="field-label" for="line-width">Line Width</label>
							<input
								id="line-width"
								type="number"
								class="field-input small"
								min="1"
								max="10"
								value={lineConfig.style.lineWidth}
								onchange={(e) => updateStyle('lineWidth', parseInt(e.currentTarget.value))}
							/>
						</div>

						<label class="checkbox-label">
							<input
								type="checkbox"
								checked={lineConfig.style.showPoints}
								onchange={(e) => updateStyle('showPoints', e.currentTarget.checked)}
							/>
							<span>Show Points</span>
						</label>
					</div>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={lineConfig.style.fillArea}
							onchange={(e) => updateStyle('fillArea', e.currentTarget.checked)}
						/>
						<span>Fill Area Under Curve</span>
					</label>
				</div>
			{/if}

			<!-- Scatter Chart Options -->
			{#if chartConfig.type === 'scatter'}
				{@const scatterConfig = chartConfig as ScatterChartConfig}
				<div class="config-section">
					<h4 class="section-title">Data Binding</h4>

					<CurveSelector
						label="X Axis"
						binding={scatterConfig.xAxis}
						{curves}
						{well}
						required
						onChange={(binding) => updateAxis('xAxis', binding)}
					/>

					<CurveSelector
						label="Y Axis"
						binding={scatterConfig.yAxis}
						{curves}
						{well}
						required
						onChange={(binding) => updateAxis('yAxis', binding)}
					/>
				</div>

				<div class="config-section">
					<h4 class="section-title">Style</h4>

					<div class="field-group">
						<label class="field-label">Point Shape</label>
						<select
							class="field-select"
							value={scatterConfig.pointShape}
							onchange={(e) =>
								onConfigChange({
									...scatterConfig,
									pointShape: e.currentTarget.value as 'circle' | 'square' | 'triangle' | 'diamond',
								})}
						>
							<option value="circle">Circle</option>
							<option value="square">Square</option>
							<option value="triangle">Triangle</option>
							<option value="diamond">Diamond</option>
						</select>
					</div>

					<div class="field-group">
						<label class="field-label">Color</label>
						<div class="color-presets">
							{#each COLOR_PRESETS as color}
								<button
									type="button"
									class="color-preset"
									class:selected={scatterConfig.style.color === color}
									style="background-color: {color}"
									onclick={() => updateStyle('color', color)}
									aria-label="Select color {color}"
								></button>
							{/each}
						</div>
					</div>

					<div class="field-group">
						<label class="field-label" for="point-size">Point Size</label>
						<input
							id="point-size"
							type="number"
							class="field-input small"
							min="2"
							max="20"
							value={scatterConfig.style.pointSize}
							onchange={(e) => updateStyle('pointSize', parseInt(e.currentTarget.value))}
						/>
					</div>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={scatterConfig.showTrendLine}
							onchange={(e) =>
								onConfigChange({ ...scatterConfig, showTrendLine: e.currentTarget.checked })}
						/>
						<span>Show Trend Line</span>
					</label>

					{#if scatterConfig.showTrendLine}
						<div class="field-group">
							<label class="field-label">Trend Line Type</label>
							<select
								class="field-select"
								value={scatterConfig.trendLineType}
								onchange={(e) =>
									onConfigChange({
										...scatterConfig,
										trendLineType: e.currentTarget.value as 'linear' | 'polynomial' | 'exponential',
									})}
							>
								<option value="linear">Linear</option>
								<option value="polynomial">Polynomial</option>
								<option value="exponential">Exponential</option>
							</select>
						</div>
					{/if}
				</div>
			{/if}

			<!-- Histogram Options -->
			{#if chartConfig.type === 'histogram'}
				{@const histConfig = chartConfig as HistogramConfig}
				<div class="config-section">
					<h4 class="section-title">Data Binding</h4>

					<CurveSelector
						label="Data Curve"
						binding={histConfig.dataCurve}
						{curves}
						{well}
						required
						onChange={(binding) => updateAxis('dataCurve', binding)}
					/>
				</div>

				<div class="config-section">
					<h4 class="section-title">Histogram Options</h4>

					<div class="field-group">
						<label class="field-label" for="bin-count">Number of Bins</label>
						<input
							id="bin-count"
							type="number"
							class="field-input"
							min="5"
							max="200"
							value={histConfig.binCount}
							onchange={(e) =>
								onConfigChange({ ...histConfig, binCount: parseInt(e.currentTarget.value) })}
						/>
					</div>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={histConfig.showPercentage}
							onchange={(e) =>
								onConfigChange({ ...histConfig, showPercentage: e.currentTarget.checked })}
						/>
						<span>Show as Percentage</span>
					</label>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={histConfig.showNormalCurve}
							onchange={(e) =>
								onConfigChange({ ...histConfig, showNormalCurve: e.currentTarget.checked })}
						/>
						<span>Show Normal Distribution</span>
					</label>
				</div>

				<div class="config-section">
					<h4 class="section-title">Style</h4>

					<div class="field-group">
						<label class="field-label">Color</label>
						<div class="color-presets">
							{#each COLOR_PRESETS as color}
								<button
									type="button"
									class="color-preset"
									class:selected={histConfig.style.color === color}
									style="background-color: {color}"
									onclick={() => updateStyle('color', color)}
									aria-label="Select color {color}"
								></button>
							{/each}
						</div>
					</div>
				</div>
			{/if}

			<!-- Cross Plot Options -->
			{#if chartConfig.type === 'crossplot'}
				{@const crossConfig = chartConfig as CrossPlotConfig}
				<div class="config-section">
					<h4 class="section-title">Data Binding</h4>

					<CurveSelector
						label="X Axis"
						binding={crossConfig.xAxis}
						{curves}
						{well}
						required
						onChange={(binding) => updateAxis('xAxis', binding)}
					/>

					<CurveSelector
						label="Y Axis"
						binding={crossConfig.yAxis}
						{curves}
						{well}
						required
						onChange={(binding) => updateAxis('yAxis', binding)}
					/>
				</div>

				<div class="config-section">
					<h4 class="section-title">Color Coding</h4>

					<div class="field-group">
						<label class="field-label">Color By</label>
						<select
							class="field-select"
							value={crossConfig.colorMode}
							onchange={(e) => {
								const newMode = e.currentTarget.value as 'curve' | 'well' | 'none';
								const newConfig = { ...crossConfig, colorMode: newMode };
								// Auto-assign well color if switching to 'well' mode
								if (newMode === 'well' && !crossConfig.wellColor) {
									newConfig.wellColor = getRandomWellColor();
								}
								onConfigChange(newConfig);
								// Trigger data reload to update frame metadata
								loadChartData(newConfig);
							}}
						>
							<option value="none">Uniform Color</option>
							<option value="curve">Z-Axis Curve</option>
							<option value="well">Well</option>
						</select>
					</div>

					{#if crossConfig.colorMode === 'curve'}
						<CurveSelector
							label="Z Axis (Color)"
							binding={crossConfig.zAxis ?? { curveId: null, autoScale: true }}
							{curves}
							{well}
							required
							onChange={(binding) => updateAxis('zAxis', binding)}
						/>

						<div class="field-group">
							<label class="field-label">Color Map</label>
							<select
								class="field-select"
								value={crossConfig.colorMap}
								onchange={(e) => {
									const newConfig = {
										...crossConfig,
										colorMap: e.currentTarget.value as 'viridis' | 'plasma' | 'rainbow' | 'grayscale',
									};
									onConfigChange(newConfig);
									// Reload data to update frame metadata with new color map
									loadChartData(newConfig);
								}}
							>
								<option value="viridis">Viridis</option>
								<option value="plasma">Plasma</option>
								<option value="rainbow">Rainbow</option>
								<option value="grayscale">Grayscale</option>
							</select>
						</div>
					{/if}

					{#if crossConfig.colorMode === 'well'}
						<div class="field-group">
							<label class="field-label">Well Color</label>
							<div class="color-presets">
								{#each COLOR_PRESETS as color}
									<button
										type="button"
										class="color-preset"
										class:selected={crossConfig.wellColor === color}
										style="background-color: {color}"
										onclick={() => {
											const newConfig = { ...crossConfig, wellColor: color };
											onConfigChange(newConfig);
											loadChartData(newConfig);
										}}
										aria-label="Select color {color}"
									></button>
								{/each}
							</div>
						</div>
					{/if}

					{#if crossConfig.colorMode === 'none'}
						<div class="field-group">
							<label class="field-label">Point Color</label>
							<div class="color-presets">
								{#each COLOR_PRESETS as color}
									<button
										type="button"
										class="color-preset"
										class:selected={crossConfig.style.color === color}
										style="background-color: {color}"
										onclick={() => updateStyle('color', color)}
										aria-label="Select color {color}"
									></button>
								{/each}
							</div>
						</div>
					{/if}
				</div>

				<div class="config-section">
					<h4 class="section-title">Style</h4>

					<div class="field-group">
						<label class="field-label" for="crossplot-point-size">Point Size</label>
						<input
							id="crossplot-point-size"
							type="number"
							class="field-input small"
							min="2"
							max="20"
							value={crossConfig.style.pointSize}
							onchange={(e) => updateStyle('pointSize', parseInt(e.currentTarget.value))}
						/>
					</div>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={crossConfig.showRegression}
							onchange={(e) =>
								onConfigChange({ ...crossConfig, showRegression: e.currentTarget.checked })}
						/>
						<span>Show Regression Line</span>
					</label>
				</div>
			{/if}

			<!-- Well Log Options -->
			{#if chartConfig.type === 'welllog'}
				{@const wellLogConfig = chartConfig as WellLogConfig}
				<div class="config-section">
					<h4 class="section-title">Data Binding</h4>

					<CurveSelector
						label="Curve"
						binding={wellLogConfig.curve}
						{curves}
						{well}
						required
						onChange={(binding) => updateWellLogCurve(binding)}
					/>
				</div>

				<div class="config-section">
					<h4 class="section-title">Depth Settings</h4>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={wellLogConfig.depthRange.autoScale}
							onchange={(e) =>
								onConfigChange({
									...wellLogConfig,
									depthRange: { ...wellLogConfig.depthRange, autoScale: e.currentTarget.checked },
								})}
						/>
						<span>Auto-scale Depth</span>
					</label>

					{#if !wellLogConfig.depthRange.autoScale}
						<div class="field-row">
							<div class="field-group">
								<label class="field-label" for="depth-min">Min Depth</label>
								<input
									id="depth-min"
									type="number"
									class="field-input"
									value={wellLogConfig.depthRange.min ?? ''}
									onchange={(e) =>
										onConfigChange({
											...wellLogConfig,
											depthRange: {
												...wellLogConfig.depthRange,
												min: e.currentTarget.value ? parseFloat(e.currentTarget.value) : null,
											},
										})}
								/>
							</div>

							<div class="field-group">
								<label class="field-label" for="depth-max">Max Depth</label>
								<input
									id="depth-max"
									type="number"
									class="field-input"
									value={wellLogConfig.depthRange.max ?? ''}
									onchange={(e) =>
										onConfigChange({
											...wellLogConfig,
											depthRange: {
												...wellLogConfig.depthRange,
												max: e.currentTarget.value ? parseFloat(e.currentTarget.value) : null,
											},
										})}
								/>
							</div>
						</div>
					{/if}

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={wellLogConfig.depthInverted}
							onchange={(e) =>
								onConfigChange({ ...wellLogConfig, depthInverted: e.currentTarget.checked })}
						/>
						<span>Invert Depth (Increasing Downward)</span>
					</label>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={wellLogConfig.showDepthTrack}
							onchange={(e) =>
								onConfigChange({ ...wellLogConfig, showDepthTrack: e.currentTarget.checked })}
						/>
						<span>Show Depth Track</span>
					</label>
				</div>

				<div class="config-section">
					<h4 class="section-title">Style</h4>

					<div class="field-group">
						<label class="field-label">Color</label>
						<div class="color-presets">
							{#each COLOR_PRESETS as color}
								<button
									type="button"
									class="color-preset"
									class:selected={wellLogConfig.style.color === color}
									style="background-color: {color}"
									onclick={() => updateStyle('color', color)}
									aria-label="Select color {color}"
								></button>
							{/each}
						</div>
					</div>

					<div class="field-group">
						<label class="field-label" for="welllog-line-width">Line Width</label>
						<input
							id="welllog-line-width"
							type="number"
							class="field-input small"
							min="1"
							max="10"
							value={wellLogConfig.style.lineWidth}
							onchange={(e) => updateStyle('lineWidth', parseInt(e.currentTarget.value))}
						/>
					</div>

					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={wellLogConfig.style.fillArea}
							onchange={(e) => updateStyle('fillArea', e.currentTarget.checked)}
						/>
						<span>Fill Area</span>
					</label>
				</div>
			{/if}
		{/if}
	</div>
</div>

<style>
	.chart-config-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}

	.panel-header {
		padding: 12px 16px;
		border-bottom: 1px solid var(--color-border, #e5e7eb);
		background: var(--color-bg-secondary, #f9fafb);
	}

	.panel-title {
		font-size: 14px;
		font-weight: 600;
		margin: 0;
		color: var(--color-text, #111827);
	}

	.pane-title {
		font-size: 12px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.panel-content {
		flex: 1;
		overflow-y: auto;
		padding: 16px;
	}

	.well-hint {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		background: var(--color-info-light, #eff6ff);
		border-radius: 6px;
		font-size: 12px;
		color: var(--color-info, #3b82f6);
	}

	.well-hint svg {
		flex-shrink: 0;
		opacity: 0.7;
	}

	.config-section {
		margin-bottom: 20px;
	}

	.section-title {
		font-size: 12px;
		font-weight: 600;
		color: var(--color-text-secondary, #6b7280);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0 0 12px 0;
		padding-bottom: 8px;
		border-bottom: 1px solid var(--color-border, #e5e7eb);
	}

	.field-group {
		margin-bottom: 12px;
	}

	.field-label {
		display: block;
		font-size: 12px;
		font-weight: 500;
		color: var(--color-text-secondary, #6b7280);
		margin-bottom: 4px;
	}

	.field-input,
	.field-select {
		width: 100%;
		padding: 8px 12px;
		border: 1px solid var(--color-border, #e5e7eb);
		border-radius: 6px;
		font-size: 13px;
		background: var(--color-bg, #ffffff);
	}

	.field-input.small {
		width: 80px;
	}

	.field-input:focus,
	.field-select:focus {
		outline: none;
		border-color: var(--color-primary, #3b82f6);
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
	}

	.field-row {
		display: flex;
		gap: 12px;
		align-items: center;
		margin-bottom: 12px;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		color: var(--color-text, #111827);
		cursor: pointer;
		margin-bottom: 8px;
	}

	.checkbox-label input[type='checkbox'] {
		width: 16px;
		height: 16px;
		cursor: pointer;
	}

	.color-presets {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.color-preset {
		width: 24px;
		height: 24px;
		border: 2px solid transparent;
		border-radius: 4px;
		cursor: pointer;
		transition: border-color 0.15s ease, transform 0.1s ease;
	}

	.color-preset:hover {
		transform: scale(1.1);
	}

	.color-preset.selected {
		border-color: var(--color-text, #111827);
	}
</style>
