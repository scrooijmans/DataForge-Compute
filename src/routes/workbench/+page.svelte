<script lang="ts">
	/**
	 * DataForge Compute Workbench - Main Page
	 *
	 * Four-panel layout:
	 * - Left: Sidebar with workspace switcher + UDF Toolbox
	 * - Center-Top: Data selection bar
	 * - Center: Execution results + Views (tabbed)
	 * - Right: Parameter configuration + Artifact Inspector
	 */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import UdfToolbox from '$lib/components/UdfToolbox.svelte';
	import ParameterForm from '$lib/components/ParameterForm.svelte';
	import ExecutionResult from '$lib/components/ExecutionResult.svelte';
	import ArtifactInspector from '$lib/components/ArtifactInspector.svelte';
	import CoverageView from '$lib/components/CoverageView.svelte';
	import SimplePlot from '$lib/components/SimplePlot.svelte';
	import ProvenancePanel from '$lib/components/ProvenancePanel.svelte';
	import WorkspaceSwitcher from '$lib/components/layout/WorkspaceSwitcher.svelte';
	import {
		status,
		wells,
		curves,
		curveData,
		selectedWorkspaceId,
		selectedWorkspace,
		selectedWellId,
		selectedCurveId,
		selectedCurve,
		selectedUdf,
		error,
		loadStatus,
		selectWell,
		selectCurve,
		clearError
	} from '$lib/stores/compute';

	// View tab state
	let activeViewTab = $state<'results' | 'coverage' | 'plot'>('results');

	// Right panel tab state
	let activeInspectorTab = $state<'artifact' | 'provenance'>('artifact');

	// Curves with data for coverage view
	let curvesWithData = $derived.by(() => {
		return $curves.map((info) => ({
			info,
			data: info.id === $selectedCurveId ? $curveData : null
		}));
	});

	// Plot curves
	let plotCurves = $derived.by(() => {
		if (!$curveData) return [];
		return [
			{
				data: $curveData,
				color: '#3b82f6',
				label: $curveData.mnemonic
			}
		];
	});

	onMount(async () => {
		await loadStatus();

		// Redirect to workspace selection if no workspace is selected
		if (!$selectedWorkspaceId) {
			goto('/workspace');
		}
	});

	// Watch for workspace deselection
	$effect(() => {
		if ($status?.connected && !$selectedWorkspaceId) {
			goto('/workspace');
		}
	});
</script>

<div class="flex h-screen flex-col">
	<!-- Header -->
	<header class="border-b bg-[hsl(var(--card))] px-4 py-3">
		<div class="flex items-center justify-between">
			<div>
				<h1 class="text-xl font-bold">DataForge Compute</h1>
				<p class="text-sm text-[hsl(var(--muted-foreground))]">
					{#if $selectedWorkspace}
						{$selectedWorkspace.name}
					{:else}
						Run computations on well log data
					{/if}
				</p>
			</div>
			<div class="flex items-center gap-4">
				<!-- View tabs -->
				{#if $status?.connected && $selectedWellId}
					<div class="flex rounded-lg border bg-[hsl(var(--muted))] p-0.5">
						<button
							onclick={() => (activeViewTab = 'results')}
							class="rounded-md px-3 py-1 text-xs font-medium transition-colors {activeViewTab ===
							'results'
								? 'bg-[hsl(var(--background))] shadow-sm'
								: 'hover:bg-[hsl(var(--background))]/50'}"
						>
							Results
						</button>
						<button
							onclick={() => (activeViewTab = 'coverage')}
							class="rounded-md px-3 py-1 text-xs font-medium transition-colors {activeViewTab ===
							'coverage'
								? 'bg-[hsl(var(--background))] shadow-sm'
								: 'hover:bg-[hsl(var(--background))]/50'}"
						>
							Coverage
						</button>
						<button
							onclick={() => (activeViewTab = 'plot')}
							class="rounded-md px-3 py-1 text-xs font-medium transition-colors {activeViewTab ===
							'plot'
								? 'bg-[hsl(var(--background))] shadow-sm'
								: 'hover:bg-[hsl(var(--background))]/50'}"
						>
							Plot
						</button>
					</div>
				{/if}

				<!-- Connection status -->
				{#if $status === null}
					<span class="text-sm text-[hsl(var(--muted-foreground))]">Connecting...</span>
				{:else if $status.connected}
					<span class="flex items-center gap-2 text-sm text-green-600 dark:text-green-400">
						<span class="h-2 w-2 rounded-full bg-green-500"></span>
						Connected
					</span>
				{:else}
					<span class="flex items-center gap-2 text-sm text-red-600 dark:text-red-400">
						<span class="h-2 w-2 rounded-full bg-red-500"></span>
						Disconnected
					</span>
				{/if}
			</div>
		</div>
	</header>

	<!-- Error Banner -->
	{#if $error}
		<div class="border-b border-red-300 bg-red-50 px-4 py-2 dark:border-red-800 dark:bg-red-900/20">
			<div class="flex items-center justify-between">
				<p class="text-sm text-red-600 dark:text-red-400">{$error}</p>
				<button onclick={() => clearError()} class="text-sm underline hover:no-underline">
					Dismiss
				</button>
			</div>
		</div>
	{/if}

	<!-- Main Content -->
	{#if $status?.connected}
		<div class="flex flex-1 overflow-hidden">
			<!-- Left Panel: Sidebar with Workspace Switcher + UDF Toolbox -->
			<aside class="flex w-64 shrink-0 flex-col border-r bg-[hsl(var(--card))]">
				<!-- Workspace Switcher -->
				<div class="border-b p-3">
					<WorkspaceSwitcher />
				</div>

				<!-- UDF Toolbox -->
				<div class="flex-1 overflow-y-auto">
					<UdfToolbox />
				</div>
			</aside>

			<!-- Center Panel: Data Selection + Views -->
			<main class="flex flex-1 flex-col overflow-hidden">
				<!-- Data Selection Bar -->
				<div class="border-b bg-[hsl(var(--muted))] p-3">
					<div class="flex items-center gap-4">
						<!-- Well -->
						<div class="flex items-center gap-2">
							<label for="well" class="text-xs font-medium text-[hsl(var(--muted-foreground))]">
								Well:
							</label>
							<select
								id="well"
								value={$selectedWellId ?? ''}
								onchange={(e) => selectWell(e.currentTarget.value)}
								class="rounded-md border bg-[hsl(var(--background))] px-2 py-1 text-sm"
							>
								<option value="">Select...</option>
								{#each $wells as well (well.id)}
									<option value={well.id}>{well.name} ({well.curve_count} curves)</option>
								{/each}
							</select>
						</div>

						<!-- Curve -->
						{#if $selectedWellId && $curves.length > 0}
							<div class="flex items-center gap-2">
								<label for="curve" class="text-xs font-medium text-[hsl(var(--muted-foreground))]">
									Curve:
								</label>
								<select
									id="curve"
									value={$selectedCurveId ?? ''}
									onchange={(e) => selectCurve(e.currentTarget.value)}
									class="rounded-md border bg-[hsl(var(--background))] px-2 py-1 text-sm"
								>
									<option value="">Select...</option>
									{#each $curves as curve (curve.id)}
										<option value={curve.id}>
											{curve.mnemonic}
											{#if curve.main_curve_type}
												({curve.main_curve_type})
											{/if}
										</option>
									{/each}
								</select>
							</div>
						{/if}
					</div>
				</div>

				<!-- Content Area -->
				<div class="flex-1 overflow-y-auto p-4">
					{#if !$selectedWellId}
						<!-- No Well Selected -->
						<div class="flex h-full items-center justify-center">
							<div class="text-center">
								<svg
									class="mx-auto mb-4 h-16 w-16 text-[hsl(var(--muted-foreground))] opacity-30"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="1.5"
										d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"
									/>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="1.5"
										d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"
									/>
								</svg>
								<h2 class="text-lg font-semibold">Select a Well</h2>
								<p class="mt-2 max-w-sm text-sm text-[hsl(var(--muted-foreground))]">
									Choose a well from the dropdown above to see available curves and run
									computations.
								</p>
							</div>
						</div>
					{:else}
						<!-- Well Selected - Show Views -->
						<div class="space-y-4">
							<!-- View Content based on active tab -->
							{#if activeViewTab === 'results'}
								<!-- Execution Result -->
								<ExecutionResult />

								<!-- Prompt to select UDF if none selected -->
								{#if !$selectedUdf}
									<div
										class="flex items-center justify-center rounded-lg border bg-[hsl(var(--card))] p-8 text-center"
									>
										<div>
											<svg
												class="mx-auto mb-4 h-12 w-12 text-[hsl(var(--muted-foreground))] opacity-50"
												fill="none"
												stroke="currentColor"
												viewBox="0 0 24 24"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="1.5"
													d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
												/>
											</svg>
											<h3 class="font-medium">Ready for Computation</h3>
											<p class="mt-1 text-sm text-[hsl(var(--muted-foreground))]">
												Select a tool from the toolbox to configure and run
											</p>
										</div>
									</div>
								{/if}

								<!-- Available Curves Summary -->
								<div class="rounded-lg border bg-[hsl(var(--card))] p-4">
									<h3 class="mb-3 text-sm font-semibold">Available Curves ({$curves.length})</h3>
									<div class="flex flex-wrap gap-2">
										{#each $curves as curve (curve.id)}
											<button
												onclick={() => selectCurve(curve.id)}
												class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs transition-colors {$selectedCurveId ===
												curve.id
													? 'bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))]'
													: 'bg-[hsl(var(--secondary))] hover:bg-[hsl(var(--secondary))]/80'}"
											>
												{curve.mnemonic}
												{#if curve.main_curve_type}
													<span
														class={$selectedCurveId === curve.id
															? 'opacity-70'
															: 'text-[hsl(var(--muted-foreground))]'}
													>
														({curve.main_curve_type})
													</span>
												{/if}
											</button>
										{/each}
									</div>
								</div>
							{:else if activeViewTab === 'coverage'}
								<!-- Coverage View -->
								<CoverageView
									curves={curvesWithData}
									selectedCurveId={$selectedCurveId}
									onCurveSelect={(id) => selectCurve(id)}
								/>
							{:else if activeViewTab === 'plot'}
								<!-- Simple Plot View -->
								{#if $curveData}
									<SimplePlot curves={plotCurves} height={400} />
								{:else}
									<div
										class="flex items-center justify-center rounded-lg border bg-[hsl(var(--card))] p-8"
									>
										<div class="text-center">
											<svg
												class="mx-auto mb-4 h-12 w-12 text-[hsl(var(--muted-foreground))] opacity-30"
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
											<p class="text-sm font-medium">Select a Curve to Plot</p>
											<p class="mt-1 text-xs text-[hsl(var(--muted-foreground))]">
												Choose a curve from the dropdown above to view its data
											</p>
										</div>
									</div>
								{/if}
							{/if}
						</div>
					{/if}
				</div>
			</main>

			<!-- Right Panel: Parameter Form + Inspector/Provenance -->
			<aside class="flex w-80 shrink-0 flex-col border-l bg-[hsl(var(--card))]">
				<!-- Parameter Form (top section) -->
				<div class="flex-1 overflow-y-auto border-b">
					<ParameterForm />
				</div>

				<!-- Inspector/Provenance Tabs (bottom section) -->
				<div class="flex h-96 flex-col">
					<!-- Tab switcher -->
					<div class="flex border-b bg-[hsl(var(--muted))]">
						<button
							onclick={() => (activeInspectorTab = 'artifact')}
							class="flex-1 px-3 py-2 text-xs font-medium transition-colors {activeInspectorTab ===
							'artifact'
								? 'border-b-2 border-[hsl(var(--primary))] bg-[hsl(var(--card))] text-[hsl(var(--foreground))]'
								: 'text-[hsl(var(--muted-foreground))] hover:text-[hsl(var(--foreground))]'}"
						>
							Artifact
						</button>
						<button
							onclick={() => (activeInspectorTab = 'provenance')}
							class="flex-1 px-3 py-2 text-xs font-medium transition-colors {activeInspectorTab ===
							'provenance'
								? 'border-b-2 border-[hsl(var(--primary))] bg-[hsl(var(--card))] text-[hsl(var(--foreground))]'
								: 'text-[hsl(var(--muted-foreground))] hover:text-[hsl(var(--foreground))]'}"
						>
							Provenance
						</button>
					</div>

					<!-- Tab content -->
					<div class="flex-1 overflow-y-auto p-2">
						{#if activeInspectorTab === 'artifact'}
							<ArtifactInspector curve={$selectedCurve ?? null} curveData={$curveData} />
						{:else}
							<ProvenancePanel curve={$selectedCurve ?? null} />
						{/if}
					</div>
				</div>
			</aside>
		</div>
	{:else}
		<!-- Disconnected State -->
		<div class="flex flex-1 items-center justify-center">
			<div class="max-w-md text-center">
				<svg
					class="mx-auto mb-4 h-16 w-16 text-[hsl(var(--muted-foreground))] opacity-30"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="1.5"
						d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
					/>
				</svg>
				<h2 class="text-lg font-semibold">DataForge Not Connected</h2>
				{#if $status?.error}
					<p class="mt-2 text-sm text-red-600 dark:text-red-400">{$status.error}</p>
				{/if}
				{#if $status?.data_dir}
					<p class="mt-2 text-sm text-[hsl(var(--muted-foreground))]">
						Expected location: {$status.data_dir}
					</p>
				{/if}
				<p class="mt-4 text-sm text-[hsl(var(--muted-foreground))]">
					Please ensure DataForge is installed and has been run at least once to create the shared
					database.
				</p>
				<button
					onclick={() => loadStatus()}
					class="mt-4 rounded-md bg-[hsl(var(--primary))] px-4 py-2 text-sm font-medium text-[hsl(var(--primary-foreground))] transition-colors hover:opacity-90"
				>
					Retry Connection
				</button>
			</div>
		</div>
	{/if}
</div>
