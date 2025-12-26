<script lang="ts">
	/**
	 * DataForge Compute Workbench - Main Page
	 *
	 * Three-panel layout:
	 * - Left: Sidebar with workspace switcher + UDF Toolbox
	 * - Center: Data selection + WorkspaceContainer (pane-based charts)
	 * - Right: Context-sensitive toolbar + Artifact Inspector
	 */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import UdfToolbox from '$lib/components/UdfToolbox.svelte';
	import ArtifactInspector from '$lib/components/ArtifactInspector.svelte';
	import ProvenancePanel from '$lib/components/ProvenancePanel.svelte';
	import WorkspaceSwitcher from '$lib/components/layout/WorkspaceSwitcher.svelte';
	import ChartToolbar from '$lib/components/charts/ChartToolbar.svelte';
	import { WorkspaceContainer, ContextToolbar } from '$lib/components/panes';
	import { workspaceManager } from '$lib/panes/workspace-manager';
	import { PaneType } from '$lib/panes/layout-model';
	import {
		status,
		wells,
		curves,
		curveData,
		selectedWorkspaceId,
		selectedWorkspace,
		selectedWellId,
		selectedCurve,
		error,
		loadStatus,
		selectWell,
		clearError
	} from '$lib/stores/compute';

	// Right panel tab state
	let activeInspectorTab = $state<'artifact' | 'provenance'>('artifact');

	// Get selected well info for ContextToolbar
	let selectedWell = $derived($wells.find((w) => w.id === $selectedWellId) ?? null);

	/**
	 * Handle adding a chart from the toolbar
	 * Creates a new pane of the specified chart type
	 */
	function handleAddChart(type: 'line' | 'scatter' | 'histogram' | 'crossplot') {
		// Map toolbar types to pane types
		const paneTypeMap: Record<string, PaneType> = {
			line: PaneType.LineChart,
			scatter: PaneType.ScatterChart,
			histogram: PaneType.Histogram,
			crossplot: PaneType.CrossPlot
		};

		const paneType = paneTypeMap[type] ?? PaneType.LineChart;

		// Add a new pane to the workspace
		// If there's an active pane, split from it; otherwise create at root
		const layout = workspaceManager.saveLayout();
		const activePaneId = layout.activePaneId;
		if (activePaneId) {
			workspaceManager.splitPane(activePaneId, 'right', paneType);
		} else {
			// Add as first pane if workspace is empty
			workspaceManager.addPane(paneType, {}, {
				title: `${type.charAt(0).toUpperCase() + type.slice(1)} Chart`
			});
		}
	}

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

			<!-- Center Panel: Chart Toolbar + Workspace -->
			<main class="flex flex-1 flex-col overflow-hidden">
				<!-- Chart Toolbar - Always visible -->
				<div class="border-b bg-[hsl(var(--card))] px-3 py-2">
					<ChartToolbar onAddChart={handleAddChart} />
				</div>

				<!-- Content Area - Always show WorkspaceContainer -->
				<div class="flex-1 overflow-hidden">
					<WorkspaceContainer />
				</div>
			</main>

			<!-- Right Panel: Context Toolbar + Inspector/Provenance -->
			<aside class="flex w-80 shrink-0 flex-col border-l bg-[hsl(var(--card))]">
				<!-- Context-Sensitive Toolbar (top section) -->
				<!-- Shows UDF parameters when UDF selected, Chart config when pane selected -->
				<div class="flex-1 overflow-y-auto border-b">
					<ContextToolbar
						wells={$wells}
						curves={$curves}
						well={selectedWell}
						onWellChange={(wellId) => selectWell(wellId)}
					/>
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
