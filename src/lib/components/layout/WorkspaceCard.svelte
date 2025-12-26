<script lang="ts">
	/**
	 * Workspace Card - Displays a workspace for selection
	 */
	import type { WorkspaceInfo } from '$lib/types';

	interface Props {
		workspace: WorkspaceInfo;
		isSelected?: boolean;
		loading?: boolean;
		onSelect: () => void;
	}

	let { workspace, isSelected = false, loading = false, onSelect }: Props = $props();
</script>

<button
	onclick={onSelect}
	disabled={loading}
	class="group relative flex w-full flex-col items-start gap-3 rounded-xl border-2 p-4 text-left transition-all {isSelected
		? 'border-[hsl(var(--primary))] bg-[hsl(var(--primary))]/5'
		: 'border-transparent bg-[hsl(var(--card))] hover:border-[hsl(var(--border))] hover:bg-[hsl(var(--muted))]'} {loading
		? 'cursor-wait opacity-70'
		: 'cursor-pointer'}"
>
	<!-- Avatar and Name Row -->
	<div class="flex w-full items-center gap-3">
		<!-- Avatar -->
		<div
			class="flex h-12 w-12 shrink-0 items-center justify-center rounded-lg text-lg font-bold {isSelected
				? 'bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))]'
				: 'bg-[hsl(var(--muted))] text-[hsl(var(--muted-foreground))] group-hover:bg-[hsl(var(--secondary))]'}"
		>
			{workspace.name.charAt(0).toUpperCase()}
		</div>

		<!-- Name -->
		<div class="min-w-0 flex-1">
			<h3 class="truncate text-base font-semibold">{workspace.name}</h3>
			<p class="text-xs text-[hsl(var(--muted-foreground))]">
				Created {new Date(workspace.created_at).toLocaleDateString()}
			</p>
		</div>

		<!-- Loading Spinner or Checkmark -->
		{#if loading}
			<svg class="h-5 w-5 animate-spin text-[hsl(var(--primary))]" fill="none" viewBox="0 0 24 24">
				<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"
				></circle>
				<path
					class="opacity-75"
					fill="currentColor"
					d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
				></path>
			</svg>
		{:else if isSelected}
			<svg class="h-5 w-5 text-[hsl(var(--primary))]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
			</svg>
		{/if}
	</div>
</button>
