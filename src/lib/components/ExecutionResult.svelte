<script lang="ts">
	/**
	 * Execution Result - Display UDF execution output with Table and Chart views
	 */
	import { browser } from '$app/environment';
	import { executionResult, selectedUdf, curveData } from '$lib/stores/compute';
	import CurveChart from '$lib/components/charts/CurveChart.svelte';
	import type { ColDef } from 'ag-grid-community';

	type ViewTab = 'table' | 'chart';
	let activeTab = $state<ViewTab>('table');

	// DataGrid state
	let DataGrid: typeof import('./data/DataGrid.svelte').default | null = $state(null);
	let gridColumnDefs = $state<ColDef[]>([]);
	let gridRowData = $state<Record<string, unknown>[]>([]);

	// Load DataGrid dynamically to avoid SSR issues
	$effect(() => {
		if (browser && $executionResult?.success && $executionResult.output_data) {
			import('./data/DataGrid.svelte').then((module) => {
				DataGrid = module.default;
			});
		}
	});

	// Prepare grid data when execution result changes
	$effect(() => {
		if ($executionResult?.success && $executionResult.output_data) {
			const outputMnemonic = $executionResult.output_mnemonic || 'Output';

			// Column definitions: Depth, Original (if available), Output
			const cols: ColDef[] = [
				{
					field: 'depth',
					headerName: 'Depth',
					valueFormatter: (params: { value: number }) =>
						params.value !== null ? params.value.toFixed(2) : '',
					width: 100
				}
			];

			// Add original curve column if we have input data
			if ($curveData && $curveData.data.length > 0) {
				cols.push({
					field: 'original',
					headerName: $curveData.mnemonic || 'Original',
					valueFormatter: (params: { value: number | null }) =>
						params.value !== null ? params.value.toFixed(4) : 'null',
					cellClass: 'text-[hsl(var(--muted-foreground))]',
					width: 120
				});
			}

			// Add output column
			cols.push({
				field: 'output',
				headerName: outputMnemonic,
				valueFormatter: (params: { value: number | null }) =>
					params.value !== null ? params.value.toFixed(4) : 'null',
				cellClass: 'text-green-600 dark:text-green-400',
				width: 120
			});

			gridColumnDefs = cols;

			// Build row data
			gridRowData = $executionResult.output_data.map((point, i) => ({
				depth: point.depth,
				original: $curveData?.data[i]?.value ?? null,
				output: point.value
			}));
		}
	});
</script>

{#if $executionResult}
	<div class="rounded-lg border bg-[hsl(var(--card))]">
		<!-- Header -->
		<div class="flex items-center justify-between border-b p-3">
			<div>
				<h3 class="text-sm font-semibold">
					{#if $executionResult.success}
						Execution Result
					{:else}
						Execution Failed
					{/if}
				</h3>
				{#if $executionResult.output_mnemonic}
					<p class="text-xs text-[hsl(var(--muted-foreground))]">
						Output: {$executionResult.output_mnemonic}
					</p>
				{/if}
			</div>
			<div class="flex items-center gap-2">
				{#if $executionResult.success}
					<span class="flex items-center gap-1 text-xs text-green-600 dark:text-green-400">
						<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M5 13l4 4L19 7"
							/>
						</svg>
						Success
					</span>
				{:else}
					<span class="flex items-center gap-1 text-xs text-red-600 dark:text-red-400">
						<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							/>
						</svg>
						Failed
					</span>
				{/if}
			</div>
		</div>

		<!-- Warnings -->
		{#if $executionResult.warnings.length > 0}
			<div class="border-b bg-yellow-50 p-3 dark:bg-yellow-900/20">
				<div class="flex items-start gap-2">
					<svg
						class="h-4 w-4 mt-0.5 text-yellow-600 dark:text-yellow-400"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
						/>
					</svg>
					<div>
						<p class="text-xs font-medium text-yellow-800 dark:text-yellow-200">Warnings</p>
						<ul class="mt-1 text-xs text-yellow-700 dark:text-yellow-300">
							{#each $executionResult.warnings as warning}
								<li>{warning}</li>
							{/each}
						</ul>
					</div>
				</div>
			</div>
		{/if}

		<!-- Error -->
		{#if $executionResult.error}
			<div class="border-b bg-red-50 p-3 dark:bg-red-900/20">
				<div class="flex items-start gap-2">
					<svg
						class="h-4 w-4 mt-0.5 text-red-600 dark:text-red-400"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
						/>
					</svg>
					<div>
						<p class="text-xs font-medium text-red-800 dark:text-red-200">Error</p>
						<p class="mt-1 text-xs text-red-700 dark:text-red-300">
							{$executionResult.error}
						</p>
					</div>
				</div>
			</div>
		{/if}

		<!-- Output Data with Tabs -->
		{#if $executionResult.success && $executionResult.output_data}
			<!-- Tab Navigation -->
			<div class="flex items-center gap-1 border-b px-3 pt-2">
				<button
					onclick={() => (activeTab = 'table')}
					class="flex items-center gap-1.5 rounded-t-md px-3 py-1.5 text-xs font-medium transition-colors {activeTab === 'table'
						? 'bg-[hsl(var(--muted))] text-[hsl(var(--foreground))]'
						: 'text-[hsl(var(--muted-foreground))] hover:text-[hsl(var(--foreground))]'}"
				>
					<svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M3 10h18M3 14h18m-9-4v8m-7 0h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
						/>
					</svg>
					Table
				</button>
				<button
					onclick={() => (activeTab = 'chart')}
					class="flex items-center gap-1.5 rounded-t-md px-3 py-1.5 text-xs font-medium transition-colors {activeTab === 'chart'
						? 'bg-[hsl(var(--muted))] text-[hsl(var(--foreground))]'
						: 'text-[hsl(var(--muted-foreground))] hover:text-[hsl(var(--foreground))]'}"
				>
					<svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M7 12l3-3 3 3 4-4M8 21l4-4 4 4M3 4h18M4 4h16v12a1 1 0 01-1 1H5a1 1 0 01-1-1V4z"
						/>
					</svg>
					Chart
				</button>
				<div class="ml-auto text-xs text-[hsl(var(--muted-foreground))]">
					{$executionResult.output_data.length.toLocaleString()} points
					{#if $executionResult.saved}
						<span class="ml-2 text-green-600 dark:text-green-400">Saved</span>
					{/if}
				</div>
			</div>

			<!-- Tab Content -->
			<div class="p-3">
				{#if activeTab === 'table'}
					<!-- Table View with AG Grid -->
					{#if DataGrid && gridRowData.length > 0}
						<div class="h-64 overflow-hidden rounded border">
							<DataGrid
								columnDefs={gridColumnDefs}
								rowData={gridRowData}
								height="100%"
								pagination={false}
								autoSizeColumns={true}
							/>
						</div>
					{:else}
						<!-- Fallback simple table while DataGrid loads -->
						<div class="max-h-64 overflow-y-auto rounded border bg-[hsl(var(--muted))]">
							<table class="w-full font-mono text-xs">
								<thead class="sticky top-0 bg-[hsl(var(--muted))]">
									<tr class="border-b border-[hsl(var(--border))]">
										<th class="px-2 py-1 text-left">Depth</th>
										{#if $curveData}
											<th class="px-2 py-1 text-right">Original</th>
										{/if}
										<th class="px-2 py-1 text-right text-green-600 dark:text-green-400">Output</th>
									</tr>
								</thead>
								<tbody>
									{#each $executionResult.output_data.slice(0, 50) as point, i}
										<tr class="border-b border-[hsl(var(--border))] last:border-0">
											<td class="px-2 py-1">{point.depth.toFixed(2)}</td>
											{#if $curveData && $curveData.data[i]}
												<td class="px-2 py-1 text-right">
													{$curveData.data[i].value !== null
														? $curveData.data[i].value?.toFixed(4)
														: 'null'}
												</td>
											{/if}
											<td class="px-2 py-1 text-right text-green-600 dark:text-green-400">
												{point.value !== null ? point.value.toFixed(4) : 'null'}
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
						{#if $executionResult.output_data.length > 50}
							<p class="mt-2 text-center text-xs text-[hsl(var(--muted-foreground))]">
								Showing first 50 of {$executionResult.output_data.length.toLocaleString()} points (loading full grid...)
							</p>
						{/if}
					{/if}
				{:else if activeTab === 'chart'}
					<!-- Chart View -->
					<div class="flex justify-center overflow-x-auto">
						<CurveChart
							outputData={$executionResult.output_data}
							outputMnemonic={$executionResult.output_mnemonic || 'Output'}
							inputData={$curveData?.data}
							inputMnemonic={$curveData?.mnemonic}
							height={350}
						/>
					</div>
				{/if}
			</div>

			<!-- Provenance Info -->
			<div class="border-t p-3">
				<div class="text-xs text-[hsl(var(--muted-foreground))]">
					<div class="flex justify-between">
						<span>Execution ID:</span>
						<span class="font-mono">{$executionResult.execution_id.slice(0, 8)}...</span>
					</div>
					{#if $selectedUdf}
						<div class="flex justify-between mt-1">
							<span>Tool:</span>
							<span>{$selectedUdf.full_id}</span>
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</div>
{/if}
