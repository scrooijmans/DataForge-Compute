<script lang="ts">
	/**
	 * Coverage View - Visualize curve data coverage across depth intervals
	 *
	 * Shows a visual representation of where each curve has valid data
	 * versus null values across the depth range. Useful for:
	 * - Identifying data gaps
	 * - Comparing coverage across multiple curves
	 * - Planning computations that require specific depth intervals
	 */
	import type { CurveInfo, CurveData } from '$lib/types';

	interface CurveWithData {
		info: CurveInfo;
		data: CurveData | null;
	}

	interface Props {
		curves: CurveWithData[];
		selectedCurveId?: string | null;
		onCurveSelect?: (curveId: string) => void;
	}

	let { curves, selectedCurveId = null, onCurveSelect }: Props = $props();

	// Calculate global depth range across all curves
	let depthRange = $derived.by(() => {
		let min = Infinity;
		let max = -Infinity;

		for (const curve of curves) {
			if (curve.data && curve.data.data.length > 0) {
				const curveMin = curve.data.data[0].depth;
				const curveMax = curve.data.data[curve.data.data.length - 1].depth;
				min = Math.min(min, curveMin);
				max = Math.max(max, curveMax);
			} else if (curve.info.min_depth !== null && curve.info.max_depth !== null) {
				min = Math.min(min, curve.info.min_depth);
				max = Math.max(max, curve.info.max_depth);
			}
		}

		return min < max ? { min, max, range: max - min } : null;
	});

	// Generate coverage intervals for a curve
	function getCoverageIntervals(
		curveData: CurveData | null,
		globalRange: { min: number; max: number; range: number } | null
	): { start: number; end: number; hasData: boolean }[] {
		if (!curveData || !globalRange || curveData.data.length === 0) {
			return [];
		}

		const intervals: { start: number; end: number; hasData: boolean }[] = [];
		let currentInterval: { start: number; hasData: boolean } | null = null;

		for (let i = 0; i < curveData.data.length; i++) {
			const point = curveData.data[i];
			const hasData = point.value !== null;

			if (currentInterval === null) {
				currentInterval = { start: point.depth, hasData };
			} else if (currentInterval.hasData !== hasData) {
				intervals.push({
					start: currentInterval.start,
					end: point.depth,
					hasData: currentInterval.hasData
				});
				currentInterval = { start: point.depth, hasData };
			}
		}

		// Close final interval
		if (currentInterval !== null) {
			intervals.push({
				start: currentInterval.start,
				end: curveData.data[curveData.data.length - 1].depth,
				hasData: currentInterval.hasData
			});
		}

		return intervals;
	}

	// Convert depth to percentage for positioning
	function depthToPercent(depth: number, range: { min: number; range: number }): number {
		return ((depth - range.min) / range.range) * 100;
	}

	// Format depth for display
	function formatDepth(depth: number): string {
		return depth.toFixed(1);
	}
</script>

<div class="rounded-lg border bg-[hsl(var(--card))]">
	<!-- Header -->
	<div class="border-b p-3">
		<h3 class="text-sm font-semibold">Coverage View</h3>
		<p class="text-xs text-[hsl(var(--muted-foreground))]">Data availability across depth</p>
	</div>

	<div class="p-3">
		{#if curves.length === 0}
			<div class="py-8 text-center">
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
				<p class="text-sm font-medium">No Curves Available</p>
				<p class="mt-1 text-xs text-[hsl(var(--muted-foreground))]">
					Select a well to view curve coverage
				</p>
			</div>
		{:else if depthRange}
			<!-- Depth scale -->
			<div class="mb-4 flex items-center justify-between text-xs text-[hsl(var(--muted-foreground))]">
				<span>{formatDepth(depthRange.min)}</span>
				<span>Depth</span>
				<span>{formatDepth(depthRange.max)}</span>
			</div>

			<!-- Coverage tracks -->
			<div class="space-y-2">
				{#each curves as curve (curve.info.id)}
					{@const intervals = getCoverageIntervals(curve.data, depthRange)}
					<button
						onclick={() => onCurveSelect?.(curve.info.id)}
						class="w-full text-left transition-colors {selectedCurveId === curve.info.id
							? 'bg-[hsl(var(--secondary))]'
							: 'hover:bg-[hsl(var(--secondary))]'} rounded p-1"
					>
						<div class="flex items-center gap-2">
							<!-- Curve name -->
							<div class="w-20 flex-shrink-0">
								<span class="text-xs font-medium truncate block">{curve.info.mnemonic}</span>
								{#if curve.info.main_curve_type}
									<span class="text-[10px] text-[hsl(var(--muted-foreground))]">
										{curve.info.main_curve_type}
									</span>
								{/if}
							</div>

							<!-- Coverage bar -->
							<div class="relative h-4 flex-1 overflow-hidden rounded bg-[hsl(var(--muted))]">
								{#if intervals.length > 0}
									{#each intervals as interval}
										<div
											class="absolute top-0 h-full {interval.hasData
												? 'bg-green-500'
												: 'bg-red-300 dark:bg-red-900/50'}"
											style="left: {depthToPercent(interval.start, depthRange)}%; width: {depthToPercent(
												interval.end,
												depthRange
											) - depthToPercent(interval.start, depthRange)}%"
										></div>
									{/each}
								{:else if curve.info.min_depth !== null && curve.info.max_depth !== null}
									<!-- No detailed data, show approximate range -->
									<div
										class="absolute top-0 h-full bg-blue-400/50"
										style="left: {depthToPercent(curve.info.min_depth, depthRange)}%; width: {depthToPercent(
											curve.info.max_depth,
											depthRange
										) - depthToPercent(curve.info.min_depth, depthRange)}%"
									></div>
								{/if}
							</div>

							<!-- Sample count -->
							<div class="w-16 flex-shrink-0 text-right">
								<span class="text-xs text-[hsl(var(--muted-foreground))]">
									{curve.info.row_count.toLocaleString()}
								</span>
							</div>
						</div>
					</button>
				{/each}
			</div>

			<!-- Legend -->
			<div class="mt-4 flex items-center justify-center gap-4 text-xs">
				<div class="flex items-center gap-1">
					<div class="h-3 w-3 rounded bg-green-500"></div>
					<span class="text-[hsl(var(--muted-foreground))]">Valid data</span>
				</div>
				<div class="flex items-center gap-1">
					<div class="h-3 w-3 rounded bg-red-300 dark:bg-red-900/50"></div>
					<span class="text-[hsl(var(--muted-foreground))]">Null values</span>
				</div>
				<div class="flex items-center gap-1">
					<div class="h-3 w-3 rounded bg-blue-400/50"></div>
					<span class="text-[hsl(var(--muted-foreground))]">Approximate</span>
				</div>
			</div>
		{:else}
			<div class="py-8 text-center">
				<p class="text-sm text-[hsl(var(--muted-foreground))]">
					Load curve data to view coverage
				</p>
			</div>
		{/if}
	</div>
</div>
