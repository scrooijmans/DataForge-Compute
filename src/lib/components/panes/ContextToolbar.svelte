<script lang="ts">
	/**
	 * ContextToolbar - Context-sensitive toolbar panel
	 *
	 * Displays different content based on what is currently selected:
	 * - UDF selected: Shows UDF parameters and execution controls
	 * - Chart pane selected: Shows chart configuration options
	 * - Nothing selected: Shows placeholder with instructions
	 */
	import type { CurveInfo, WellInfo, SegmentedCurveData } from '$lib/types';
	import { selectionContext } from '$lib/panes/selection-context';
	import type { ChartConfiguration } from '$lib/panes/chart-configs';
	import { workspaceManager } from '$lib/panes/workspace-manager';
	import ChartConfigPanel from './ChartConfigPanel.svelte';
	import ParameterForm from '$lib/components/ParameterForm.svelte';
	import type { ChartDataFrame } from '$lib/charts/types';
	import type { CorrelationCurveData } from '$lib/charts/correlation-types';

	interface Props {
		/** Available wells */
		wells: WellInfo[];
		/** Available curves for the selected well */
		curves: CurveInfo[];
		/** Selected well info */
		well: WellInfo | null;
		/** Callback when well selection changes */
		onWellChange?: (wellId: string) => void;
	}

	let { wells, curves, well, onWellChange }: Props = $props();

	/** Selection stores */
	let selection = selectionContext.selection;
	let selectionType = selectionContext.selectionType;
	let selectedPane = selectionContext.selectedPane;
	let selectedUdf = selectionContext.selectedUdf;

	/**
	 * Handle chart config changes
	 */
	function handleChartConfigChange(config: ChartConfiguration): void {
		if ($selectedPane) {
			// Update the selection context
			selectionContext.updatePaneConfig(config);

			// Also update the pane config in workspace manager
			workspaceManager.updatePaneConfig($selectedPane.paneId, {
				chartConfig: config as any, // Store the full chart config
			});
		}
	}

	/**
	 * Handle chart data changes (when curve data is loaded)
	 */
	function handleChartDataChange(data: ChartDataFrame | null): void {
		console.log('[ContextToolbar] handleChartDataChange called:', data ? `frame with ${data.length} points` : 'null');
		if ($selectedPane) {
			console.log('[ContextToolbar] Updating pane config for:', $selectedPane.paneId);
			// Store the chart data in the pane config
			workspaceManager.updatePaneConfig($selectedPane.paneId, {
				chartData: data as any, // Store the chart data frame
			});
		} else {
			console.log('[ContextToolbar] No selected pane, cannot update');
		}
	}

	/**
	 * Handle segmented chart data changes (new segment-based architecture)
	 */
	function handleSegmentedDataChange(data: SegmentedCurveData | null): void {
		console.log('[ContextToolbar] handleSegmentedDataChange called:', data ? `${data.segments.length} segments, ${data.total_points} points` : 'null');
		if ($selectedPane) {
			console.log('[ContextToolbar] Updating pane segmented data for:', $selectedPane.paneId);
			// Store the segmented chart data in the pane config
			workspaceManager.updatePaneConfig($selectedPane.paneId, {
				segmentedChartData: data as any, // Store the segmented chart data
			});
		} else {
			console.log('[ContextToolbar] No selected pane, cannot update segmented data');
		}
	}

	/**
	 * Handle correlation curve data changes (for correlation panels)
	 * This stores curve data in a Map keyed by track ID
	 */
	function handleCorrelationCurveDataChange(trackId: string, data: CorrelationCurveData | null): void {
		console.log('[ContextToolbar] handleCorrelationCurveDataChange called:', { trackId, hasData: !!data, points: data?.totalPoints });
		if ($selectedPane) {
			const paneId = $selectedPane.paneId;

			// Get the CURRENT pane config from workspace manager (not the cached selection)
			const currentPane = workspaceManager.getPaneById(paneId);
			if (!currentPane) {
				console.log('[ContextToolbar] Could not find pane in workspace:', paneId);
				return;
			}

			const existingOptions = currentPane.config?.options ?? {};
			const existingMap = existingOptions.correlationCurveData as Map<string, CorrelationCurveData> | undefined;
			const curveDataMap = new Map(existingMap ?? []);

			console.log('[ContextToolbar] Existing map size:', curveDataMap.size, 'keys:', Array.from(curveDataMap.keys()));

			if (data) {
				curveDataMap.set(trackId, data);
				console.log('[ContextToolbar] Added curve data for track:', trackId, 'Map size:', curveDataMap.size);
			} else {
				curveDataMap.delete(trackId);
				console.log('[ContextToolbar] Removed curve data for track:', trackId);
			}

			// Update the pane config with the new map
			workspaceManager.updatePaneConfig(paneId, {
				options: {
					...existingOptions,
					correlationCurveData: curveDataMap
				}
			});

			console.log('[ContextToolbar] Updated pane config, new map size:', curveDataMap.size);
		} else {
			console.log('[ContextToolbar] No selected pane, cannot update correlation data');
		}
	}
</script>

<div class="context-toolbar">
	{#if $selectionType === 'none'}
		<!-- No Selection -->
		<div class="empty-state">
			<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
				<path d="M15 15l-2 5L9 9l11 4-5 2zm0 0l5 5M7.188 2.239l.777 2.897M5.136 7.965l-2.898-.777M13.95 4.05l-2.122 2.122m-5.657 5.656l-2.12 2.122" />
			</svg>
			<h3>Select an Item</h3>
			<p>
				Click on a <strong>chart pane</strong> to configure its settings, or select a <strong>tool</strong> from the toolbox to configure parameters.
			</p>
		</div>
	{:else if $selectionType === 'udf' && $selectedUdf}
		<!-- UDF Selected - Show Parameter Form -->
		<div class="toolbar-section">
			<div class="section-header">
				<span class="section-icon udf-icon">
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
					</svg>
				</span>
				<div class="section-title-group">
					<h3 class="section-title">{$selectedUdf.name}</h3>
					<span class="section-subtitle">{$selectedUdf.category}</span>
				</div>
			</div>
			<div class="section-content">
				<ParameterForm />
			</div>
		</div>
	{:else if $selectionType === 'pane' && $selectedPane}
		<!-- Pane Selected - Show Chart Config -->
		<div class="toolbar-section">
			<div class="section-header">
				<span class="section-icon chart-icon">
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M3 3v18h18M7 16l4-4 4 4 4-8" />
					</svg>
				</span>
				<div class="section-title-group">
					<h3 class="section-title">Chart Settings</h3>
					<span class="section-subtitle">{$selectedPane.paneNode.title}</span>
				</div>
			</div>
			<div class="section-content">
				<ChartConfigPanel
					pane={$selectedPane.paneNode}
					config={$selectedPane.chartConfig ?? null}
					{wells}
					{curves}
					{well}
					{onWellChange}
					onConfigChange={handleChartConfigChange}
					onDataChange={handleChartDataChange}
					onSegmentedDataChange={handleSegmentedDataChange}
					onCorrelationCurveDataChange={handleCorrelationCurveDataChange}
				/>
			</div>
		</div>
	{:else if $selectionType === 'curve'}
		<!-- Curve Selected - Show Curve Info (placeholder for now) -->
		<div class="toolbar-section">
			<div class="section-header">
				<span class="section-icon curve-icon">
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
					</svg>
				</span>
				<div class="section-title-group">
					<h3 class="section-title">Curve Details</h3>
					<span class="section-subtitle">View curve information</span>
				</div>
			</div>
			<div class="section-content">
				<p class="placeholder-text">Curve details panel coming soon...</p>
			</div>
		</div>
	{/if}
</div>

<style>
	.context-toolbar {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--color-bg, #ffffff);
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 32px 24px;
		text-align: center;
		color: var(--color-text-tertiary, #9ca3af);
		height: 100%;
	}

	.empty-state svg {
		margin-bottom: 16px;
		opacity: 0.4;
	}

	.empty-state h3 {
		font-size: 14px;
		font-weight: 600;
		color: var(--color-text-secondary, #6b7280);
		margin: 0 0 8px 0;
	}

	.empty-state p {
		font-size: 13px;
		line-height: 1.5;
		margin: 0;
		max-width: 240px;
	}

	.empty-state strong {
		color: var(--color-text, #111827);
	}

	.toolbar-section {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 16px;
		border-bottom: 1px solid var(--color-border, #e5e7eb);
		background: var(--color-bg-secondary, #f9fafb);
	}

	.section-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border-radius: 8px;
		flex-shrink: 0;
	}

	.udf-icon {
		background: var(--color-primary-light, #eff6ff);
		color: var(--color-primary, #3b82f6);
	}

	.chart-icon {
		background: var(--color-success-light, #f0fdf4);
		color: var(--color-success, #22c55e);
	}

	.curve-icon {
		background: var(--color-warning-light, #fffbeb);
		color: var(--color-warning, #f59e0b);
	}

	.section-title-group {
		flex: 1;
		min-width: 0;
	}

	.section-title {
		font-size: 14px;
		font-weight: 600;
		color: var(--color-text, #111827);
		margin: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.section-subtitle {
		font-size: 12px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.section-content {
		flex: 1;
		overflow-y: auto;
	}

	.placeholder-text {
		padding: 24px;
		font-size: 13px;
		color: var(--color-text-tertiary, #9ca3af);
		text-align: center;
	}
</style>
