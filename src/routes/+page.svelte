<script lang="ts">
	/**
	 * DataForge Compute - Main Page
	 *
	 * Demonstrates reading data from DataForge's shared database
	 * and running computations (moving average) on curve data.
	 */
	import { invoke } from '@tauri-apps/api/core';

	// Types matching Rust structs
	interface DataForgeStatus {
		connected: boolean;
		data_dir: string | null;
		db_exists: boolean;
		error: string | null;
	}

	interface WorkspaceInfo {
		id: string;
		name: string;
		created_at: string;
	}

	interface WellInfo {
		id: string;
		name: string;
		uwi: string | null;
		field: string | null;
		curve_count: number;
	}

	interface CurveInfo {
		id: string;
		mnemonic: string;
		unit: string | null;
		description: string | null;
		min_depth: number | null;
		max_depth: number | null;
		row_count: number;
	}

	interface CurveDataPoint {
		depth: number;
		value: number | null;
	}

	interface CurveData {
		curve_id: string;
		mnemonic: string;
		unit: string | null;
		data: CurveDataPoint[];
	}

	interface MovingAverageResult {
		input_curve: string;
		window_size: number;
		data: CurveDataPoint[];
	}

	// State
	let status = $state<DataForgeStatus | null>(null);
	let workspaces = $state<WorkspaceInfo[]>([]);
	let wells = $state<WellInfo[]>([]);
	let curves = $state<CurveInfo[]>([]);
	let curveData = $state<CurveData | null>(null);
	let movingAvgResult = $state<MovingAverageResult | null>(null);

	let selectedWorkspaceId = $state<string | null>(null);
	let selectedWellId = $state<string | null>(null);
	let selectedCurveId = $state<string | null>(null);
	let windowSize = $state(5);

	let isLoading = $state(false);
	let error = $state<string | null>(null);

	// Load status on mount
	loadStatus();

	async function loadStatus() {
		try {
			status = await invoke<DataForgeStatus>('get_dataforge_status');
			if (status.connected) {
				await loadWorkspaces();
			}
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function loadWorkspaces() {
		try {
			workspaces = await invoke<WorkspaceInfo[]>('list_workspaces');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function selectWorkspace(id: string) {
		selectedWorkspaceId = id;
		selectedWellId = null;
		selectedCurveId = null;
		wells = [];
		curves = [];
		curveData = null;
		movingAvgResult = null;

		try {
			wells = await invoke<WellInfo[]>('list_wells', { workspaceId: id });
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function selectWell(id: string) {
		selectedWellId = id;
		selectedCurveId = null;
		curves = [];
		curveData = null;
		movingAvgResult = null;

		try {
			curves = await invoke<CurveInfo[]>('list_curves', { wellId: id });
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function selectCurve(id: string) {
		selectedCurveId = id;
		curveData = null;
		movingAvgResult = null;
		isLoading = true;

		try {
			curveData = await invoke<CurveData>('get_curve_data', { curveId: id });
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			isLoading = false;
		}
	}

	async function runMovingAverage() {
		if (!selectedCurveId) return;
		isLoading = true;
		error = null;

		try {
			movingAvgResult = await invoke<MovingAverageResult>('compute_moving_average', {
				curveId: selectedCurveId,
				windowSize
			});
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			isLoading = false;
		}
	}

	// Derived values for display
	let selectedWorkspace = $derived(workspaces.find((w) => w.id === selectedWorkspaceId));
	let selectedWell = $derived(wells.find((w) => w.id === selectedWellId));
	let selectedCurve = $derived(curves.find((c) => c.id === selectedCurveId));
</script>

<div class="mx-auto max-w-6xl p-6">
	<!-- Header -->
	<header class="mb-8">
		<h1 class="text-3xl font-bold">DataForge Compute</h1>
		<p class="text-[hsl(var(--muted-foreground))]">
			Read data from DataForge and run computations
		</p>
	</header>

	<!-- Connection Status -->
	<div class="mb-6 rounded-lg border bg-[hsl(var(--card))] p-4">
		<h2 class="mb-2 font-semibold">DataForge Connection</h2>
		{#if status === null}
			<p class="text-[hsl(var(--muted-foreground))]">Checking connection...</p>
		{:else if status.connected}
			<div class="flex items-center gap-2">
				<span class="h-3 w-3 rounded-full bg-green-500"></span>
				<span class="text-green-600 dark:text-green-400">Connected</span>
			</div>
			<p class="mt-1 text-sm text-[hsl(var(--muted-foreground))]">{status.data_dir}</p>
		{:else}
			<div class="flex items-center gap-2">
				<span class="h-3 w-3 rounded-full bg-red-500"></span>
				<span class="text-red-600 dark:text-red-400">Not Connected</span>
			</div>
			{#if status.error}
				<p class="mt-1 text-sm text-red-600 dark:text-red-400">{status.error}</p>
			{/if}
			{#if status.data_dir}
				<p class="mt-1 text-sm text-[hsl(var(--muted-foreground))]">
					Expected location: {status.data_dir}
				</p>
			{/if}
			<p class="mt-2 text-sm">
				Please ensure DataForge is installed and has been run at least once.
			</p>
		{/if}
	</div>

	{#if error}
		<div class="mb-6 rounded-lg border border-red-300 bg-red-50 p-4 dark:border-red-800 dark:bg-red-900/20">
			<p class="text-red-600 dark:text-red-400">{error}</p>
			<button
				onclick={() => (error = null)}
				class="mt-2 text-sm underline hover:no-underline"
			>
				Dismiss
			</button>
		</div>
	{/if}

	{#if status?.connected}
		<div class="grid gap-6 lg:grid-cols-3">
			<!-- Left Column: Selection -->
			<div class="space-y-4 lg:col-span-1">
				<!-- Workspace Selection -->
				<div class="rounded-lg border bg-[hsl(var(--card))] p-4">
					<h3 class="mb-3 font-semibold">1. Select Workspace</h3>
					{#if workspaces.length === 0}
						<p class="text-sm text-[hsl(var(--muted-foreground))]">No workspaces found</p>
					{:else}
						<div class="space-y-1">
							{#each workspaces as workspace (workspace.id)}
								<button
									onclick={() => selectWorkspace(workspace.id)}
									class="w-full rounded-md px-3 py-2 text-left text-sm transition-colors {selectedWorkspaceId ===
									workspace.id
										? 'bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))]'
										: 'hover:bg-[hsl(var(--secondary))]'}"
								>
									{workspace.name}
								</button>
							{/each}
						</div>
					{/if}
				</div>

				<!-- Well Selection -->
				{#if selectedWorkspaceId}
					<div class="rounded-lg border bg-[hsl(var(--card))] p-4">
						<h3 class="mb-3 font-semibold">2. Select Well</h3>
						{#if wells.length === 0}
							<p class="text-sm text-[hsl(var(--muted-foreground))]">No wells in this workspace</p>
						{:else}
							<div class="max-h-48 space-y-1 overflow-y-auto">
								{#each wells as well (well.id)}
									<button
										onclick={() => selectWell(well.id)}
										class="w-full rounded-md px-3 py-2 text-left text-sm transition-colors {selectedWellId ===
										well.id
											? 'bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))]'
											: 'hover:bg-[hsl(var(--secondary))]'}"
									>
										<div class="font-medium">{well.name}</div>
										<div class="text-xs opacity-70">{well.curve_count} curves</div>
									</button>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

				<!-- Curve Selection -->
				{#if selectedWellId}
					<div class="rounded-lg border bg-[hsl(var(--card))] p-4">
						<h3 class="mb-3 font-semibold">3. Select Curve</h3>
						{#if curves.length === 0}
							<p class="text-sm text-[hsl(var(--muted-foreground))]">No curves in this well</p>
						{:else}
							<div class="max-h-48 space-y-1 overflow-y-auto">
								{#each curves as curve (curve.id)}
									<button
										onclick={() => selectCurve(curve.id)}
										class="w-full rounded-md px-3 py-2 text-left text-sm transition-colors {selectedCurveId ===
										curve.id
											? 'bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))]'
											: 'hover:bg-[hsl(var(--secondary))]'}"
									>
										<div class="font-medium">{curve.mnemonic}</div>
										<div class="text-xs opacity-70">
											{curve.row_count} points
											{#if curve.unit}
												| {curve.unit}
											{/if}
										</div>
									</button>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Right Column: Data & Computation -->
			<div class="space-y-4 lg:col-span-2">
				<!-- Curve Info -->
				{#if selectedCurve && curveData}
					<div class="rounded-lg border bg-[hsl(var(--card))] p-4">
						<h3 class="mb-3 font-semibold">Curve Data: {curveData.mnemonic}</h3>
						<div class="mb-4 grid grid-cols-3 gap-4 text-sm">
							<div>
								<div class="text-[hsl(var(--muted-foreground))]">Points</div>
								<div class="font-medium">{curveData.data.length.toLocaleString()}</div>
							</div>
							<div>
								<div class="text-[hsl(var(--muted-foreground))]">Unit</div>
								<div class="font-medium">{curveData.unit || 'N/A'}</div>
							</div>
							<div>
								<div class="text-[hsl(var(--muted-foreground))]">Depth Range</div>
								<div class="font-medium">
									{#if curveData.data.length > 0}
										{curveData.data[0].depth.toFixed(1)} - {curveData.data[curveData.data.length - 1].depth.toFixed(1)}
									{:else}
										N/A
									{/if}
								</div>
							</div>
						</div>

						<!-- Data Preview -->
						<div class="rounded border bg-[hsl(var(--muted))] p-3">
							<div class="mb-2 text-xs font-medium text-[hsl(var(--muted-foreground))]">
								Data Preview (first 10 points)
							</div>
							<div class="max-h-40 overflow-y-auto font-mono text-xs">
								<table class="w-full">
									<thead>
										<tr class="border-b border-[hsl(var(--border))]">
											<th class="px-2 py-1 text-left">Depth</th>
											<th class="px-2 py-1 text-right">Value</th>
										</tr>
									</thead>
									<tbody>
										{#each curveData.data.slice(0, 10) as point}
											<tr class="border-b border-[hsl(var(--border))] last:border-0">
												<td class="px-2 py-1">{point.depth.toFixed(2)}</td>
												<td class="px-2 py-1 text-right">
													{point.value !== null ? point.value.toFixed(4) : 'null'}
												</td>
											</tr>
										{/each}
									</tbody>
								</table>
							</div>
						</div>
					</div>

					<!-- Moving Average Computation -->
					<div class="rounded-lg border bg-[hsl(var(--card))] p-4">
						<h3 class="mb-3 font-semibold">Computation: Moving Average</h3>
						<div class="flex items-end gap-4">
							<div>
								<label for="window-size" class="mb-1 block text-sm text-[hsl(var(--muted-foreground))]">
									Window Size
								</label>
								<input
									id="window-size"
									type="number"
									bind:value={windowSize}
									min="1"
									max="100"
									class="w-24 rounded-md border bg-[hsl(var(--background))] px-3 py-2 text-sm"
								/>
							</div>
							<button
								onclick={runMovingAverage}
								disabled={isLoading}
								class="rounded-md bg-[hsl(var(--primary))] px-4 py-2 text-sm font-medium text-[hsl(var(--primary-foreground))] transition-colors hover:opacity-90 disabled:opacity-50"
							>
								{isLoading ? 'Computing...' : 'Run Computation'}
							</button>
						</div>

						{#if movingAvgResult}
							<div class="mt-4 rounded border bg-[hsl(var(--muted))] p-3">
								<div class="mb-2 text-xs font-medium text-[hsl(var(--muted-foreground))]">
									Result: Moving Average (window={movingAvgResult.window_size})
								</div>
								<div class="max-h-40 overflow-y-auto font-mono text-xs">
									<table class="w-full">
										<thead>
											<tr class="border-b border-[hsl(var(--border))]">
												<th class="px-2 py-1 text-left">Depth</th>
												<th class="px-2 py-1 text-right">Original</th>
												<th class="px-2 py-1 text-right">Smoothed</th>
											</tr>
										</thead>
										<tbody>
											{#each movingAvgResult.data.slice(0, 10) as point, i}
												<tr class="border-b border-[hsl(var(--border))] last:border-0">
													<td class="px-2 py-1">{point.depth.toFixed(2)}</td>
													<td class="px-2 py-1 text-right">
														{curveData.data[i]?.value !== null
															? curveData.data[i].value?.toFixed(4)
															: 'null'}
													</td>
													<td class="px-2 py-1 text-right text-green-600 dark:text-green-400">
														{point.value !== null ? point.value.toFixed(4) : 'null'}
													</td>
												</tr>
											{/each}
										</tbody>
									</table>
								</div>
								<p class="mt-2 text-xs text-[hsl(var(--muted-foreground))]">
									Showing first 10 of {movingAvgResult.data.length.toLocaleString()} smoothed points
								</p>
							</div>
						{/if}
					</div>
				{:else if isLoading}
					<div class="flex items-center justify-center rounded-lg border bg-[hsl(var(--card))] p-8">
						<svg
							class="h-8 w-8 animate-spin text-[hsl(var(--muted-foreground))]"
							fill="none"
							viewBox="0 0 24 24"
						>
							<circle
								class="opacity-25"
								cx="12"
								cy="12"
								r="10"
								stroke="currentColor"
								stroke-width="4"
							></circle>
							<path
								class="opacity-75"
								fill="currentColor"
								d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
							></path>
						</svg>
					</div>
				{:else}
					<div class="flex flex-col items-center justify-center rounded-lg border bg-[hsl(var(--card))] p-8 text-center">
						<svg
							class="mb-4 h-12 w-12 text-[hsl(var(--muted-foreground))] opacity-50"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="1.5"
								d="M7 12l3-3 3 3 4-4M8 21l4-4 4 4M3 4h18M4 4v16"
							/>
						</svg>
						<h3 class="font-medium">Select a Curve</h3>
						<p class="mt-1 text-sm text-[hsl(var(--muted-foreground))]">
							Choose a workspace, well, and curve to view data and run computations
						</p>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
