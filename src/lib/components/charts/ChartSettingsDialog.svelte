<script lang="ts">
	/**
	 * ChartSettingsDialog - Modal dialog for chart configuration
	 *
	 * A modal dialog that contains tabbed chart settings.
	 * Opens when the user clicks the settings gear in ChartSettingsToolbar.
	 *
	 * Tab Categories:
	 * - Data: Well selector, curve bindings
	 * - Axes: Scale type, auto-scale, min/max ranges
	 * - Style: Colors, line width, point size
	 * - Display: Title, legend, grid, zoom
	 * - Advanced: Chart-type-specific options
	 */

	import { onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import type { CurveInfo, WellInfo, SegmentedCurveData, MultiWellCurveData } from '$lib/types';
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
		createDefaultCorrelationConfig,
		COLOR_PRESETS,
		getChartTypeName,
	} from '$lib/panes/chart-configs';
	import type { ChartDataFrame } from '$lib/charts/types';
	import type {
		CorrelationConfig,
		WellCorrelationEntry,
		CorrelationTrack,
		CorrelationCurveData,
		SelectedCurveType,
		CorrelationLayoutConfig,
		WellTop,
	} from '$lib/charts/correlation-types';
	import {
		createWellEntry,
		createTrack,
		createTrackWithDefaults,
		getDefaultCurveColor,
		getDefaultCurveRange,
		getNextWellColor,
		DEFAULT_LAYOUT,
	} from '$lib/charts/correlation-types';
	import type { CurveInfoWithWell } from '$lib/types';
	import { allWorkspaceCurves } from '$lib/stores/compute';
	import { loadCurveData, loadSegmentedCurveData } from '$lib/stores/dataStore';
	import { curveDataToFrame } from '$lib/charts/types';
	import CurveSelector from '$lib/components/panes/CurveSelector.svelte';

	type TabId = 'data' | 'axes' | 'style' | 'display' | 'advanced' | 'wellTops';

	interface Tab {
		id: TabId;
		label: string;
		icon: string;
	}

	interface Props {
		/** Whether the dialog is open */
		open: boolean;
		/** Callback to close the dialog */
		onClose: () => void;
		/** Selected pane node */
		pane: PaneNode | null;
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
		/** Callback when chart data changes */
		onDataChange?: (data: ChartDataFrame | null) => void;
		/** Callback when segmented chart data changes */
		onSegmentedDataChange?: (data: SegmentedCurveData | null) => void;
		/** Callback when correlation curve data changes */
		onCorrelationCurveDataChange?: (trackId: string, data: CorrelationCurveData | null) => void;
		/** Callback when multi-well data changes (for crossplot, scatter, line) */
		onMultiWellDataChange?: (data: MultiWellCurveData[]) => void;
	}

	let {
		open,
		onClose,
		pane,
		config,
		wells,
		curves,
		well,
		onWellChange,
		onConfigChange,
		onDataChange,
		onSegmentedDataChange,
		onCorrelationCurveDataChange,
		onMultiWellDataChange,
	}: Props = $props();

	let dialogRef: HTMLDialogElement | null = $state(null);
	let activeTab: TabId = $state('data');
	let isLoadingData = $state(false);

	// Workspace curves for correlation panels
	let workspaceCurves: CurveInfoWithWell[] = $state([]);
	const unsubscribeWorkspaceCurves = allWorkspaceCurves.subscribe((value) => {
		workspaceCurves = value;
	});
	onDestroy(() => unsubscribeWorkspaceCurves());

	// ============================================================================
	// Well Tops (Formation Markers) - For Correlation Charts
	// ============================================================================

	/** MarkerData interface - matches DataForge's marker type */
	interface MarkerData {
		id: string;
		name: string;
		measured_depth: number;
		tvd: number | null;
		marker_type: string | null;
		quality: string | null;
		comments: string | null;
	}

	/** State for available well tops from database */
	let availableWellTops: MarkerData[] = $state([]);
	let loadingWellTops = $state(false);
	let selectedWellForTops: string = $state('');

	/** Color palette for well tops */
	const WELL_TOP_COLORS = ['#ef4444', '#f59e0b', '#22c55e', '#3b82f6', '#8b5cf6', '#ec4899'];
	let topColorIndex = 0;

	/** Get next color for well top */
	function getNextTopColor(): string {
		const color = WELL_TOP_COLORS[topColorIndex % WELL_TOP_COLORS.length];
		topColorIndex++;
		return color;
	}

	/** Load well tops from DataForge via Tauri */
	async function loadWellTops(wellId: string): Promise<void> {
		if (!wellId) {
			availableWellTops = [];
			return;
		}

		selectedWellForTops = wellId;
		loadingWellTops = true;

		try {
			const markers = await invoke<MarkerData[]>('get_well_markers', { wellId });
			availableWellTops = markers;
		} catch (error) {
			console.error('Failed to load well tops:', error);
			availableWellTops = [];
		} finally {
			loadingWellTops = false;
		}
	}

	/** Check if a marker is already in the config */
	function isWellTopSelected(marker: MarkerData): boolean {
		if (chartConfig?.type !== 'correlation') return false;
		const corrConfig = chartConfig as CorrelationConfig;
		return corrConfig.wellTops.some(
			(top) => top.name === marker.name && top.depth === marker.measured_depth
		);
	}

	/** Toggle a well top on/off */
	function toggleWellTop(marker: MarkerData): void {
		if (chartConfig?.type !== 'correlation') return;
		const corrConfig = chartConfig as CorrelationConfig;

		const existing = corrConfig.wellTops.find(
			(top) => top.name === marker.name && top.depth === marker.measured_depth
		);

		let newTops: WellTop[];
		if (existing) {
			// Remove
			newTops = corrConfig.wellTops.filter((top) => top !== existing);
		} else {
			// Add
			const newTop: WellTop = {
				name: marker.name,
				depth: marker.measured_depth,
				color: getNextTopColor(),
				lineStyle: 'solid',
				lineWidth: 1,
				wellId: selectedWellForTops,
				showLabel: true,
				labelPosition: 'left',
			};
			newTops = [...corrConfig.wellTops, newTop];
		}

		updateCorrelationConfig('wellTops', newTops);
	}

	/** Get color for a well top */
	function getWellTopColor(marker: MarkerData): string {
		if (chartConfig?.type !== 'correlation') return '#ff0000';
		const corrConfig = chartConfig as CorrelationConfig;
		const existing = corrConfig.wellTops.find(
			(top) => top.name === marker.name && top.depth === marker.measured_depth
		);
		return existing?.color ?? '#ff0000';
	}

	/** Set color for a well top */
	function setWellTopColor(marker: MarkerData, color: string): void {
		if (chartConfig?.type !== 'correlation') return;
		const corrConfig = chartConfig as CorrelationConfig;
		const newTops = corrConfig.wellTops.map((top) => {
			if (top.name === marker.name && top.depth === marker.measured_depth) {
				return { ...top, color };
			}
			return top;
		});
		updateCorrelationConfig('wellTops', newTops);
	}

	/** Remove a well top */
	function removeWellTop(top: WellTop): void {
		if (chartConfig?.type !== 'correlation') return;
		const corrConfig = chartConfig as CorrelationConfig;
		const newTops = corrConfig.wellTops.filter((t) => t !== top);
		updateCorrelationConfig('wellTops', newTops);
	}

	// ============================================================================
	// Tab definitions based on chart type
	const baseTabs: Tab[] = [
		{ id: 'data', label: 'Data', icon: 'M3 3v18h18V3H3zm16 16H5V5h14v14zM7 7h4v4H7V7zm6 0h4v4h-4V7zm-6 6h4v4H7v-4zm6 0h4v4h-4v-4z' },
		{ id: 'axes', label: 'Axes', icon: 'M19 19H5V5h2v12h12v2zM7 17V7h10v2H9v8H7z' },
		{ id: 'style', label: 'Style', icon: 'M12 3c-4.97 0-9 4.03-9 9s4.03 9 9 9c.83 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1.01-.23-.26-.38-.61-.38-.99 0-.83.67-1.5 1.5-1.5H16c2.76 0 5-2.24 5-5 0-4.42-4.03-8-9-8zm-5.5 9c-.83 0-1.5-.67-1.5-1.5S5.67 9 6.5 9 8 9.67 8 10.5 7.33 12 6.5 12zm3-4C8.67 8 8 7.33 8 6.5S8.67 5 9.5 5s1.5.67 1.5 1.5S10.33 8 9.5 8zm5 0c-.83 0-1.5-.67-1.5-1.5S13.67 5 14.5 5s1.5.67 1.5 1.5S15.33 8 14.5 8zm3 4c-.83 0-1.5-.67-1.5-1.5S16.67 9 17.5 9s1.5.67 1.5 1.5-.67 1.5-1.5 1.5z' },
		{ id: 'display', label: 'Display', icon: 'M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zM12 17c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm0-8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z' },
		{ id: 'advanced', label: 'Advanced', icon: 'M19.14 12.94c.04-.31.06-.63.06-.94 0-.31-.02-.63-.06-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.04.31-.06.63-.06.94s.02.63.06.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z' },
	];

	// Available tabs depend on chart type
	let availableTabs = $derived.by(() => {
		if (!pane || !chartConfig) return baseTabs;

		// Correlation charts have different tab structure
		if (chartConfig.type === 'correlation') {
			return [
				{ id: 'data' as TabId, label: 'Data', icon: baseTabs[0].icon },
				{ id: 'wellTops' as TabId, label: 'Well Tops', icon: 'M3 12h18M6 8v8M12 8v8M18 8v8' },
				{ id: 'display' as TabId, label: 'Display', icon: baseTabs[3].icon },
				{ id: 'advanced' as TabId, label: 'Layout', icon: baseTabs[4].icon },
			];
		}

		return baseTabs;
	});

	// Sync dialog open/close state
	$effect(() => {
		if (!dialogRef) return;

		if (open && !dialogRef.open) {
			dialogRef.showModal();
			// Reset to first tab when opening
			activeTab = 'data';
		} else if (!open && dialogRef.open) {
			dialogRef.close();
		}
	});

	function handleDialogClick(event: MouseEvent) {
		// Close if clicking on backdrop (outside dialog content)
		if (event.target === dialogRef) {
			onClose();
		}
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}

	// --- Chart Configuration Logic (from ChartConfigPanel) ---

	/** Initialize config if not present */
	let chartConfig = $derived.by(() => {
		if (config) return config;
		if (!pane) return null;
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
			case PaneType.Correlation:
				return createDefaultCorrelationConfig();
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
		if (!chartConfig) return;
		onConfigChange({ ...chartConfig, [key]: value } as ChartConfiguration);
	}

	/**
	 * Update axis binding and load curve data if applicable
	 */
	async function updateAxis(axisKey: 'xAxis' | 'yAxis' | 'zAxis' | 'dataCurve', binding: AxisBinding): Promise<void> {
		if (!chartConfig) return;
		const newConfig = { ...chartConfig, [axisKey]: binding } as ChartConfiguration;
		onConfigChange(newConfig);
		await loadChartData(newConfig);
	}

	/**
	 * Load chart data based on axis bindings
	 */
	async function loadChartData(currentConfig: ChartConfiguration): Promise<void> {
		if (currentConfig.type === 'crossplot') {
			await loadCrossPlotData(currentConfig as CrossPlotConfig);
			return;
		}

		if (currentConfig.type !== 'line' && currentConfig.type !== 'scatter') {
			return;
		}

		const lineConfig = currentConfig as LineChartConfig | ScatterChartConfig;
		const xAxisCurveId = lineConfig.xAxis?.curveId;
		const yAxisCurveId = lineConfig.yAxis?.curveId;

		if (!yAxisCurveId) {
			onDataChange?.(null);
			return;
		}

		isLoadingData = true;

		try {
			const yData = await loadCurveData(yAxisCurveId);

			if (!yData || yData.data.length === 0) {
				onDataChange?.(null);
				return;
			}

			if (xAxisCurveId) {
				const xData = await loadCurveData(xAxisCurveId);

				if (xData && xData.data.length > 0) {
					const xByDepth = new Map<number, number | null>();
					for (const point of xData.data) {
						const roundedDepth = Math.round(point.depth * 10000) / 10000;
						xByDepth.set(roundedDepth, point.value);
					}

					const pairedData: Array<{ depth: number; xValue: number; yValue: number }> = [];
					for (const yPoint of yData.data) {
						const roundedDepth = Math.round(yPoint.depth * 10000) / 10000;
						const xValue = xByDepth.get(roundedDepth);

						if (xValue !== undefined && xValue !== null && yPoint.value !== null) {
							pairedData.push({
								depth: yPoint.depth,
								xValue: xValue,
								yValue: yPoint.value
							});
						}
					}

					if (pairedData.length > 0) {
						const frame = createCurveVsCurveFrame(
							pairedData,
							xData.mnemonic,
							yData.mnemonic,
							xData.unit,
							yData.unit,
							well?.id
						);
						onDataChange?.(frame);
					} else {
						onDataChange?.(null);
					}
					return;
				}
			}

			const frame = curveDataToFrame(yData.data, yData.mnemonic, {
				type: 'well_curve',
				wellId: well?.id,
				curveId: yAxisCurveId
			});

			if (yData.unit && frame.fields[1]) {
				frame.fields[1].unit = yData.unit;
			}

			onDataChange?.(frame);
		} catch (error) {
			console.error('[ChartSettingsDialog] Failed to load curve data:', error);
			onDataChange?.(null);
		} finally {
			isLoadingData = false;
		}
	}

	/**
	 * Load crossplot data with optional Z-axis for color coding
	 */
	async function loadCrossPlotData(crossConfig: CrossPlotConfig): Promise<void> {
		const xAxisCurveId = crossConfig.xAxis?.curveId;
		const yAxisCurveId = crossConfig.yAxis?.curveId;
		const zAxisCurveId = crossConfig.colorMode === 'curve' ? crossConfig.zAxis?.curveId : null;

		if (!xAxisCurveId || !yAxisCurveId) {
			onDataChange?.(null);
			return;
		}

		isLoadingData = true;

		try {
			const [xData, yData] = await Promise.all([
				loadCurveData(xAxisCurveId),
				loadCurveData(yAxisCurveId)
			]);

			if (!xData || !yData || xData.data.length === 0 || yData.data.length === 0) {
				onDataChange?.(null);
				return;
			}

			let zData = null;
			if (zAxisCurveId) {
				zData = await loadCurveData(zAxisCurveId);
			}

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

			const crossPlotData: Array<{ depth: number; xValue: number; yValue: number; zValue?: number }> = [];

			for (const yPoint of yData.data) {
				const roundedDepth = Math.round(yPoint.depth * 10000) / 10000;
				const xValue = xByDepth.get(roundedDepth);

				if (xValue === undefined || xValue === null || yPoint.value === null) {
					continue;
				}

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

			if (crossPlotData.length > 0) {
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
				onDataChange?.(frame);
			} else {
				onDataChange?.(null);
			}
		} catch (error) {
			console.error('[ChartSettingsDialog] Failed to load crossplot data:', error);
			onDataChange?.(null);
		} finally {
			isLoadingData = false;
		}
	}

	function getRandomWellColor(): string {
		const colors = ['#3b82f6', '#22c55e', '#ef4444', '#f59e0b', '#8b5cf6', '#06b6d4', '#ec4899', '#84cc16'];
		return colors[Math.floor(Math.random() * colors.length)];
	}

	function createCurveVsCurveFrame(
		data: Array<{ depth: number; xValue: number; yValue: number }>,
		xMnemonic: string,
		yMnemonic: string,
		xUnit: string | null,
		yUnit: string | null,
		wellId?: string
	): ChartDataFrame {
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
				depthInverted: false,
				depthRange: {
					min: depths[0],
					max: depths[depths.length - 1]
				}
			}
		};
	}

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
	): ChartDataFrame {
		const depths = new Float64Array(data.length);
		const xValues = new Float64Array(data.length);
		const yValues = new Float64Array(data.length);

		for (let i = 0; i < data.length; i++) {
			depths[i] = data[i].depth;
			xValues[i] = data[i].xValue;
			yValues[i] = data[i].yValue;
		}

		const fields: ChartDataFrame['fields'] = [
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
				depthInverted: false,
				crossplot: true,
				colorMode,
				colorMap,
				wellColor,
				zMnemonic,
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
		if (!chartConfig) return;
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
		if (!chartConfig) return;
		const wellLogConfig = chartConfig as WellLogConfig;
		const newConfig = { ...wellLogConfig, curve: binding };
		onConfigChange(newConfig);
		await loadWellLogData(newConfig);
	}

	/**
	 * Load well log data
	 */
	async function loadWellLogData(currentConfig: WellLogConfig): Promise<void> {
		const curveId = currentConfig.curve?.curveId;

		if (!curveId) {
			onDataChange?.(null);
			onSegmentedDataChange?.(null);
			return;
		}

		isLoadingData = true;

		try {
			const [curveData, segmentedData] = await Promise.all([
				loadCurveData(curveId),
				loadSegmentedCurveData(curveId)
			]);

			if (segmentedData && segmentedData.segments.length > 0) {
				onSegmentedDataChange?.(segmentedData);
			} else {
				onSegmentedDataChange?.(null);
			}

			if (!curveData || curveData.data.length === 0) {
				onDataChange?.(null);
				return;
			}

			const frame = curveDataToFrame(curveData.data, curveData.mnemonic, {
				type: 'well_curve',
				wellId: well?.id,
				curveId: curveId
			});

			if (curveData.unit && frame.fields[1]) {
				frame.fields[1].unit = curveData.unit;
			}

			frame.meta = {
				...frame.meta,
				depthInverted: currentConfig.depthInverted,
				preferredChartType: 'welllog',
			};

			onDataChange?.(frame);
		} catch (error) {
			console.error('[ChartSettingsDialog] Failed to load well log data:', error);
			onDataChange?.(null);
			onSegmentedDataChange?.(null);
		} finally {
			isLoadingData = false;
		}
	}

	// --- Correlation Chart Helpers ---

	let uniqueMnemonics = $derived.by(() => {
		const mnemonics = new Set<string>();
		for (const curve of workspaceCurves) {
			mnemonics.add(curve.mnemonic.toUpperCase());
		}
		return [...mnemonics].sort();
	});

	function toggleCurveType(mnemonic: string): void {
		if (!chartConfig || chartConfig.type !== 'correlation') return;
		const correlationConfig = chartConfig as CorrelationConfig;
		const upperMnemonic = mnemonic.toUpperCase();
		const exists = correlationConfig.selectedCurveTypes?.find(
			(ct) => ct.mnemonic.toUpperCase() === upperMnemonic
		);

		let newSelectedCurveTypes: SelectedCurveType[];
		if (exists) {
			newSelectedCurveTypes = (correlationConfig.selectedCurveTypes ?? []).filter(
				(ct) => ct.mnemonic.toUpperCase() !== upperMnemonic
			);
		} else {
			const range = getDefaultCurveRange(mnemonic);
			const newCurveType: SelectedCurveType = {
				mnemonic: upperMnemonic,
				color: getDefaultCurveColor(mnemonic),
				xMin: range?.min,
				xMax: range?.max,
				logScale: range?.logScale ?? false
			};
			newSelectedCurveTypes = [...(correlationConfig.selectedCurveTypes ?? []), newCurveType];
		}

		onConfigChange({
			...correlationConfig,
			selectedCurveTypes: newSelectedCurveTypes
		} as ChartConfiguration);

		rebuildWellTracks(newSelectedCurveTypes, correlationConfig.selectedWellIds ?? []);
	}

	function updateCurveTypeSettings(mnemonic: string, updates: Partial<SelectedCurveType>): void {
		if (!chartConfig || chartConfig.type !== 'correlation') return;
		const correlationConfig = chartConfig as CorrelationConfig;
		const upperMnemonic = mnemonic.toUpperCase();
		const updatedCurveTypes = (correlationConfig.selectedCurveTypes ?? []).map((ct) => {
			if (ct.mnemonic.toUpperCase() === upperMnemonic) {
				return { ...ct, ...updates };
			}
			return ct;
		});

		onConfigChange({
			...correlationConfig,
			selectedCurveTypes: updatedCurveTypes
		} as ChartConfiguration);

		rebuildWellTracks(updatedCurveTypes, correlationConfig.selectedWellIds ?? []);
	}

	function toggleWell(wellId: string): void {
		if (!chartConfig || chartConfig.type !== 'correlation') return;
		const correlationConfig = chartConfig as CorrelationConfig;
		const selectedWellIds = correlationConfig.selectedWellIds ?? [];
		const isSelected = selectedWellIds.includes(wellId);

		let newSelectedWellIds: string[];
		if (isSelected) {
			newSelectedWellIds = selectedWellIds.filter((id) => id !== wellId);
		} else {
			newSelectedWellIds = [...selectedWellIds, wellId];
		}

		onConfigChange({
			...correlationConfig,
			selectedWellIds: newSelectedWellIds
		} as ChartConfiguration);

		rebuildWellTracks(correlationConfig.selectedCurveTypes ?? [], newSelectedWellIds);
	}

	function getWellCurveMatchCount(wellId: string): number {
		if (!chartConfig || chartConfig.type !== 'correlation') return 0;
		const correlationConfig = chartConfig as CorrelationConfig;
		const selectedMnemonics = new Set(
			(correlationConfig.selectedCurveTypes ?? []).map((ct) => ct.mnemonic.toUpperCase())
		);
		if (selectedMnemonics.size === 0) return 0;

		const wellCurves = workspaceCurves.filter(
			(c) => c.well_id === wellId && selectedMnemonics.has(c.mnemonic.toUpperCase())
		);
		return wellCurves.length;
	}

	async function rebuildWellTracks(
		selectedCurveTypes: SelectedCurveType[],
		selectedWellIds: string[]
	): Promise<void> {
		if (!chartConfig || chartConfig.type !== 'correlation') return;
		const correlationConfig = chartConfig as CorrelationConfig;

		if (selectedCurveTypes.length === 0 || selectedWellIds.length === 0) {
			onConfigChange({
				...correlationConfig,
				wells: [],
				selectedCurveTypes,
				selectedWellIds
			} as ChartConfiguration);
			return;
		}

		const selectedMnemonics = new Set(
			selectedCurveTypes.map((ct) => ct.mnemonic.toUpperCase())
		);
		const newWells: WellCorrelationEntry[] = [];

		for (const wellId of selectedWellIds) {
			const wellInfo = wells.find((w) => w.id === wellId);
			if (!wellInfo) continue;

			const matchingCurves = workspaceCurves.filter(
				(c) => c.well_id === wellId && selectedMnemonics.has(c.mnemonic.toUpperCase())
			);

			if (matchingCurves.length === 0) continue;

			const tracks = matchingCurves.map((curve) => {
				const curveTypeConfig = selectedCurveTypes.find(
					(ct) => ct.mnemonic.toUpperCase() === curve.mnemonic.toUpperCase()
				);
				return createTrackWithDefaults(wellId, curve.id, curve.mnemonic, curveTypeConfig);
			});

			newWells.push({
				wellId,
				wellName: wellInfo.name,
				wellColor: getNextWellColor(newWells.length),
				tracks
			});
		}

		onConfigChange({
			...correlationConfig,
			selectedCurveTypes,
			selectedWellIds,
			wells: newWells
		} as ChartConfiguration);

		for (const well of newWells) {
			for (const track of well.tracks) {
				await loadTrackCurveData(well.wellId, track);
			}
		}
	}

	async function loadTrackCurveData(wellId: string, track: CorrelationTrack): Promise<void> {
		try {
			const segmentedData = await loadSegmentedCurveData(track.curveId);
			if (segmentedData) {
				const correlationCurveData: CorrelationCurveData = {
					trackId: track.id,
					mnemonic: segmentedData.mnemonic,
					unit: segmentedData.unit,
					segments: segmentedData.segments.map((seg) => ({
						depthStart: seg.depth_start,
						depthEnd: seg.depth_end,
						depths: seg.depths,
						values: seg.values
					})),
					depthRange: {
						min: segmentedData.depth_range[0],
						max: segmentedData.depth_range[1]
					},
					totalPoints: segmentedData.total_points,
					source: {
						type: 'well_curve',
						wellId: wellId,
						curveId: track.curveId
					}
				};
				onCorrelationCurveDataChange?.(track.id, correlationCurveData);
			}
		} catch (error) {
			console.error('[ChartSettingsDialog] Failed to load track curve data:', error);
		}
	}

	function updateCorrelationDepthRange(updates: Partial<CorrelationConfig['depthRange']>): void {
		if (!chartConfig || chartConfig.type !== 'correlation') return;
		const correlationConfig = chartConfig as CorrelationConfig;
		onConfigChange({
			...correlationConfig,
			depthRange: { ...correlationConfig.depthRange, ...updates }
		} as ChartConfiguration);
	}

	function updateCorrelationConfig<K extends keyof CorrelationConfig>(
		key: K,
		value: CorrelationConfig[K]
	): void {
		if (!chartConfig || chartConfig.type !== 'correlation') return;
		const correlationConfig = chartConfig as CorrelationConfig;
		onConfigChange({
			...correlationConfig,
			[key]: value
		} as ChartConfiguration);
	}

	function updateLayoutConfig(updates: Partial<CorrelationLayoutConfig>): void {
		if (!chartConfig || chartConfig.type !== 'correlation') return;
		const correlationConfig = chartConfig as CorrelationConfig;
		onConfigChange({
			...correlationConfig,
			layout: { ...(correlationConfig.layout ?? DEFAULT_LAYOUT), ...updates }
		} as ChartConfiguration);
	}

	// =========================================================================
	// Multi-Well Support for Non-Correlation Charts (CrossPlot, Scatter, Line)
	// =========================================================================

	/** Toggle well selection for multi-well charts (crossplot, scatter, line) */
	function toggleMultiWell(wellId: string): void {
		if (!chartConfig) return;
		if (chartConfig.type !== 'crossplot' && chartConfig.type !== 'scatter' && chartConfig.type !== 'line') return;

		const config = chartConfig as CrossPlotConfig | ScatterChartConfig | LineChartConfig;
		const selectedWellIds = config.selectedWellIds ?? [];
		const isSelected = selectedWellIds.includes(wellId);

		const newSelectedWellIds = isSelected
			? selectedWellIds.filter((id) => id !== wellId)
			: [...selectedWellIds, wellId];

		onConfigChange({
			...config,
			selectedWellIds: newSelectedWellIds
		} as ChartConfiguration);
	}

	/** Check if chart type supports multi-well selection */
	function supportsMultiWell(chartType: string): boolean {
		return chartType === 'crossplot' || chartType === 'scatter' || chartType === 'line';
	}

	/** Get selected well IDs for multi-well charts */
	function getSelectedWellIds(): string[] {
		if (!chartConfig) return [];
		if (!supportsMultiWell(chartConfig.type)) return [];
		const config = chartConfig as CrossPlotConfig | ScatterChartConfig | LineChartConfig;
		return config.selectedWellIds ?? [];
	}

	/** Update curve type for multi-well chart axis */
	function updateMultiWellCurveType(axis: 'x' | 'y' | 'z', mnemonic: string | null): void {
		if (!chartConfig) return;
		if (!supportsMultiWell(chartConfig.type)) return;

		const config = chartConfig as CrossPlotConfig | ScatterChartConfig | LineChartConfig;
		const updates: Partial<typeof config> = {};

		if (axis === 'x') {
			updates.xCurveType = mnemonic ?? undefined;
		} else if (axis === 'y') {
			updates.yCurveType = mnemonic ?? undefined;
		} else if (axis === 'z' && chartConfig.type === 'crossplot') {
			(updates as Partial<CrossPlotConfig>).zCurveType = mnemonic ?? undefined;
		}

		onConfigChange({
			...config,
			...updates
		} as ChartConfiguration);
	}

	/** Get curve type for an axis */
	function getMultiWellCurveType(axis: 'x' | 'y' | 'z'): string | undefined {
		if (!chartConfig) return undefined;
		if (!supportsMultiWell(chartConfig.type)) return undefined;

		const config = chartConfig as CrossPlotConfig | ScatterChartConfig | LineChartConfig;
		if (axis === 'x') return config.xCurveType;
		if (axis === 'y') return config.yCurveType;
		if (axis === 'z' && chartConfig.type === 'crossplot') {
			return (config as CrossPlotConfig).zCurveType;
		}
		return undefined;
	}

	/** Check if a well has curves matching the selected curve types */
	function getMultiWellCurveMatchCount(wellId: string): number {
		if (!chartConfig) return 0;
		if (!supportsMultiWell(chartConfig.type)) return 0;

		const config = chartConfig as CrossPlotConfig | ScatterChartConfig | LineChartConfig;
		const xType = config.xCurveType?.toUpperCase();
		const yType = config.yCurveType?.toUpperCase();
		const zType = chartConfig.type === 'crossplot' ? (config as CrossPlotConfig).zCurveType?.toUpperCase() : undefined;

		const requiredTypes = [xType, yType, zType].filter(Boolean);
		if (requiredTypes.length === 0) return 0;

		const wellCurves = workspaceCurves.filter((c) => c.well_id === wellId);
		const wellMnemonics = new Set(wellCurves.map((c) => c.mnemonic.toUpperCase()));

		return requiredTypes.filter((type) => wellMnemonics.has(type!)).length;
	}

	/** Get total required curve types count */
	function getRequiredCurveTypeCount(): number {
		if (!chartConfig) return 0;
		if (!supportsMultiWell(chartConfig.type)) return 0;

		const config = chartConfig as CrossPlotConfig | ScatterChartConfig | LineChartConfig;
		let count = 0;
		if (config.xCurveType) count++;
		if (config.yCurveType) count++;
		if (chartConfig.type === 'crossplot' && (config as CrossPlotConfig).zCurveType) count++;
		return count;
	}

	/** Load multi-well data when selections change */
	async function loadMultiWellData(): Promise<void> {
		if (!chartConfig) return;
		if (!supportsMultiWell(chartConfig.type)) return;

		const config = chartConfig as CrossPlotConfig | ScatterChartConfig | LineChartConfig;
		const selectedWellIds = config.selectedWellIds ?? [];
		const xCurveType = config.xCurveType;
		const yCurveType = config.yCurveType;
		const zCurveType = chartConfig.type === 'crossplot' ? (config as CrossPlotConfig).zCurveType : undefined;

		// Need at least one well and both X/Y curve types
		if (selectedWellIds.length === 0 || !xCurveType || !yCurveType) {
			onMultiWellDataChange?.([]);
			return;
		}

		const multiWellData: MultiWellCurveData[] = [];
		const wellColors = ['#3b82f6', '#22c55e', '#ef4444', '#f59e0b', '#8b5cf6', '#06b6d4', '#ec4899', '#84cc16'];

		for (let i = 0; i < selectedWellIds.length; i++) {
			const wellId = selectedWellIds[i];
			const wellInfo = wells.find(w => w.id === wellId);
			if (!wellInfo) continue;

			// Find curves for this well matching the curve types
			const wellCurves = workspaceCurves.filter(c => c.well_id === wellId);
			const xCurve = wellCurves.find(c => c.mnemonic.toUpperCase() === xCurveType.toUpperCase());
			const yCurve = wellCurves.find(c => c.mnemonic.toUpperCase() === yCurveType.toUpperCase());
			const zCurve = zCurveType ? wellCurves.find(c => c.mnemonic.toUpperCase() === zCurveType.toUpperCase()) : undefined;

			// Load curve data
			let xData: SegmentedCurveData | null = null;
			let yData: SegmentedCurveData | null = null;
			let zData: SegmentedCurveData | null = null;

			try {
				if (xCurve) {
					xData = await loadSegmentedCurveData(xCurve.id);
				}
				if (yCurve) {
					yData = await loadSegmentedCurveData(yCurve.id);
				}
				if (zCurve) {
					zData = await loadSegmentedCurveData(zCurve.id);
				}
			} catch (error) {
				console.error('[ChartSettingsDialog] Failed to load multi-well data for', wellId, error);
				continue;
			}

			if (xData && yData) {
				multiWellData.push({
					wellId,
					wellName: wellInfo.name,
					wellColor: wellColors[i % wellColors.length],
					xCurve: xData,
					yCurve: yData,
					zCurve: zData
				});
			}
		}

		onMultiWellDataChange?.(multiWellData);
	}

	// Trigger multi-well data loading when config changes
	$effect(() => {
		if (!chartConfig) return;
		if (!supportsMultiWell(chartConfig.type)) return;

		const config = chartConfig as CrossPlotConfig | ScatterChartConfig | LineChartConfig;
		// Tracking these values will re-run the effect when they change
		const _selectedWellIds = config.selectedWellIds;
		const _xCurveType = config.xCurveType;
		const _yCurveType = config.yCurveType;

		loadMultiWellData();
	});
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<dialog
	bind:this={dialogRef}
	class="chart-settings-dialog"
	onclick={handleDialogClick}
	onkeydown={handleKeyDown}
>
	<div class="dialog-container">
		<header class="dialog-header">
			<h2 class="dialog-title">
				{#if pane && chartConfig}
					{getChartTypeName(chartConfig.type)} Settings
				{:else}
					Chart Settings
				{/if}
			</h2>
			<button class="close-button" onclick={onClose} aria-label="Close dialog">
				<svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
					<path
						fill="currentColor"
						d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12 19 6.41z"
					/>
				</svg>
			</button>
		</header>

		{#if pane && chartConfig}
			<!-- Tab Navigation -->
			<nav class="tab-nav" role="tablist">
				{#each availableTabs as tab (tab.id)}
					<button
						class="tab-button"
						class:active={activeTab === tab.id}
						onclick={() => activeTab = tab.id}
						role="tab"
						aria-selected={activeTab === tab.id}
					>
						<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
							<path fill="currentColor" d={tab.icon} />
						</svg>
						<span>{tab.label}</span>
					</button>
				{/each}
			</nav>

			<!-- Tab Content -->
			<div class="dialog-content">
				<!-- DATA TAB -->
				{#if activeTab === 'data'}
					<div class="tab-panel">
						<!-- Correlation charts get their own special data UI - no single well selector needed -->
						{#if chartConfig.type === 'correlation'}
							{@const correlationConfig = chartConfig as CorrelationConfig}
							<!-- Curve Types Section - show all available mnemonics from workspace -->
							<div class="config-section">
								<h4 class="section-title">Curve Types</h4>
								<p class="section-description">Select which curve types to display across all wells</p>
								{#if uniqueMnemonics.length === 0}
									<div class="hint-box">
										<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
											<path d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
										</svg>
										<span>No curves available in this workspace</span>
									</div>
								{:else}
									<div class="curve-type-list">
										{#each uniqueMnemonics as mnemonic (mnemonic)}
											{@const isSelected = (correlationConfig.selectedCurveTypes ?? []).some(ct => ct.mnemonic.toUpperCase() === mnemonic)}
											<label class="curve-type-item">
												<input
													type="checkbox"
													checked={isSelected}
													onchange={() => toggleCurveType(mnemonic)}
												/>
												<span class="curve-type-dot" style="background: {getDefaultCurveColor(mnemonic)}"></span>
												<span class="curve-type-name">{mnemonic}</span>
											</label>
										{/each}
									</div>

									{#if (correlationConfig.selectedCurveTypes ?? []).length > 0}
										<div class="curve-type-settings-list">
											<p class="settings-hint">Configure X-axis range for each curve type:</p>
											{#each correlationConfig.selectedCurveTypes ?? [] as curveType (curveType.mnemonic)}
												<div class="curve-type-settings">
													<span class="curve-type-label" style="color: {curveType.color}">{curveType.mnemonic}</span>
													<div class="curve-type-inputs">
														<input
															type="number"
															class="field-input small"
															placeholder="Min"
															value={curveType.xMin ?? ''}
															onchange={(e) => updateCurveTypeSettings(curveType.mnemonic, {
																xMin: e.currentTarget.value ? parseFloat(e.currentTarget.value) : undefined
															})}
														/>
														<span class="input-separator">-</span>
														<input
															type="number"
															class="field-input small"
															placeholder="Max"
															value={curveType.xMax ?? ''}
															onchange={(e) => updateCurveTypeSettings(curveType.mnemonic, {
																xMax: e.currentTarget.value ? parseFloat(e.currentTarget.value) : undefined
															})}
														/>
														<label class="checkbox-label compact">
															<input
																type="checkbox"
																checked={curveType.logScale ?? false}
																onchange={(e) => updateCurveTypeSettings(curveType.mnemonic, {
																	logScale: e.currentTarget.checked
																})}
															/>
															<span>Log</span>
														</label>
													</div>
												</div>
											{/each}
										</div>
									{/if}
								{/if}
							</div>

							<!-- Wells Section - multi-select all wells in workspace -->
							<div class="config-section">
								<h4 class="section-title">Wells</h4>
								<p class="section-description">Select which wells to include in the correlation</p>
								{#if wells.length === 0}
									<div class="hint-box">
										<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
											<path d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
										</svg>
										<span>No wells available in workspace</span>
									</div>
								{:else}
									<div class="wells-checklist">
										{#each wells as wellInfo (wellInfo.id)}
											{@const isSelected = (correlationConfig.selectedWellIds ?? []).includes(wellInfo.id)}
											{@const matchCount = getWellCurveMatchCount(wellInfo.id)}
											{@const totalSelected = (correlationConfig.selectedCurveTypes ?? []).length}
											<label class="well-checkbox-item">
												<input
													type="checkbox"
													checked={isSelected}
													onchange={() => toggleWell(wellInfo.id)}
												/>
												<span class="well-checkbox-name">{wellInfo.name}</span>
												{#if totalSelected > 0}
													<span class="match-badge" class:partial={matchCount < totalSelected && matchCount > 0} class:none={matchCount === 0}>
														{matchCount}/{totalSelected}
													</span>
												{:else}
													<span class="curve-count">({wellInfo.curve_count} curves)</span>
												{/if}
											</label>
										{/each}
									</div>
								{/if}
							</div>
						{:else if supportsMultiWell(chartConfig.type)}
							<!-- Multi-well charts: CrossPlot, Scatter, Line -->
							<!-- Curve Type Selection -->
							<div class="config-section">
								<h4 class="section-title">Curve Types</h4>
								<p class="section-description">Select curve types for each axis (applied across all selected wells)</p>

								{#if uniqueMnemonics.length === 0}
									<div class="hint-box">
										<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
											<path d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
										</svg>
										<span>No curves available in this workspace</span>
									</div>
								{:else}
									<div class="field-group">
										<label class="field-label" for="x-curve-type">X Axis Curve</label>
										<select
											id="x-curve-type"
											class="field-select"
											value={getMultiWellCurveType('x') ?? ''}
											onchange={(e) => updateMultiWellCurveType('x', e.currentTarget.value || null)}
										>
											<option value="">Select X axis curve type...</option>
											{#each uniqueMnemonics as mnemonic (mnemonic)}
												<option value={mnemonic}>{mnemonic}</option>
											{/each}
										</select>
									</div>

									<div class="field-group">
										<label class="field-label" for="y-curve-type">Y Axis Curve</label>
										<select
											id="y-curve-type"
											class="field-select"
											value={getMultiWellCurveType('y') ?? ''}
											onchange={(e) => updateMultiWellCurveType('y', e.currentTarget.value || null)}
										>
											<option value="">Select Y axis curve type...</option>
											{#each uniqueMnemonics as mnemonic (mnemonic)}
												<option value={mnemonic}>{mnemonic}</option>
											{/each}
										</select>
									</div>

									{#if chartConfig.type === 'crossplot'}
										{@const crossConfig = chartConfig as CrossPlotConfig}
										{#if crossConfig.colorMode === 'curve'}
											<div class="field-group">
												<label class="field-label" for="z-curve-type">Z Axis Curve (Color)</label>
												<select
													id="z-curve-type"
													class="field-select"
													value={getMultiWellCurveType('z') ?? ''}
													onchange={(e) => updateMultiWellCurveType('z', e.currentTarget.value || null)}
												>
													<option value="">Select Z axis curve type...</option>
													{#each uniqueMnemonics as mnemonic (mnemonic)}
														<option value={mnemonic}>{mnemonic}</option>
													{/each}
												</select>
											</div>
										{/if}
									{/if}
								{/if}
							</div>

							<!-- Wells Section - multi-select -->
							<div class="config-section">
								<h4 class="section-title">Wells</h4>
								<p class="section-description">Select wells to display (same curve types plotted per well)</p>
								{#if wells.length === 0}
									<div class="hint-box">
										<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
											<path d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
										</svg>
										<span>No wells available in workspace</span>
									</div>
								{:else}
									<div class="wells-checklist">
										{#each wells as wellInfo (wellInfo.id)}
											{@const isSelected = getSelectedWellIds().includes(wellInfo.id)}
											{@const matchCount = getMultiWellCurveMatchCount(wellInfo.id)}
											{@const totalRequired = getRequiredCurveTypeCount()}
											<label class="well-checkbox-item">
												<input
													type="checkbox"
													checked={isSelected}
													onchange={() => toggleMultiWell(wellInfo.id)}
												/>
												<span class="well-checkbox-name">{wellInfo.name}</span>
												{#if totalRequired > 0}
													<span class="match-badge" class:partial={matchCount < totalRequired && matchCount > 0} class:none={matchCount === 0}>
														{matchCount}/{totalRequired}
													</span>
												{:else}
													<span class="curve-count">({wellInfo.curve_count} curves)</span>
												{/if}
											</label>
										{/each}
									</div>
								{/if}
							</div>
						{:else}
							<!-- Histogram and WellLog: Single well selection -->
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
									<div class="hint-box">
										<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
											<path d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0z" />
										</svg>
										<span>Select a well to choose curves for plotting</span>
									</div>
								{/if}
							</div>

							{#if well}
								<!-- Curve Bindings based on chart type -->
								{#if chartConfig.type === 'histogram'}
									{@const histConfig = chartConfig as HistogramConfig}
									<div class="config-section">
										<h4 class="section-title">Curve Binding</h4>
										<CurveSelector
											label="Data Curve"
											binding={histConfig.dataCurve}
											{curves}
											{well}
											required
											onChange={(binding) => updateAxis('dataCurve', binding)}
										/>
									</div>
								{:else if chartConfig.type === 'welllog'}
									{@const wellLogConfig = chartConfig as WellLogConfig}
									<div class="config-section">
										<h4 class="section-title">Curve Binding</h4>
										<CurveSelector
											label="Curve"
											binding={wellLogConfig.curve}
											{curves}
											{well}
											required
											onChange={(binding) => updateWellLogCurve(binding)}
										/>
									</div>
								{/if}
							{/if}
						{/if}
					</div>
				{/if}

				<!-- AXES TAB -->
				{#if activeTab === 'axes'}
					<div class="tab-panel">
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
						{:else if chartConfig.type === 'correlation'}
							{@const correlationConfig = chartConfig as CorrelationConfig}
							<div class="config-section">
								<h4 class="section-title">Depth Range</h4>

								<label class="checkbox-label">
									<input
										type="checkbox"
										checked={correlationConfig.depthRange.autoScale}
										onchange={(e) => updateCorrelationDepthRange({ autoScale: e.currentTarget.checked })}
									/>
									<span>Auto-scale from all curves</span>
								</label>

								{#if !correlationConfig.depthRange.autoScale}
									<div class="field-row">
										<div class="field-group">
											<label class="field-label" for="corr-depth-min">Min Depth</label>
											<input
												id="corr-depth-min"
												type="number"
												class="field-input"
												value={correlationConfig.depthRange.min ?? ''}
												onchange={(e) => updateCorrelationDepthRange({
													min: e.currentTarget.value ? parseFloat(e.currentTarget.value) : null
												})}
											/>
										</div>
										<div class="field-group">
											<label class="field-label" for="corr-depth-max">Max Depth</label>
											<input
												id="corr-depth-max"
												type="number"
												class="field-input"
												value={correlationConfig.depthRange.max ?? ''}
												onchange={(e) => updateCorrelationDepthRange({
													max: e.currentTarget.value ? parseFloat(e.currentTarget.value) : null
												})}
											/>
										</div>
									</div>
								{/if}

								<label class="checkbox-label">
									<input
										type="checkbox"
										checked={correlationConfig.depthRange.inverted}
										onchange={(e) => updateCorrelationDepthRange({ inverted: e.currentTarget.checked })}
									/>
									<span>Invert Depth (Increasing Downward)</span>
								</label>
							</div>
						{:else}
							<div class="config-section">
								<h4 class="section-title">Axis Configuration</h4>
								<p class="hint-text">Axis settings are automatically configured based on your data.</p>
							</div>
						{/if}
					</div>
				{/if}

				<!-- STYLE TAB -->
				{#if activeTab === 'style'}
					<div class="tab-panel">
						{#if chartConfig.type === 'line'}
							{@const lineConfig = chartConfig as LineChartConfig}
							<div class="config-section">
								<h4 class="section-title">Line Style</h4>

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
						{:else if chartConfig.type === 'scatter'}
							{@const scatterConfig = chartConfig as ScatterChartConfig}
							<div class="config-section">
								<h4 class="section-title">Point Style</h4>

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
							</div>
						{:else if chartConfig.type === 'histogram'}
							{@const histConfig = chartConfig as HistogramConfig}
							<div class="config-section">
								<h4 class="section-title">Histogram Style</h4>

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
						{:else if chartConfig.type === 'crossplot'}
							{@const crossConfig = chartConfig as CrossPlotConfig}
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
											if (newMode === 'well' && !crossConfig.wellColor) {
												newConfig.wellColor = getRandomWellColor();
											}
											onConfigChange(newConfig);
											loadChartData(newConfig);
										}}
									>
										<option value="none">Uniform Color</option>
										<option value="curve">Z-Axis Curve</option>
										<option value="well">Well</option>
									</select>
								</div>

								{#if crossConfig.colorMode === 'curve'}
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
												loadChartData(newConfig);
											}}
										>
											<option value="viridis">Viridis</option>
											<option value="plasma">Plasma</option>
											<option value="rainbow">Rainbow</option>
											<option value="grayscale">Grayscale</option>
										</select>
									</div>
								{:else if crossConfig.colorMode === 'well'}
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
								{:else}
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
								<h4 class="section-title">Point Style</h4>

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
							</div>
						{:else if chartConfig.type === 'welllog'}
							{@const wellLogConfig = chartConfig as WellLogConfig}
							<div class="config-section">
								<h4 class="section-title">Line Style</h4>

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
						{:else}
							<div class="config-section">
								<p class="hint-text">Style options are not available for this chart type.</p>
							</div>
						{/if}
					</div>
				{/if}

				<!-- DISPLAY TAB -->
				{#if activeTab === 'display'}
					<div class="tab-panel">
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
					</div>
				{/if}

				<!-- ADVANCED TAB -->
				{#if activeTab === 'advanced'}
					<div class="tab-panel">
						{#if chartConfig.type === 'line'}
							{@const lineConfig = chartConfig as LineChartConfig}
							<div class="config-section">
								<h4 class="section-title">Line Options</h4>

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
							</div>
						{:else if chartConfig.type === 'scatter'}
							{@const scatterConfig = chartConfig as ScatterChartConfig}
							<div class="config-section">
								<h4 class="section-title">Trend Line</h4>

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
						{:else if chartConfig.type === 'histogram'}
							{@const histConfig = chartConfig as HistogramConfig}
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
						{:else if chartConfig.type === 'crossplot'}
							{@const crossConfig = chartConfig as CrossPlotConfig}
							<div class="config-section">
								<h4 class="section-title">Regression</h4>

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
						{:else if chartConfig.type === 'correlation'}
							{@const correlationConfig = chartConfig as CorrelationConfig}
							<div class="config-section">
								<h4 class="section-title">Layout</h4>

								<div class="field-group">
									<label class="field-label" for="track-width">Track Width (px)</label>
									<input
										id="track-width"
										type="number"
										class="field-input"
										min="80"
										max="300"
										value={correlationConfig.layout?.trackWidth ?? 140}
										onchange={(e) => updateLayoutConfig({
											trackWidth: parseInt(e.currentTarget.value) || 140
										})}
									/>
								</div>
							</div>
						{:else}
							<div class="config-section">
								<p class="hint-text">No advanced options available for this chart type.</p>
							</div>
						{/if}
					</div>
				{/if}

				<!-- WELL TOPS TAB (Correlation Charts Only) -->
				{#if activeTab === 'wellTops' && chartConfig.type === 'correlation'}
					{@const correlationConfig = chartConfig as CorrelationConfig}
					<div class="tab-panel">
						<div class="config-section">
							<h4 class="section-title">Well Tops (Formation Markers)</h4>
							<p class="hint-text">Load well tops from the database and display them as horizontal lines across the correlation chart.</p>
						</div>

						<!-- Well Selector for loading tops -->
						<div class="config-section">
							<div class="field-group">
								<label class="field-label" for="well-tops-selector">Load Well Tops From</label>
								<select
									id="well-tops-selector"
									class="field-select"
									value={selectedWellForTops}
									onchange={(e) => loadWellTops(e.currentTarget.value)}
								>
									<option value="">Select a well...</option>
									{#each correlationConfig.wells as well}
										<option value={well.wellId}>{well.wellName}</option>
									{/each}
								</select>
							</div>
						</div>

						<!-- Loading indicator -->
						{#if loadingWellTops}
							<div class="config-section">
								<p class="loading-text">Loading well tops...</p>
							</div>
						{/if}

						<!-- Available Well Tops from Database -->
						{#if availableWellTops.length > 0}
							<div class="config-section">
								<h4 class="section-title">Available Markers ({availableWellTops.length})</h4>
								<div class="well-tops-list">
									{#each availableWellTops as marker (marker.id)}
										<div class="marker-item">
											<label class="marker-checkbox">
												<input
													type="checkbox"
													checked={isWellTopSelected(marker)}
													onchange={() => toggleWellTop(marker)}
												/>
												<span class="marker-info">
													<span class="marker-name">{marker.name}</span>
													<span class="marker-depth">{marker.measured_depth.toFixed(1)} m</span>
												</span>
											</label>
											{#if isWellTopSelected(marker)}
												<input
													type="color"
													class="marker-color"
													value={getWellTopColor(marker)}
													onchange={(e) => setWellTopColor(marker, e.currentTarget.value)}
													title="Change color"
												/>
											{/if}
										</div>
									{/each}
								</div>
							</div>
						{:else if selectedWellForTops && !loadingWellTops}
							<div class="config-section">
								<p class="hint-text">No well tops found for this well.</p>
							</div>
						{/if}

						<!-- Active Well Tops -->
						{#if correlationConfig.wellTops.length > 0}
							<div class="config-section">
								<h4 class="section-title">Active Well Tops ({correlationConfig.wellTops.length})</h4>
								<div class="active-tops-list">
									{#each correlationConfig.wellTops as top (top.name + top.depth)}
										<div class="active-top-item">
											<div class="top-color-indicator" style="background: {top.color}"></div>
											<span class="top-name">{top.name}</span>
											<span class="top-depth">{top.depth.toFixed(1)} m</span>
											<button
												class="remove-top-btn"
												onclick={() => removeWellTop(top)}
												title="Remove well top"
											>
												×
											</button>
										</div>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{:else}
			<div class="dialog-content">
				<div class="no-pane-message">
					<p>No chart selected. Select a chart pane to configure its settings.</p>
				</div>
			</div>
		{/if}
	</div>
</dialog>

<style>
	.chart-settings-dialog {
		position: fixed;
		/* Center the dialog */
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		margin: 0;
		max-width: 600px;
		max-height: 80vh;
		width: 90vw;
		padding: 0;
		border: none;
		border-radius: 12px;
		box-shadow:
			0 25px 50px -12px rgba(0, 0, 0, 0.25),
			0 0 0 1px hsl(var(--border));
		background: hsl(var(--background));
	}

	.chart-settings-dialog::backdrop {
		background: rgba(0, 0, 0, 0.5);
		backdrop-filter: blur(4px);
	}

	.dialog-container {
		display: flex;
		flex-direction: column;
		max-height: 80vh;
	}

	.dialog-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px;
		border-bottom: 1px solid hsl(var(--border));
		background: hsl(var(--muted));
	}

	.dialog-title {
		font-size: 16px;
		font-weight: 600;
		color: hsl(var(--foreground));
		margin: 0;
	}

	.close-button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		padding: 0;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: hsl(var(--muted-foreground));
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			color 0.15s ease;
	}

	.close-button:hover {
		background: hsl(var(--accent));
		color: hsl(var(--foreground));
	}

	.close-button:focus-visible {
		outline: 2px solid hsl(var(--ring));
		outline-offset: 2px;
	}

	/* Tab Navigation */
	.tab-nav {
		display: flex;
		gap: 2px;
		padding: 8px 16px;
		background: hsl(var(--muted));
		border-bottom: 1px solid hsl(var(--border));
	}

	.tab-button {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 12px;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: hsl(var(--muted-foreground));
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			color 0.15s ease;
	}

	.tab-button:hover {
		background: hsl(var(--accent));
		color: hsl(var(--foreground));
	}

	.tab-button.active {
		background: hsl(var(--background));
		color: hsl(var(--foreground));
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
	}

	.tab-button svg {
		flex-shrink: 0;
	}

	.dialog-content {
		flex: 1;
		overflow-y: auto;
		min-height: 300px;
	}

	.tab-panel {
		padding: 16px 20px;
	}

	.no-pane-message {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 48px 24px;
		text-align: center;
		color: hsl(var(--muted-foreground));
	}

	/* Config Section Styles */
	.config-section {
		margin-bottom: 20px;
	}

	.section-title {
		font-size: 12px;
		font-weight: 600;
		color: hsl(var(--muted-foreground));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0 0 12px 0;
		padding-bottom: 8px;
		border-bottom: 1px solid hsl(var(--border));
	}

	.field-group {
		margin-bottom: 12px;
	}

	.field-label {
		display: block;
		font-size: 12px;
		font-weight: 500;
		color: hsl(var(--muted-foreground));
		margin-bottom: 4px;
	}

	.field-input,
	.field-select {
		width: 100%;
		padding: 8px 12px;
		border: 1px solid hsl(var(--border));
		border-radius: 6px;
		font-size: 13px;
		background: hsl(var(--background));
		color: hsl(var(--foreground));
	}

	.field-input.small {
		width: 80px;
	}

	.field-input:focus,
	.field-select:focus {
		outline: none;
		border-color: hsl(var(--primary));
		box-shadow: 0 0 0 3px hsla(var(--primary), 0.1);
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
		color: hsl(var(--foreground));
		cursor: pointer;
		margin-bottom: 8px;
	}

	.checkbox-label input[type='checkbox'] {
		width: 16px;
		height: 16px;
		cursor: pointer;
	}

	.hint-box {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		background: hsl(var(--accent));
		border-radius: 6px;
		font-size: 12px;
		color: hsl(var(--muted-foreground));
	}

	.hint-box svg {
		flex-shrink: 0;
		opacity: 0.7;
	}

	.hint-text {
		font-size: 13px;
		color: hsl(var(--muted-foreground));
		font-style: italic;
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
		border-color: hsl(var(--foreground));
	}

	/* Curve Type Styles */
	.curve-type-list {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-bottom: 12px;
	}

	.curve-type-item {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 8px;
		background: hsl(var(--accent));
		border-radius: 4px;
		font-size: 12px;
		cursor: pointer;
		transition: background 0.15s ease;
	}

	.curve-type-item:hover {
		background: hsl(var(--muted));
	}

	.curve-type-item input[type='checkbox'] {
		width: 14px;
		height: 14px;
		cursor: pointer;
	}

	.curve-type-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.curve-type-name {
		font-weight: 500;
		color: hsl(var(--foreground));
	}

	.curve-type-settings-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-top: 8px;
		padding-top: 8px;
		border-top: 1px solid hsl(var(--border));
	}

	.curve-type-settings {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		background: hsl(var(--accent));
		border-radius: 4px;
	}

	.curve-type-label {
		font-size: 11px;
		font-weight: 600;
		min-width: 40px;
	}

	.curve-type-inputs {
		display: flex;
		align-items: center;
		gap: 6px;
		flex: 1;
	}

	.input-separator {
		color: hsl(var(--muted-foreground));
		font-size: 12px;
	}

	.checkbox-label.compact {
		margin-bottom: 0;
		font-size: 11px;
		gap: 4px;
	}

	.checkbox-label.compact input[type='checkbox'] {
		width: 14px;
		height: 14px;
	}

	/* Wells Checklist */
	.wells-checklist {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.well-checkbox-item {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		background: hsl(var(--accent));
		border-radius: 4px;
		font-size: 12px;
		cursor: pointer;
		transition: background 0.15s ease;
	}

	.well-checkbox-item:hover {
		background: hsl(var(--muted));
	}

	.well-checkbox-item input[type='checkbox'] {
		width: 14px;
		height: 14px;
		cursor: pointer;
	}

	.well-checkbox-name {
		flex: 1;
		font-weight: 500;
		color: hsl(var(--foreground));
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.match-badge {
		padding: 2px 6px;
		font-size: 10px;
		font-weight: 500;
		border-radius: 3px;
		background: hsl(142 76% 90%);
		color: hsl(142 76% 36%);
	}

	.match-badge.partial {
		background: hsl(45 93% 90%);
		color: hsl(45 93% 35%);
	}

	.match-badge.none {
		background: hsl(0 0% 90%);
		color: hsl(0 0% 50%);
	}

	.curve-count {
		font-size: 11px;
		color: hsl(var(--muted-foreground));
	}

	.section-description {
		font-size: 12px;
		color: hsl(var(--muted-foreground));
		margin: -8px 0 12px 0;
	}

	.settings-hint {
		font-size: 11px;
		color: hsl(var(--muted-foreground));
		margin: 0 0 8px 0;
	}

	/* Well Tops Tab Styles */
	.loading-text {
		font-size: 12px;
		color: hsl(var(--muted-foreground));
		font-style: italic;
	}

	.well-tops-list {
		display: flex;
		flex-direction: column;
		gap: 4px;
		max-height: 200px;
		overflow-y: auto;
		padding-right: 4px;
	}

	.marker-item {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		background: hsl(var(--accent));
		border-radius: 4px;
	}

	.marker-checkbox {
		display: flex;
		align-items: center;
		gap: 8px;
		cursor: pointer;
		flex: 1;
		min-width: 0;
	}

	.marker-checkbox input[type='checkbox'] {
		width: 14px;
		height: 14px;
		cursor: pointer;
		flex-shrink: 0;
	}

	.marker-info {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-width: 0;
	}

	.marker-name {
		font-size: 12px;
		font-weight: 500;
		color: hsl(var(--foreground));
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
	}

	.marker-depth {
		font-size: 11px;
		color: hsl(var(--muted-foreground));
		flex-shrink: 0;
	}

	.marker-color {
		width: 24px;
		height: 24px;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		padding: 0;
		flex-shrink: 0;
	}

	.active-tops-list {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.active-top-item {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		background: hsl(var(--accent));
		border-radius: 4px;
	}

	.top-color-indicator {
		width: 12px;
		height: 12px;
		border-radius: 2px;
		flex-shrink: 0;
	}

	.top-name {
		font-size: 12px;
		font-weight: 500;
		color: hsl(var(--foreground));
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.top-depth {
		font-size: 11px;
		color: hsl(var(--muted-foreground));
		flex-shrink: 0;
	}

	.remove-top-btn {
		width: 20px;
		height: 20px;
		border: none;
		background: hsl(var(--destructive) / 0.1);
		color: hsl(var(--destructive));
		border-radius: 4px;
		cursor: pointer;
		font-size: 14px;
		font-weight: 600;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background 0.15s ease;
		flex-shrink: 0;
	}

	.remove-top-btn:hover {
		background: hsl(var(--destructive) / 0.2);
	}
</style>
