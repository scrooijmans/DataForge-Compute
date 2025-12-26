<script lang="ts">
	/**
	 * Workspace Switcher - Displays current workspace and allows switching
	 */
	import { goto } from '$app/navigation';
	import { selectedWorkspace } from '$lib/stores/compute';

	function switchWorkspace() {
		goto('/workspace');
	}
</script>

<button
	onclick={switchWorkspace}
	class="flex w-full items-center gap-3 rounded-lg bg-[hsl(var(--primary))] p-2 text-left text-[hsl(var(--primary-foreground))] transition-colors hover:opacity-90"
>
	<!-- Workspace Avatar -->
	<div
		class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-[hsl(var(--primary-foreground))]/20 text-sm font-semibold"
	>
		{#if $selectedWorkspace}
			{$selectedWorkspace.name.charAt(0).toUpperCase()}
		{:else}
			?
		{/if}
	</div>

	<!-- Workspace Name -->
	<div class="min-w-0 flex-1">
		<p class="truncate text-sm font-medium">
			{#if $selectedWorkspace}
				{$selectedWorkspace.name}
			{:else}
				Select Workspace
			{/if}
		</p>
		<p class="truncate text-xs opacity-70">Click to switch</p>
	</div>

	<!-- Chevron -->
	<svg class="h-4 w-4 shrink-0 opacity-70" fill="none" stroke="currentColor" viewBox="0 0 24 24">
		<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l4-4 4 4m0 6l-4 4-4-4" />
	</svg>
</button>
