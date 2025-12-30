<script lang="ts">
	/**
	 * ChartToolbar - Toolbar for adding/managing charts
	 *
	 * Provides buttons to add different chart types to the visualization area.
	 * Part of the high-performance charting system.
	 */

	interface Props {
		onAddChart?: (type: ChartType) => void;
		disabled?: boolean;
	}

	type ChartType = 'line' | 'scatter' | 'histogram' | 'crossplot' | 'welllog';

	let { onAddChart, disabled = false }: Props = $props();

	const chartTypes: { type: ChartType; label: string; icon: string; description: string }[] = [
		{
			type: 'welllog',
			label: 'Well Log',
			icon: 'M4 2v20M8 2v20M4 6h4M4 10h4M4 14h4M4 18h4M12 2c0 4 4 4 4 8s-4 4-4 8 4 4 4 8',
			description: 'Single curve well log (Depth on Y-axis)'
		},
		{
			type: 'line',
			label: 'Line Chart',
			icon: 'M3 17L9 11L13 15L21 7',
			description: 'Depth vs Value line plot'
		},
		{
			type: 'scatter',
			label: 'Scatter Plot',
			icon: 'M12 12m-1 0a1 1 0 102 0a1 1 0 10-2 0M6 8m-1 0a1 1 0 102 0a1 1 0 10-2 0M18 16m-1 0a1 1 0 102 0a1 1 0 10-2 0M9 15m-1 0a1 1 0 102 0a1 1 0 10-2 0M15 9m-1 0a1 1 0 102 0a1 1 0 10-2 0',
			description: 'X-Y scatter crossplot'
		},
		{
			type: 'histogram',
			label: 'Histogram',
			icon: 'M3 21V8h4v13H3zM10 21V3h4v18h-4zM17 21v-9h4v9h-4z',
			description: 'Value distribution'
		},
		{
			type: 'crossplot',
			label: 'Crossplot',
			icon: 'M3 21L21 3M3 3v18h18',
			description: 'Two-curve crossplot'
		}
	];

	function handleAddChart(type: ChartType) {
		if (!disabled && onAddChart) {
			onAddChart(type);
		}
	}
</script>

<div class="chart-toolbar">
	<span class="toolbar-label">Add Chart:</span>

	<div class="toolbar-buttons">
		{#each chartTypes as chart (chart.type)}
			<button
				type="button"
				class="toolbar-btn"
				title={chart.description}
				disabled={disabled}
				onclick={() => handleAddChart(chart.type)}
			>
				<svg
					class="toolbar-icon"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d={chart.icon} />
				</svg>
				<span class="btn-label">{chart.label}</span>
			</button>
		{/each}
	</div>
</div>

<style>
	.chart-toolbar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.5rem 0.75rem;
		background: hsl(var(--muted));
		border-radius: 6px;
	}

	.toolbar-label {
		font-size: 0.75rem;
		font-weight: 500;
		color: hsl(var(--muted-foreground));
		white-space: nowrap;
	}

	.toolbar-buttons {
		display: flex;
		gap: 0.25rem;
	}

	.toolbar-btn {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.375rem 0.625rem;
		font-size: 0.75rem;
		font-weight: 500;
		color: hsl(var(--foreground));
		background: hsl(var(--background));
		border: 1px solid hsl(var(--border));
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.15s ease;
	}

	.toolbar-btn:hover:not(:disabled) {
		background: hsl(var(--accent));
		border-color: hsl(var(--accent));
	}

	.toolbar-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.toolbar-icon {
		width: 14px;
		height: 14px;
		flex-shrink: 0;
	}

	.btn-label {
		white-space: nowrap;
	}

	/* Responsive: hide labels on smaller screens */
	@media (max-width: 768px) {
		.btn-label {
			display: none;
		}

		.toolbar-btn {
			padding: 0.5rem;
		}

		.toolbar-icon {
			width: 16px;
			height: 16px;
		}
	}
</style>
