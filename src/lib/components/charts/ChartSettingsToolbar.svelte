<script lang="ts">
	/**
	 * ChartSettingsToolbar - Top toolbar with chart info and settings gear
	 *
	 * Displays:
	 * - Chart type label
	 * - Selected chart pane title
	 * - Settings gear button that opens the ChartSettingsDialog
	 *
	 * Inspired by TradingView's top settings bar.
	 */

	import { selectionContext } from '$lib/panes/selection-context';
	import { PaneType } from '$lib/panes/layout-model';

	interface Props {
		/** Callback when settings button is clicked */
		onOpenSettings: () => void;
	}

	let { onOpenSettings }: Props = $props();

	/** Selection stores */
	let selectionType = selectionContext.selectionType;
	let selectedPane = selectionContext.selectedPane;

	/** Whether a chart pane is currently selected */
	let hasSelectedChart = $derived($selectionType === 'pane' && $selectedPane !== null);

	/** Get the chart type name for display */
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
			case PaneType.Table:
				return 'Table';
			default:
				return 'Chart';
		}
	}

	/** Chart type name */
	let chartTypeName = $derived(
		hasSelectedChart ? getChartTypeName($selectedPane?.paneNode.paneType) : 'No Chart Selected'
	);

	/** Chart title */
	let chartTitle = $derived(hasSelectedChart ? ($selectedPane?.paneNode.title ?? '') : '');
</script>

<div class="chart-settings-toolbar">
	<div class="toolbar-info">
		<span class="chart-type">{chartTypeName}</span>
		{#if chartTitle}
			<span class="separator">|</span>
			<span class="chart-title">{chartTitle}</span>
		{/if}
	</div>

	<button
		class="settings-button"
		class:disabled={!hasSelectedChart}
		onclick={onOpenSettings}
		disabled={!hasSelectedChart}
		title={hasSelectedChart ? 'Open chart settings' : 'Select a chart to configure'}
		aria-label="Open chart settings"
	>
		<svg viewBox="0 0 24 24" class="settings-icon" aria-hidden="true">
			<path
				fill="currentColor"
				d="M12 15.5A3.5 3.5 0 0 1 8.5 12A3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5a3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97c0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1c0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66Z"
			/>
		</svg>
		<span class="settings-label">Settings</span>
	</button>
</div>

<style>
	.chart-settings-toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 6px 12px;
		background: hsl(var(--muted));
		border-bottom: 1px solid hsl(var(--border));
		min-height: 36px;
	}

	.toolbar-info {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		flex: 1;
	}

	.chart-type {
		font-size: 11px;
		font-weight: 600;
		color: hsl(var(--muted-foreground));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}

	.separator {
		color: hsl(var(--border));
	}

	.chart-title {
		font-size: 13px;
		font-weight: 500;
		color: hsl(var(--foreground));
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.settings-button {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		border: 1px solid hsl(var(--border));
		border-radius: 6px;
		background: hsl(var(--background));
		color: hsl(var(--foreground));
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease,
			color 0.15s ease;
	}

	.settings-button:hover:not(:disabled) {
		background: hsl(var(--accent));
		border-color: hsl(var(--accent));
	}

	.settings-button.disabled,
	.settings-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.settings-button:focus-visible {
		outline: 2px solid hsl(var(--ring));
		outline-offset: 2px;
	}

	.settings-icon {
		width: 16px;
		height: 16px;
	}

	.settings-label {
		white-space: nowrap;
	}
</style>
