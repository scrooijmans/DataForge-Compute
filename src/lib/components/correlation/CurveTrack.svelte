<script lang="ts">
	/**
	 * CurveTrack - Individual curve chart within a well column
	 *
	 * Wraps EChartsChart for a single curve, configured for well log display:
	 * - X-axis: curve values
	 * - Y-axis: depth (inverted, shared via linkGroup)
	 */
	import type { CorrelationTrack, CorrelationCurveData } from '$lib/charts/correlation-types';
	import EChartsChart from '$lib/components/charts/EChartsChart.svelte';
	import type { SegmentedCurveData } from '$lib/types';

	interface Props {
		/** Track configuration */
		track: CorrelationTrack;
		/** Loaded curve data (null if not yet loaded) */
		data: CorrelationCurveData | null;
		/** Shared depth range */
		depthRange: { min: number; max: number };
		/** Track height in pixels */
		height: number;
		/** Link group for synchronization */
		linkGroup: string;
		/** Current cursor depth */
		cursorDepth: number | null;
	}

	let { track, data, depthRange, height, linkGroup, cursorDepth }: Props = $props();

	/** Convert CorrelationCurveData to SegmentedCurveData for EChartsChart
	 * This uses the segmentedData prop path which respects disableDownsampling
	 */
	let segmentedChartData = $derived.by((): SegmentedCurveData | null => {
		if (!data || data.segments.length === 0) {
			return null;
		}

		// Convert CorrelationCurveData to SegmentedCurveData format
		return {
			curve_id: data.trackId,
			mnemonic: data.mnemonic,
			unit: data.unit,
			segments: data.segments.map(seg => ({
				depth_start: seg.depthStart,
				depth_end: seg.depthEnd,
				depths: seg.depths,
				values: seg.values
			})),
			depth_range: [data.depthRange.min, data.depthRange.max] as [number, number],
			total_points: data.totalPoints
		};
	});

	/** Header height */
	const headerHeight = 24;
	const chartHeight = $derived(height - headerHeight);

	/** Chart ID for registration */
	const chartId = $derived(`correlation-${track.id}`);
</script>

<div class="curve-track">
	<!-- Track Header -->
	<div class="track-header">
		<span class="mnemonic" style="color: {track.color}">{track.mnemonic}</span>
	</div>

	<!-- Chart Area -->
	<div class="chart-container" style="height: {chartHeight}px">
		{#if data && segmentedChartData}
			<EChartsChart
				id={chartId}
				data={null}
				segmentedData={segmentedChartData}
				type="line"
				title=""
				height={chartHeight}
				invertY={true}
				showCursor={true}
				enableZoom={true}
				{linkGroup}
				syncCursor={true}
				syncViewport={true}
				yAxisMin={depthRange.min}
				yAxisMax={depthRange.max}
				hideYAxis={true}
				xAxisMin={track.xMin}
				xAxisMax={track.xMax}
				xAxisLogScale={track.logScale}
				xAxisPosition="top"
				disableDownsampling={true}
				yAxisOnlyZoom={true}
				series={[{
					field: track.mnemonic,
					color: track.color,
					width: track.lineWidth ?? 1
				}]}
			/>
		{:else}
			<div class="loading-state">
				{#if data === null}
					<span class="loading-text">Loading...</span>
				{:else}
					<span class="no-data-text">No data</span>
				{/if}
			</div>
		{/if}

		<!-- Cursor crosshair overlay -->
		{#if cursorDepth !== null}
			{@const yRatio = (cursorDepth - depthRange.min) / (depthRange.max - depthRange.min)}
			{@const yPos = yRatio * chartHeight}
			{#if yPos >= 0 && yPos <= chartHeight}
				<div class="cursor-line" style="top: {yPos}px"></div>
			{/if}
		{/if}
	</div>
</div>

<style>
	.curve-track {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		overflow: hidden;
	}

	.track-header {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 4px 8px;
		height: 24px;
		border-bottom: 1px solid var(--color-border-light, #f3f4f6);
		background: var(--color-bg, #ffffff);
		flex-shrink: 0;
		box-sizing: border-box;
	}

	.mnemonic {
		font-size: 11px;
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.chart-container {
		flex: 1;
		position: relative;
		overflow: hidden;
	}

	.loading-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		background: var(--color-bg-tertiary, #f3f4f6);
	}

	.loading-text {
		font-size: 11px;
		color: var(--color-text-tertiary, #9ca3af);
		animation: pulse 1.5s infinite;
	}

	.no-data-text {
		font-size: 11px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.cursor-line {
		position: absolute;
		left: 0;
		right: 0;
		height: 1px;
		background: rgba(59, 130, 246, 0.5);
		pointer-events: none;
		z-index: 10;
	}

	@keyframes pulse {
		0%, 100% { opacity: 0.5; }
		50% { opacity: 1; }
	}
</style>
