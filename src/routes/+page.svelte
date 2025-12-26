<script lang="ts">
	/**
	 * Root Page - Redirects to workspace selection
	 */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { selectedWorkspaceId, loadStatus, status } from '$lib/stores/compute';

	onMount(async () => {
		await loadStatus();

		// If a workspace is already selected, go to workbench
		// Otherwise, go to workspace selection
		if ($selectedWorkspaceId) {
			goto('/workbench');
		} else {
			goto('/workspace');
		}
	});
</script>

<!-- Loading state while determining redirect -->
<div class="flex min-h-screen items-center justify-center">
	<div class="text-center">
		<svg
			class="mx-auto h-8 w-8 animate-spin text-[hsl(var(--muted-foreground))]"
			fill="none"
			viewBox="0 0 24 24"
		>
			<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"
			></circle>
			<path
				class="opacity-75"
				fill="currentColor"
				d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
			></path>
		</svg>
		<p class="mt-4 text-sm text-[hsl(var(--muted-foreground))]">Loading...</p>
	</div>
</div>
