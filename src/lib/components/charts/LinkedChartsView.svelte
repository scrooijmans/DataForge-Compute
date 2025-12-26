<script lang="ts">
	/**
	 * LinkedChartsView - Side-by-side linked charts with synchronized cursor/viewport
	 *
	 * Demonstrates the ChartManager linking capabilities:
	 * - Multiple curves displayed as separate tracks (well log convention)
	 * - Synchronized cursor/tooltip across all charts
	 * - Synchronized depth axis (pan/zoom) across all charts
	 */
	import EChartsChart from './EChartsChart.svelte';
	import type { ChartTrack } from '$lib/charts/types';

	interface Props {
		/** Link group ID for synchronization */
		linkGroup?: string;
		/** Chart tracks to display */
		tracks: ChartTrack[];
		/** Chart height in pixels */
		height?: number;
		/** Sync cursor across charts (default: true) */
		syncCursor?: boolean;
		/** Sync viewport (depth axis) across charts (default: true) */
		syncViewport?: boolean;
	}

	let {
		linkGroup = 'well-log-group',
		tracks,
		height = 500,
		syncCursor = true,
		syncViewport = true
	}: Props = $props();

	// Calculate track width based on number of tracks
	let trackWidth = $derived.by(() => {
		if (tracks.length === 0) return '100%';
		if (tracks.length === 1) return '100%';
		if (tracks.length === 2) return '50%';
		if (tracks.length === 3) return '33.333%';
		return `${100 / tracks.length}%`;
	});
</script>

<div class="linked-charts-container rounded-lg border bg-[hsl(var(--card))]">
	<!-- Header -->
	<div class="flex items-center justify-between border-b p-3">
		<div>
			<h3 class="text-sm font-semibold">Linked Well Log Display</h3>
			<p class="text-xs text-[hsl(var(--muted-foreground))]">
				{tracks.length} track{tracks.length !== 1 ? 's' : ''} • Cursor and depth axis synchronized
			</p>
		</div>
		<div class="flex items-center gap-2">
			{#if syncCursor}
				<span class="rounded bg-blue-100 px-2 py-0.5 text-xs text-blue-700 dark:bg-blue-900/30 dark:text-blue-300">
					Cursor Sync
				</span>
			{/if}
			{#if syncViewport}
				<span class="rounded bg-green-100 px-2 py-0.5 text-xs text-green-700 dark:bg-green-900/30 dark:text-green-300">
					Viewport Sync
				</span>
			{/if}
		</div>
	</div>

	<!-- Charts -->
	{#if tracks.length === 0}
		<div class="flex items-center justify-center p-8" style="height: {height}px">
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
						d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
					/>
				</svg>
				<p class="text-sm font-medium">No Tracks Selected</p>
				<p class="mt-1 text-xs text-[hsl(var(--muted-foreground))]">
					Add curves to see linked chart tracks
				</p>
			</div>
		</div>
	{:else}
		<div class="flex overflow-x-auto">
			{#each tracks as track (track.id)}
				<div class="shrink-0 border-r last:border-r-0" style="width: {trackWidth}; min-width: 200px">
					<EChartsChart
						id={track.id}
						data={track.data}
						type="line"
						title={track.title}
						{height}
						invertY={true}
						showCursor={true}
						enableZoom={true}
						{linkGroup}
						{syncCursor}
						{syncViewport}
						series={track.color ? [{ field: track.data.fields[1]?.name ?? '', color: track.color }] : undefined}
					/>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.linked-charts-container {
		width: 100%;
		overflow: hidden;
	}
</style>
