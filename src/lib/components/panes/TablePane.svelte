<script lang="ts">
	/**
	 * TablePane - Table view pane for displaying UDF output data
	 *
	 * Uses AG Grid to display curve data in a tabular format with:
	 * - Depth column
	 * - Value column(s)
	 * - Sorting and filtering
	 * - Virtual scrolling for large datasets
	 */
	import type { ColDef } from 'ag-grid-community';
	import DataGrid from '$lib/components/data/DataGrid.svelte';
	import type { CurveDataPoint } from '$lib/types';

	interface TableConfig {
		/** Output curve mnemonic */
		mnemonic?: string;
		/** Output curve unit */
		unit?: string;
		/** Execution ID for reference */
		executionId?: string;
		/** UDF name that generated this output */
		udfName?: string;
		/** The curve data points */
		data?: CurveDataPoint[];
	}

	interface Props {
		/** Configuration for the table */
		config: TableConfig;
		/** Available height for the table */
		height?: number;
	}

	let { config, height = 400 }: Props = $props();

	/** Column definitions for the grid */
	let columnDefs = $derived<ColDef[]>([
		{
			field: 'depth',
			headerName: 'Depth',
			width: 120,
			valueFormatter: (params) => {
				if (params.value === null || params.value === undefined) return '';
				return typeof params.value === 'number' ? params.value.toFixed(2) : String(params.value);
			},
			sort: 'asc'
		},
		{
			field: 'value',
			headerName: config.mnemonic || 'Value',
			flex: 1,
			valueFormatter: (params) => {
				if (params.value === null || params.value === undefined) return 'null';
				return typeof params.value === 'number' ? params.value.toFixed(4) : String(params.value);
			}
		}
	]);

	/** Transform curve data points to row data */
	let rowData = $derived<Record<string, unknown>[]>(
		(config.data || []).map((point, index) => ({
			id: index,
			depth: point.depth,
			value: point.value
		}))
	);

	/** Default column definition */
	const defaultColDef: ColDef = {
		sortable: true,
		filter: true,
		resizable: true
	};
</script>

<div class="table-pane">
	<!-- Header info -->
	{#if config.udfName || config.mnemonic}
		<div class="table-header">
			{#if config.udfName}
				<span class="table-header-udf">{config.udfName}</span>
			{/if}
			{#if config.mnemonic}
				<span class="table-header-mnemonic">
					{config.mnemonic}
					{#if config.unit}
						<span class="table-header-unit">({config.unit})</span>
					{/if}
				</span>
			{/if}
			{#if config.data}
				<span class="table-header-count">{config.data.length} rows</span>
			{/if}
		</div>
	{/if}

	<!-- Data grid -->
	<div class="table-content" style="height: {height - 32}px;">
		{#if rowData.length > 0}
			<DataGrid
				{columnDefs}
				{rowData}
				{defaultColDef}
				height="100%"
				autoSizeColumns={true}
				getRowId={(data) => String(data.id)}
			/>
		{:else}
			<div class="table-empty">
				<div class="table-empty-icon">
					<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M3 10h18M3 14h18M3 6h18M3 18h18" stroke-linecap="round" />
						<rect x="3" y="3" width="18" height="18" rx="2" />
					</svg>
				</div>
				<p class="table-empty-text">No data to display</p>
				<p class="table-empty-hint">Execute a UDF to see output data here</p>
			</div>
		{/if}
	</div>
</div>

<style>
	.table-pane {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		overflow: hidden;
	}

	.table-header {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 6px 12px;
		background: hsl(var(--muted));
		border-bottom: 1px solid hsl(var(--border));
		font-size: 12px;
		flex-shrink: 0;
	}

	.table-header-udf {
		font-weight: 600;
		color: hsl(var(--foreground));
	}

	.table-header-mnemonic {
		color: hsl(var(--primary));
		font-weight: 500;
	}

	.table-header-unit {
		color: hsl(var(--muted-foreground));
		font-weight: 400;
	}

	.table-header-count {
		margin-left: auto;
		color: hsl(var(--muted-foreground));
	}

	.table-content {
		flex: 1;
		overflow: hidden;
	}

	.table-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		padding: 24px;
		text-align: center;
		color: hsl(var(--muted-foreground));
	}

	.table-empty-icon {
		margin-bottom: 12px;
		opacity: 0.5;
	}

	.table-empty-text {
		font-size: 14px;
		font-weight: 500;
		margin: 0 0 4px 0;
	}

	.table-empty-hint {
		font-size: 12px;
		margin: 0;
		opacity: 0.7;
	}
</style>
