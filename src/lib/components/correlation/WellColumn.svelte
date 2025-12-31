<script lang="ts">
	/**
	 * WellColumn - Container for a single well's curve tracks
	 *
	 * Displays:
	 * - Well header (name, color indicator)
	 * - Multiple CurveTrack components (one per track/curve)
	 * - Well tops overlay
	 */
	import type {
		WellCorrelationEntry,
		CorrelationCurveData,
		WellTop
	} from '$lib/charts/correlation-types';
	import CurveTrack from './CurveTrack.svelte';
	import WellTopOverlay from './WellTopOverlay.svelte';

	interface Props {
		/** Well entry with track configuration */
		well: WellCorrelationEntry;
		/** Loaded curve data by track ID */
		curveData: Map<string, CorrelationCurveData>;
		/** Shared depth range */
		depthRange: { min: number; max: number };
		/** Column height in pixels */
		height: number;
		/** Link group for chart synchronization */
		linkGroup: string;
		/** Well tops to display */
		wellTops: WellTop[];
		/** Current cursor depth for crosshair */
		cursorDepth: number | null;
		/** Fixed width per track in pixels (optional, defaults to percentage-based) */
		trackWidth?: number;
	}

	let { well, curveData, depthRange, height, linkGroup, wellTops, cursorDepth, trackWidth }: Props = $props();

	/** Calculate track width - use fixed pixel width if provided, else percentage-based */
	let computedTrackWidth = $derived.by(() => {
		// If trackWidth prop is provided, use fixed pixel width
		if (trackWidth !== undefined) {
			return `${trackWidth}px`;
		}
		// Otherwise, calculate percentage-based width
		const trackCount = well.tracks.length;
		if (trackCount === 0) return '100%';
		if (trackCount === 1) return '100%';
		return `${100 / trackCount}%`;
	});

	/** Header height for calculations */
	const headerHeight = 36;
	const trackAreaHeight = $derived(height - headerHeight);
</script>

<div class="well-column">
	<!-- Well Header -->
	<div class="well-header" style="border-left: 3px solid {well.wellColor}">
		<span class="well-name" title={well.wellName}>{well.wellName}</span>
		<span class="track-count">{well.tracks.length} track{well.tracks.length !== 1 ? 's' : ''}</span>
	</div>

	<!-- Tracks Container -->
	<div class="tracks-container" style="height: {trackAreaHeight}px">
		{#if well.tracks.length === 0}
			<div class="empty-tracks">
				<p>No curves selected</p>
			</div>
		{:else}
			<div class="tracks-row">
				{#each well.tracks as track (track.id)}
					{@const trackData = curveData.get(track.id)}
					<div class="track-wrapper" style="width: {computedTrackWidth}; min-width: 80px">
						<CurveTrack
							{track}
							data={trackData ?? null}
							{depthRange}
							height={trackAreaHeight}
							{linkGroup}
							{cursorDepth}
						/>
					</div>
				{/each}
			</div>

			<!-- Well Tops Overlay (spans all tracks) -->
			{#if wellTops.length > 0}
				<WellTopOverlay
					tops={wellTops}
					{depthRange}
					height={trackAreaHeight}
				/>
			{/if}
		{/if}
	</div>
</div>

<style>
	.well-column {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		overflow: hidden;
	}

	.well-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 12px;
		background: var(--color-bg-secondary, #f9fafb);
		border-bottom: 1px solid var(--color-border, #e5e7eb);
		flex-shrink: 0;
		height: 36px;
		box-sizing: border-box;
	}

	.well-name {
		font-size: 12px;
		font-weight: 600;
		color: var(--color-text, #111827);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 70%;
	}

	.track-count {
		font-size: 10px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.tracks-container {
		flex: 1;
		position: relative;
		overflow: hidden;
	}

	.tracks-row {
		display: flex;
		width: 100%;
		height: 100%;
	}

	.track-wrapper {
		flex-shrink: 0;
		border-right: 1px solid var(--color-border-light, #f3f4f6);
		height: 100%;
	}

	.track-wrapper:last-child {
		border-right: none;
	}

	.empty-tracks {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-tertiary, #9ca3af);
		font-size: 12px;
	}
</style>
