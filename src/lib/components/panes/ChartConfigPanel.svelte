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
	}

	let { pane, config, wells, curves, well, onWellChange, onConfigChange }: Props = $props();

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
	 * Update axis binding
	 */
	function updateAxis(axisKey: 'xAxis' | 'yAxis' | 'zAxis' | 'dataCurve', binding: AxisBinding): void {
		onConfigChange({ ...chartConfig, [axisKey]: binding } as ChartConfiguration);
	}

	/**
	 * Update style property
	 */
	function updateStyle<K extends keyof SeriesStyle>(key: K, value: SeriesStyle[K]): void {
		const currentConfig = chartConfig as LineChartConfig | ScatterChartConfig | HistogramConfig;
		onConfigChange({
			...currentConfig,
			style: { ...currentConfig.style, [key]: value },
		} as ChartConfiguration);
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

					<CurveSelector
						label="Z Axis (Color)"
						binding={crossConfig.zAxis ?? { curveId: null, autoScale: true }}
						{curves}
						{well}
						onChange={(binding) => updateAxis('zAxis', binding)}
					/>
				</div>

				<div class="config-section">
					<h4 class="section-title">Style</h4>

					<div class="field-group">
						<label class="field-label">Color Map</label>
						<select
							class="field-select"
							value={crossConfig.colorMap}
							onchange={(e) =>
								onConfigChange({
									...crossConfig,
									colorMap: e.currentTarget.value as 'viridis' | 'plasma' | 'rainbow' | 'grayscale',
								})}
						>
							<option value="viridis">Viridis</option>
							<option value="plasma">Plasma</option>
							<option value="rainbow">Rainbow</option>
							<option value="grayscale">Grayscale</option>
						</select>
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
				</div>

				<div class="config-section">
					<h4 class="section-title">Tracks</h4>

					<div class="tracks-info">
						<p>{wellLogConfig.tracks.length} track(s) configured</p>
						<button type="button" class="add-track-button">
							<svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
								<path d="M7 2v10M2 7h10" stroke="currentColor" stroke-width="2" fill="none" />
							</svg>
							Add Track
						</button>
					</div>

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

	.tracks-info {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 12px;
	}

	.tracks-info p {
		font-size: 13px;
		color: var(--color-text-secondary, #6b7280);
		margin: 0;
	}

	.add-track-button {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		border: 1px solid var(--color-border, #e5e7eb);
		border-radius: 6px;
		background: var(--color-bg, #ffffff);
		font-size: 12px;
		cursor: pointer;
		transition: background-color 0.15s ease;
	}

	.add-track-button:hover {
		background: var(--color-bg-hover, #f3f4f6);
	}
</style>
