<script lang="ts">
	/**
	 * UDF Toolbox - Browse and select available UDFs
	 */
	import {
		providers,
		udfs,
		udfsByCategory,
		selectedUdfId,
		selectUdf
	} from '$lib/stores/compute';
	import type { UdfInfo } from '$lib/types';

	let searchQuery = $state('');
	let expandedCategories = $state<Set<string>>(new Set(['Petrophysics', 'Smoothing']));

	// Filter UDFs by search query
	let filteredUdfs = $derived.by(() => {
		if (!searchQuery.trim()) {
			return $udfs;
		}
		const query = searchQuery.toLowerCase();
		return $udfs.filter(
			(udf) =>
				udf.name.toLowerCase().includes(query) ||
				udf.description.toLowerCase().includes(query) ||
				udf.tags.some((tag) => tag.toLowerCase().includes(query))
		);
	});

	// Group filtered UDFs by category
	let filteredByCategory = $derived.by(() => {
		const map = new Map<string, UdfInfo[]>();
		for (const udf of filteredUdfs) {
			const category = udf.category || 'Other';
			if (!map.has(category)) {
				map.set(category, []);
			}
			map.get(category)!.push(udf);
		}
		return map;
	});

	function toggleCategory(category: string) {
		expandedCategories = new Set(expandedCategories);
		if (expandedCategories.has(category)) {
			expandedCategories.delete(category);
		} else {
			expandedCategories.add(category);
		}
	}

	function handleSelectUdf(udf: UdfInfo) {
		selectUdf(udf.full_id);
	}
</script>

<div class="flex h-full flex-col">
	<!-- Header -->
	<div class="border-b p-3">
		<h2 class="text-sm font-semibold">Toolbox</h2>
		<p class="text-xs text-[hsl(var(--muted-foreground))]">
			{$udfs.length} tools from {$providers.length} providers
		</p>
	</div>

	<!-- Search -->
	<div class="border-b p-2">
		<input
			type="text"
			placeholder="Search tools..."
			bind:value={searchQuery}
			class="w-full rounded-md border bg-[hsl(var(--background))] px-3 py-1.5 text-sm"
		/>
	</div>

	<!-- UDF List by Category -->
	<div class="flex-1 overflow-y-auto">
		{#each [...filteredByCategory.entries()] as [category, categoryUdfs] (category)}
			<div class="border-b">
				<!-- Category Header -->
				<button
					onclick={() => toggleCategory(category)}
					class="flex w-full items-center gap-2 px-3 py-2 text-left hover:bg-[hsl(var(--secondary))]"
				>
					<svg
						class="h-3 w-3 transition-transform {expandedCategories.has(category) ? 'rotate-90' : ''}"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
					</svg>
					<span class="text-sm font-medium">{category}</span>
					<span class="text-xs text-[hsl(var(--muted-foreground))]">({categoryUdfs.length})</span>
				</button>

				<!-- UDFs in Category -->
				{#if expandedCategories.has(category)}
					<div class="pb-1">
						{#each categoryUdfs as udf (udf.full_id)}
							<button
								onclick={() => handleSelectUdf(udf)}
								class="w-full px-4 py-2 text-left transition-colors {$selectedUdfId === udf.full_id
									? 'bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))]'
									: 'hover:bg-[hsl(var(--secondary))]'}"
							>
								<div class="text-sm font-medium">{udf.name}</div>
								<div class="text-xs opacity-70 line-clamp-1">{udf.description}</div>
								{#if udf.tags.length > 0}
									<div class="mt-1 flex flex-wrap gap-1">
										{#each udf.tags.slice(0, 3) as tag}
											<span class="rounded bg-[hsl(var(--muted))] px-1.5 py-0.5 text-[10px]">
												{tag}
											</span>
										{/each}
									</div>
								{/if}
							</button>
						{/each}
					</div>
				{/if}
			</div>
		{/each}

		{#if filteredByCategory.size === 0}
			<div class="p-4 text-center text-sm text-[hsl(var(--muted-foreground))]">
				No tools found matching "{searchQuery}"
			</div>
		{/if}
	</div>
</div>
