<script lang="ts">
	/**
	 * WellCorrelationPanel - Main container for multi-well correlation display
	 *
	 * Implements Grafana-inspired architecture:
	 * - Props passed down from PaneContainer (single source of truth)
	 * - Callbacks bubble up for state changes
	 * - No child-to-child communication
	 *
	 * Layout: DepthTrack | WellColumn | WellColumn | ...
	 */
	import type { CorrelationConfig, CorrelationCurveData } from '$lib/charts/correlation-types';
	import WellColumn from './WellColumn.svelte';
	import DepthTrack from './DepthTrack.svelte';
	import { chartManager } from '$lib/charts/chart-manager';
	import { onMount, onDestroy } from 'svelte';

	interface Props {
		/** Correlation configuration (single source of truth) */
		config: CorrelationConfig;
		/** Loaded curve data by track ID */
		curveData: Map<string, CorrelationCurveData>;
		/** Calculated depth range (shared across all tracks) */
		depthRange: { min: number; max: number };
		/** Panel height in pixels */
		height?: number;
		/** Callback when depth range changes (from pan/zoom) */
		onDepthRangeChange?: (range: { min: number; max: number }) => void;
		/** Callback when cursor depth changes */
		onCursorMove?: (depth: number | null) => void;
	}

	let {
		config,
		curveData,
		depthRange,
		height = 600,
		onDepthRangeChange,
		onCursorMove
	}: Props = $props();

	/** Link group ID for chart synchronization */
	const linkGroup = $derived(config.id);

	/** Current cursor depth (for crosshair sync) */
	let cursorDepth = $state<number | null>(null);

	/** Visible depth range after zooming (null = show full range) */
	let visibleDepthRange = $state<{ min: number; max: number } | null>(null);

	/** Effective depth range to use (visible if zoomed, otherwise full data range) */
	let effectiveDepthRange = $derived(visibleDepthRange ?? depthRange);

	/** Calculate column width based on layout config and track count */
	function getWellColumnWidth(trackCount: number): string {
		// Use fixed track width from layout config
		const trackWidth = config.layout?.trackWidth ?? 140;
		return `${trackCount * trackWidth}px`;
	}

	/** Subscribe to cursor events from ChartManager */
	let cursorUnsubscribe: (() => void) | null = null;
	let viewportUnsubscribe: (() => void) | null = null;

	onMount(() => {
		// Subscribe to cursor events for crosshair sync
		cursorUnsubscribe = chartManager.onCursor((event) => {
			if (event.groupId === linkGroup) {
				cursorDepth = event.position?.y ?? null;
				onCursorMove?.(cursorDepth);
			}
		});

		// Subscribe to viewport events for depth sync
		viewportUnsubscribe = chartManager.onViewport((event) => {
			if (event.groupId === linkGroup) {
				// Update visible depth range from zoom
				const newRange = {
					min: event.viewport.yMin,
					max: event.viewport.yMax
				};
				visibleDepthRange = newRange;
				onDepthRangeChange?.(newRange);
			}
		});
	});

	onDestroy(() => {
		cursorUnsubscribe?.();
		viewportUnsubscribe?.();
	});
</script>

<div class="correlation-panel" style="height: {height}px">
	<!-- Header -->
	<div class="panel-header">
		<h3 class="panel-title">{config.title || 'Well Correlation'}</h3>
		<div class="panel-info">
			<span class="info-badge">{config.wells.length} well{config.wells.length !== 1 ? 's' : ''}</span>
			{#if config.enableZoom}
				<span class="info-badge sync">Sync Enabled</span>
			{/if}
		</div>
	</div>

	<!-- Main content area -->
	<div class="panel-content">
		{#if config.wells.length === 0}
			<!-- Empty state -->
			<div class="empty-state">
				<svg class="empty-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="1.5"
						d="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
					/>
				</svg>
				<p class="empty-title">No Wells Selected</p>
				<p class="empty-subtitle">Use the configuration panel to add wells and curves</p>
			</div>
		{:else}
			<!-- Depth track (leftmost) -->
			<div class="depth-track-container">
				<DepthTrack
					depthRange={effectiveDepthRange}
					height={height - 60}
					inverted={config.depthRange.inverted}
				/>
			</div>

			<!-- Well columns -->
			<div class="wells-container">
				{#each config.wells as well (well.wellId)}
					<div class="well-column-wrapper" style="width: {getWellColumnWidth(well.tracks.length)}; min-width: {config.layout?.trackWidth ?? 140}px">
						<WellColumn
							{well}
							{curveData}
							{depthRange}
							height={height - 60}
							{linkGroup}
							wellTops={config.wellTops.filter(t => !t.wellId || t.wellId === well.wellId)}
							{cursorDepth}
							trackWidth={config.layout?.trackWidth}
						/>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.correlation-panel {
		display: flex;
		flex-direction: column;
		width: 100%;
		overflow: hidden;
		background: var(--color-bg, #ffffff);
		border-radius: 8px;
		border: 1px solid var(--color-border, #e5e7eb);
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 16px;
		border-bottom: 1px solid var(--color-border, #e5e7eb);
		background: var(--color-bg-secondary, #f9fafb);
		flex-shrink: 0;
	}

	.panel-title {
		margin: 0;
		font-size: 14px;
		font-weight: 600;
		color: var(--color-text, #111827);
	}

	.panel-info {
		display: flex;
		gap: 8px;
	}

	.info-badge {
		padding: 2px 8px;
		font-size: 11px;
		border-radius: 4px;
		background: var(--color-bg-tertiary, #e5e7eb);
		color: var(--color-text-secondary, #6b7280);
	}

	.info-badge.sync {
		background: #dbeafe;
		color: #1d4ed8;
	}

	.panel-content {
		flex: 1;
		display: flex;
		overflow: hidden;
	}

	.empty-state {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 40px;
		text-align: center;
	}

	.empty-icon {
		width: 48px;
		height: 48px;
		color: var(--color-text-tertiary, #9ca3af);
		opacity: 0.5;
		margin-bottom: 16px;
	}

	.empty-title {
		margin: 0;
		font-size: 14px;
		font-weight: 500;
		color: var(--color-text, #111827);
	}

	.empty-subtitle {
		margin: 8px 0 0;
		font-size: 12px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.depth-track-container {
		width: 60px;
		flex-shrink: 0;
		border-right: 1px solid var(--color-border, #e5e7eb);
	}

	.wells-container {
		flex: 1;
		display: flex;
		overflow-x: auto;
		overflow-y: hidden;

		/* Visible scrollbar styling for horizontal navigation */
		scrollbar-width: thin;
		scrollbar-color: var(--color-text-tertiary, #9ca3af) var(--color-bg-secondary, #f9fafb);
	}

	.wells-container::-webkit-scrollbar {
		height: 12px;
	}

	.wells-container::-webkit-scrollbar-track {
		background: var(--color-bg-secondary, #f9fafb);
		border-radius: 6px;
	}

	.wells-container::-webkit-scrollbar-thumb {
		background: var(--color-text-tertiary, #9ca3af);
		border-radius: 6px;
		border: 2px solid var(--color-bg-secondary, #f9fafb);
	}

	.wells-container::-webkit-scrollbar-thumb:hover {
		background: var(--color-text-secondary, #6b7280);
	}

	.well-column-wrapper {
		flex-shrink: 0;
		border-right: 1px solid var(--color-border, #e5e7eb);
	}

	.well-column-wrapper:last-child {
		border-right: none;
	}
</style>
