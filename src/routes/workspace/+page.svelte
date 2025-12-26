<script lang="ts">
	/**
	 * Workspace Selection Page
	 *
	 * Landing page that displays all available workspaces and allows
	 * the user to select one before proceeding to the workbench.
	 */
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import WorkspaceCard from '$lib/components/layout/WorkspaceCard.svelte';
	import {
		status,
		workspaces,
		selectedWorkspaceId,
		loadStatus,
		selectWorkspace,
		error,
		clearError
	} from '$lib/stores/compute';

	let isSelecting = $state(false);
	let selectingId = $state<string | null>(null);

	async function handleSelect(workspaceId: string) {
		isSelecting = true;
		selectingId = workspaceId;

		try {
			await selectWorkspace(workspaceId);
			goto('/workbench');
		} catch (e) {
			console.error('Failed to select workspace:', e);
		} finally {
			isSelecting = false;
			selectingId = null;
		}
	}

	onMount(() => {
		loadStatus();
	});
</script>

<div class="flex min-h-screen flex-col">
	<!-- Header -->
	<header class="border-b bg-[hsl(var(--card))] px-6 py-4">
		<div class="mx-auto max-w-4xl">
			<div class="flex items-center justify-between">
				<div>
					<h1 class="text-2xl font-bold">DataForge Compute</h1>
					<p class="text-sm text-[hsl(var(--muted-foreground))]">Select a workspace to continue</p>
				</div>

				<!-- Connection Status -->
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
		<div class="border-b border-red-300 bg-red-50 px-6 py-2 dark:border-red-800 dark:bg-red-900/20">
			<div class="mx-auto flex max-w-4xl items-center justify-between">
				<p class="text-sm text-red-600 dark:text-red-400">{$error}</p>
				<button onclick={() => clearError()} class="text-sm underline hover:no-underline">
					Dismiss
				</button>
			</div>
		</div>
	{/if}

	<!-- Main Content -->
	<main class="flex-1 px-6 py-8">
		<div class="mx-auto max-w-4xl">
			{#if $status === null}
				<!-- Loading -->
				<div class="flex items-center justify-center py-16">
					<svg class="h-8 w-8 animate-spin text-[hsl(var(--muted-foreground))]" fill="none" viewBox="0 0 24 24">
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
						<path
							class="opacity-75"
							fill="currentColor"
							d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
						></path>
					</svg>
				</div>
			{:else if !$status.connected}
				<!-- Disconnected -->
				<div class="flex flex-col items-center justify-center py-16 text-center">
					<svg
						class="mb-4 h-16 w-16 text-[hsl(var(--muted-foreground))] opacity-30"
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
					{#if $status.error}
						<p class="mt-2 text-sm text-red-600 dark:text-red-400">{$status.error}</p>
					{/if}
					{#if $status.data_dir}
						<p class="mt-2 text-sm text-[hsl(var(--muted-foreground))]">
							Expected location: {$status.data_dir}
						</p>
					{/if}
					<p class="mt-4 max-w-md text-sm text-[hsl(var(--muted-foreground))]">
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
			{:else if $workspaces.length === 0}
				<!-- No Workspaces -->
				<div class="flex flex-col items-center justify-center py-16 text-center">
					<svg
						class="mb-4 h-16 w-16 text-[hsl(var(--muted-foreground))] opacity-30"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="1.5"
							d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
						/>
					</svg>
					<h2 class="text-lg font-semibold">No Workspaces Available</h2>
					<p class="mt-2 max-w-md text-sm text-[hsl(var(--muted-foreground))]">
						Create a workspace in the DataForge application first, then return here to run
						computations.
					</p>
					<button
						onclick={() => loadStatus()}
						class="mt-4 rounded-md border bg-[hsl(var(--background))] px-4 py-2 text-sm font-medium transition-colors hover:bg-[hsl(var(--muted))]"
					>
						Refresh
					</button>
				</div>
			{:else}
				<!-- Workspace Grid -->
				<div class="space-y-6">
					<div class="flex items-center justify-between">
						<h2 class="text-lg font-semibold">Your Workspaces</h2>
						<span class="text-sm text-[hsl(var(--muted-foreground))]">
							{$workspaces.length} workspace{$workspaces.length === 1 ? '' : 's'}
						</span>
					</div>

					<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
						{#each $workspaces as workspace (workspace.id)}
							<WorkspaceCard
								{workspace}
								isSelected={$selectedWorkspaceId === workspace.id}
								loading={isSelecting && selectingId === workspace.id}
								onSelect={() => handleSelect(workspace.id)}
							/>
						{/each}
					</div>
				</div>
			{/if}
		</div>
	</main>

	<!-- Footer -->
	<footer class="border-t bg-[hsl(var(--card))] px-6 py-3">
		<div class="mx-auto max-w-4xl">
			<p class="text-center text-xs text-[hsl(var(--muted-foreground))]">
				DataForge Compute &middot; Run computations on well log data
			</p>
		</div>
	</footer>
</div>
