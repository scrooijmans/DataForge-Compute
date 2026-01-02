<script lang="ts">
	/**
	 * ContextToolbar - Context-sensitive toolbar panel
	 *
	 * Displays different content based on what is currently selected:
	 * - UDF selected: Shows UDF parameters and execution controls
	 * - Chart pane selected: Shows hint to use settings dialog
	 * - Nothing selected: Shows placeholder with instructions
	 *
	 * Note: Chart configuration has moved to ChartSettingsDialog (TradingView-style).
	 */
	import type { CurveInfo, WellInfo } from '$lib/types';
	import { selectionContext } from '$lib/panes/selection-context';
	import ParameterForm from '$lib/components/ParameterForm.svelte';
	import { PaneType } from '$lib/panes/layout-model';

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
	let selectionType = selectionContext.selectionType;
	let selectedPane = selectionContext.selectedPane;
	let selectedUdf = selectionContext.selectedUdf;

	/** Get chart type name for display */
	function getChartTypeName(paneType: PaneType | undefined): string {
		switch (paneType) {
			case PaneType.LineChart:
				return 'Line Chart';
			case PaneType.ScatterChart:
				return 'Scatter Chart';
			case PaneType.Histogram:
				return 'Histogram';
			case PaneType.CrossPlot:
				return 'Cross Plot';
			case PaneType.WellLog:
				return 'Well Log';
			case PaneType.LinkedCharts:
				return 'Linked Charts';
			case PaneType.Correlation:
				return 'Correlation';
			case PaneType.DataGrid:
				return 'Data Grid';
			default:
				return 'Chart';
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
		<!-- Pane Selected - Show hint to use settings dialog -->
		<div class="toolbar-section">
			<div class="section-header">
				<span class="section-icon chart-icon">
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M3 3v18h18M7 16l4-4 4 4 4-8" />
					</svg>
				</span>
				<div class="section-title-group">
					<h3 class="section-title">{getChartTypeName($selectedPane.paneNode.paneType)}</h3>
					<span class="section-subtitle">{$selectedPane.paneNode.title}</span>
				</div>
			</div>
			<div class="section-content">
				<div class="settings-hint">
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M12 15.5A3.5 3.5 0 0 1 8.5 12A3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5a3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97c0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1c0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66Z" />
					</svg>
					<p>
						Use the <strong>Settings</strong> button in the toolbar above to configure this chart.
					</p>
				</div>
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

	.settings-hint {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 32px 24px;
		text-align: center;
		color: var(--color-text-tertiary, #9ca3af);
		height: 100%;
	}

	.settings-hint svg {
		margin-bottom: 16px;
		opacity: 0.4;
	}

	.settings-hint p {
		font-size: 13px;
		line-height: 1.5;
		margin: 0;
		max-width: 200px;
	}

	.settings-hint strong {
		color: var(--color-text, #111827);
	}

	.placeholder-text {
		padding: 24px;
		font-size: 13px;
		color: var(--color-text-tertiary, #9ca3af);
		text-align: center;
	}
</style>
