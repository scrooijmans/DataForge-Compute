<script lang="ts">
	/**
	 * Artifact Inspector - Display detailed curve metadata and statistics
	 *
	 * Shows comprehensive information about a selected curve including:
	 * - Basic metadata (mnemonic, unit, description)
	 * - Depth range and sample count
	 * - Value statistics (min, max, mean, null count)
	 * - Provenance information for derived curves
	 */
	import { invoke } from '@tauri-apps/api/core';
	import type { CurveInfo, CurveData } from '$lib/types';

	interface Props {
		curve: CurveInfo | null;
		curveData?: CurveData | null;
	}

	let { curve, curveData = null }: Props = $props();

	// Computed statistics from curve data
	let statistics = $derived.by(() => {
		if (!curveData || !curveData.data.length) {
			return null;
		}

		const values = curveData.data.map((p) => p.value).filter((v): v is number => v !== null);
		const nullCount = curveData.data.length - values.length;

		if (values.length === 0) {
			return {
				min: null,
				max: null,
				mean: null,
				nullCount,
				validCount: 0,
				totalCount: curveData.data.length,
				depthMin: curveData.data[0]?.depth ?? null,
				depthMax: curveData.data[curveData.data.length - 1]?.depth ?? null
			};
		}

		const min = Math.min(...values);
		const max = Math.max(...values);
		const mean = values.reduce((a, b) => a + b, 0) / values.length;

		return {
			min,
			max,
			mean,
			nullCount,
			validCount: values.length,
			totalCount: curveData.data.length,
			depthMin: curveData.data[0].depth,
			depthMax: curveData.data[curveData.data.length - 1].depth
		};
	});

	function formatNumber(n: number | null | undefined, decimals = 4): string {
		if (n === null || n === undefined) return 'N/A';
		if (Math.abs(n) < 0.0001 || Math.abs(n) > 10000) {
			return n.toExponential(2);
		}
		return n.toFixed(decimals);
	}
</script>

<div class="rounded-lg border bg-[hsl(var(--card))]">
	<!-- Header -->
	<div class="border-b p-3">
		<div class="flex items-center justify-between">
			<div>
				<h3 class="text-sm font-semibold">Artifact Inspector</h3>
				<p class="text-xs text-[hsl(var(--muted-foreground))]">Curve metadata and statistics</p>
			</div>
			{#if curve?.main_curve_type}
				<span class="rounded-full bg-[hsl(var(--secondary))] px-2 py-0.5 text-xs">
					{curve.main_curve_type}
				</span>
			{/if}
		</div>
	</div>

	<div class="p-3">
		{#if curve}
			<div class="space-y-4">
				<!-- Basic Info -->
				<div>
					<h4 class="mb-2 text-xs font-medium text-[hsl(var(--muted-foreground))]">
						Identification
					</h4>
					<div class="space-y-1 text-sm">
						<div class="flex justify-between">
							<span class="text-[hsl(var(--muted-foreground))]">Mnemonic:</span>
							<span class="font-medium">{curve.mnemonic}</span>
						</div>
						{#if curve.unit}
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Unit:</span>
								<span>{curve.unit}</span>
							</div>
						{/if}
						{#if curve.description}
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Description:</span>
								<span class="text-right max-w-[60%] truncate" title={curve.description}>
									{curve.description}
								</span>
							</div>
						{/if}
						<div class="flex justify-between">
							<span class="text-[hsl(var(--muted-foreground))]">ID:</span>
							<span class="font-mono text-xs">{curve.id.slice(0, 8)}...</span>
						</div>
					</div>
				</div>

				<!-- Depth Range -->
				<div>
					<h4 class="mb-2 text-xs font-medium text-[hsl(var(--muted-foreground))]">Depth Range</h4>
					<div class="space-y-1 text-sm">
						{#if statistics}
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Top:</span>
								<span>{formatNumber(statistics.depthMin, 2)}</span>
							</div>
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Bottom:</span>
								<span>{formatNumber(statistics.depthMax, 2)}</span>
							</div>
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Interval:</span>
								<span>
									{statistics.depthMin !== null && statistics.depthMax !== null
										? formatNumber(statistics.depthMax - statistics.depthMin, 2)
										: 'N/A'}
								</span>
							</div>
						{:else if curve.min_depth !== null && curve.max_depth !== null}
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Top:</span>
								<span>{formatNumber(curve.min_depth, 2)}</span>
							</div>
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Bottom:</span>
								<span>{formatNumber(curve.max_depth, 2)}</span>
							</div>
						{:else}
							<p class="text-xs text-[hsl(var(--muted-foreground))]">Load curve data to see depth range</p>
						{/if}
					</div>
				</div>

				<!-- Sample Count -->
				<div>
					<h4 class="mb-2 text-xs font-medium text-[hsl(var(--muted-foreground))]">Samples</h4>
					<div class="space-y-1 text-sm">
						<div class="flex justify-between">
							<span class="text-[hsl(var(--muted-foreground))]">Total:</span>
							<span>{(statistics?.totalCount ?? curve.row_count).toLocaleString()}</span>
						</div>
						{#if statistics}
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Valid:</span>
								<span class="text-green-600 dark:text-green-400">
									{statistics.validCount.toLocaleString()}
								</span>
							</div>
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Null:</span>
								<span class="text-yellow-600 dark:text-yellow-400">
									{statistics.nullCount.toLocaleString()}
								</span>
							</div>
							<!-- Coverage bar -->
							<div class="mt-2">
								<div class="h-2 w-full overflow-hidden rounded-full bg-[hsl(var(--muted))]">
									<div
										class="h-full bg-green-500"
										style="width: {(statistics.validCount / statistics.totalCount) * 100}%"
									></div>
								</div>
								<p class="mt-1 text-center text-xs text-[hsl(var(--muted-foreground))]">
									{((statistics.validCount / statistics.totalCount) * 100).toFixed(1)}% valid
								</p>
							</div>
						{/if}
					</div>
				</div>

				<!-- Value Statistics -->
				{#if statistics && statistics.validCount > 0}
					<div>
						<h4 class="mb-2 text-xs font-medium text-[hsl(var(--muted-foreground))]">
							Value Statistics
						</h4>
						<div class="space-y-1 text-sm">
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Min:</span>
								<span>{formatNumber(statistics.min)}</span>
							</div>
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Max:</span>
								<span>{formatNumber(statistics.max)}</span>
							</div>
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Mean:</span>
								<span>{formatNumber(statistics.mean)}</span>
							</div>
							<div class="flex justify-between">
								<span class="text-[hsl(var(--muted-foreground))]">Range:</span>
								<span>
									{statistics.min !== null && statistics.max !== null
										? formatNumber(statistics.max - statistics.min)
										: 'N/A'}
								</span>
							</div>
						</div>
					</div>
				{/if}

				<!-- Data Quality Indicators -->
				{#if statistics}
					<div>
						<h4 class="mb-2 text-xs font-medium text-[hsl(var(--muted-foreground))]">
							Data Quality
						</h4>
						<div class="flex flex-wrap gap-2">
							{#if statistics.nullCount === 0}
								<span
									class="inline-flex items-center gap-1 rounded-full bg-green-100 px-2 py-0.5 text-xs text-green-700 dark:bg-green-900/30 dark:text-green-400"
								>
									<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M5 13l4 4L19 7"
										/>
									</svg>
									Complete
								</span>
							{:else if statistics.nullCount / statistics.totalCount < 0.1}
								<span
									class="inline-flex items-center gap-1 rounded-full bg-yellow-100 px-2 py-0.5 text-xs text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400"
								>
									Minor gaps
								</span>
							{:else}
								<span
									class="inline-flex items-center gap-1 rounded-full bg-red-100 px-2 py-0.5 text-xs text-red-700 dark:bg-red-900/30 dark:text-red-400"
								>
									Significant gaps
								</span>
							{/if}

							{#if statistics.validCount > 100}
								<span
									class="inline-flex items-center gap-1 rounded-full bg-blue-100 px-2 py-0.5 text-xs text-blue-700 dark:bg-blue-900/30 dark:text-blue-400"
								>
									High resolution
								</span>
							{/if}
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<!-- No curve selected -->
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
						d="M7 12l3-3 3 3 4-4M8 21l4-4 4 4M3 4h18M4 4v16"
					/>
				</svg>
				<p class="text-sm font-medium">No Curve Selected</p>
				<p class="mt-1 text-xs text-[hsl(var(--muted-foreground))]">
					Select a curve to view its metadata and statistics
				</p>
			</div>
		{/if}
	</div>
</div>
