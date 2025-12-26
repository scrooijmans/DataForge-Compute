<script lang="ts">
	/**
	 * CurveSelectorDialog - Modal dialog for selecting curves from the database
	 *
	 * Features:
	 * - Shows all curves in the workspace with well names
	 * - Filters curves based on UDF's allowed_types
	 * - Search by mnemonic, well name, or curve type
	 * - Sort by columns
	 */
	import type { CurveInfoWithWell } from '$lib/types';
	import { allWorkspaceCurves, loadAllWorkspaceCurves } from '$lib/stores/compute';
	import { onMount } from 'svelte';

	interface Props {
		/** Whether the dialog is open */
		open: boolean;
		/** Allowed curve types (if empty, all types allowed) */
		allowedTypes?: string[];
		/** Currently selected curve ID */
		selectedCurveId?: string | null;
		/** Callback when a curve is selected */
		onSelect: (curve: CurveInfoWithWell) => void;
		/** Callback to close the dialog */
		onClose: () => void;
	}

	let { open, allowedTypes = [], selectedCurveId = null, onSelect, onClose }: Props = $props();

	/** Search query */
	let searchQuery = $state('');

	/** Sort column and direction */
	let sortColumn = $state<'well_name' | 'mnemonic' | 'main_curve_type'>('well_name');
	let sortDirection = $state<'asc' | 'desc'>('asc');

	/** Loading state */
	let isLoading = $state(false);

	/** Map of curve type codes to display names (for filtering) */
	const typeCodeToDisplayName: Record<string, string> = {
		GR: 'Gamma Ray',
		RHOB: 'Bulk Density',
		NPHI: 'Neutron Porosity',
		RT: 'Resistivity',
		CALI: 'Caliper',
		DT: 'Sonic',
		SP: 'Spontaneous Potential',
		PE: 'Photo-electric Factor',
		DEPTH: 'Depth',
		OTHER: 'Other'
	};

	/** Filter curves by allowed types and search query */
	let filteredCurves = $derived.by(() => {
		let result = $allWorkspaceCurves;

		// Filter by allowed types
		if (allowedTypes && allowedTypes.length > 0) {
			result = result.filter((c) => {
				// Curves without a known type are NOT compatible with type-restricted parameters
				if (!c.main_curve_type) return false;
				const displayType = typeCodeToDisplayName[c.main_curve_type] || c.main_curve_type;
				return allowedTypes.includes(displayType);
			});
		}

		// Filter by search query
		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase();
			result = result.filter(
				(c) =>
					c.mnemonic.toLowerCase().includes(query) ||
					c.well_name.toLowerCase().includes(query) ||
					(c.main_curve_type && c.main_curve_type.toLowerCase().includes(query)) ||
					(c.unit && c.unit.toLowerCase().includes(query))
			);
		}

		// Sort
		result = [...result].sort((a, b) => {
			let aVal = a[sortColumn] ?? '';
			let bVal = b[sortColumn] ?? '';
			if (typeof aVal === 'string') aVal = aVal.toLowerCase();
			if (typeof bVal === 'string') bVal = bVal.toLowerCase();
			if (aVal < bVal) return sortDirection === 'asc' ? -1 : 1;
			if (aVal > bVal) return sortDirection === 'asc' ? 1 : -1;
			return 0;
		});

		return result;
	});

	/** Handle column header click for sorting */
	function handleSort(column: 'well_name' | 'mnemonic' | 'main_curve_type') {
		if (sortColumn === column) {
			sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
		} else {
			sortColumn = column;
			sortDirection = 'asc';
		}
	}

	/** Handle curve selection */
	function handleSelect(curve: CurveInfoWithWell) {
		onSelect(curve);
		onClose();
	}

	/** Handle backdrop click */
	function handleBackdropClick(event: MouseEvent) {
		if (event.target === event.currentTarget) {
			onClose();
		}
	}

	/** Handle Escape key */
	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}

	/** Load curves when dialog opens */
	$effect(() => {
		if (open && $allWorkspaceCurves.length === 0) {
			isLoading = true;
			loadAllWorkspaceCurves().finally(() => {
				isLoading = false;
			});
		}
	});
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if open}
	<!-- Backdrop -->
	<div
		class="dialog-backdrop"
		onclick={handleBackdropClick}
		role="presentation"
	>
		<!-- Dialog -->
		<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title">
			<!-- Header -->
			<div class="dialog-header">
				<h2 id="dialog-title">Select Curve</h2>
				{#if allowedTypes && allowedTypes.length > 0}
					<p class="dialog-subtitle">
						Showing curves of type: {allowedTypes.join(', ')}
					</p>
				{/if}
				<button
					type="button"
					class="dialog-close"
					onclick={onClose}
					aria-label="Close dialog"
				>
					<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M4 4L16 16M16 4L4 16" />
					</svg>
				</button>
			</div>

			<!-- Search -->
			<div class="dialog-search">
				<svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="11" cy="11" r="8" />
					<path d="M21 21l-4.35-4.35" />
				</svg>
				<input
					type="text"
					placeholder="Search by mnemonic, well, or type..."
					bind:value={searchQuery}
					class="search-input"
				/>
				{#if searchQuery}
					<button
						type="button"
						class="search-clear"
						onclick={() => (searchQuery = '')}
						aria-label="Clear search"
					>
						<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M2 2L12 12M12 2L2 12" />
						</svg>
					</button>
				{/if}
			</div>

			<!-- Table -->
			<div class="dialog-content">
				{#if isLoading}
					<div class="loading-state">
						<svg class="spinner" viewBox="0 0 24 24" fill="none">
							<circle class="spinner-track" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
							<path class="spinner-head" d="M12 2a10 10 0 019 5.5" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
						</svg>
						<span>Loading curves...</span>
					</div>
				{:else if filteredCurves.length === 0}
					<div class="empty-state">
						<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<path d="M9.172 9.172a4 4 0 015.656 5.656M9.172 9.172L4 4m5.172 5.172l5.656 5.656m0 0L20 20M21 21l-6-6" />
						</svg>
						<h3>No curves found</h3>
						{#if searchQuery}
							<p>No curves match "{searchQuery}"</p>
						{:else if allowedTypes && allowedTypes.length > 0}
							<p>No curves of type {allowedTypes.join(', ')} available</p>
						{:else}
							<p>No curves available in this workspace</p>
						{/if}
					</div>
				{:else}
					<table class="curves-table">
						<thead>
							<tr>
								<th class="col-well">
									<button type="button" class="sort-button" onclick={() => handleSort('well_name')}>
										Well
										{#if sortColumn === 'well_name'}
											<span class="sort-indicator">{sortDirection === 'asc' ? '▲' : '▼'}</span>
										{/if}
									</button>
								</th>
								<th class="col-mnemonic">
									<button type="button" class="sort-button" onclick={() => handleSort('mnemonic')}>
										Mnemonic
										{#if sortColumn === 'mnemonic'}
											<span class="sort-indicator">{sortDirection === 'asc' ? '▲' : '▼'}</span>
										{/if}
									</button>
								</th>
								<th class="col-type">
									<button type="button" class="sort-button" onclick={() => handleSort('main_curve_type')}>
										Type
										{#if sortColumn === 'main_curve_type'}
											<span class="sort-indicator">{sortDirection === 'asc' ? '▲' : '▼'}</span>
										{/if}
									</button>
								</th>
								<th class="col-unit">Unit</th>
								<th class="col-rows">Samples</th>
							</tr>
						</thead>
						<tbody>
							{#each filteredCurves as curve (curve.id)}
								<tr
									class="curve-row"
									class:selected={selectedCurveId === curve.id}
									onclick={() => handleSelect(curve)}
								>
									<td class="col-well">{curve.well_name}</td>
									<td class="col-mnemonic">
										<span class="mnemonic">{curve.mnemonic}</span>
									</td>
									<td class="col-type">
										{#if curve.main_curve_type}
											<span class="curve-type-badge">{curve.main_curve_type}</span>
										{:else}
											<span class="unknown-type">—</span>
										{/if}
									</td>
									<td class="col-unit">{curve.unit ?? '—'}</td>
									<td class="col-rows">{curve.row_count.toLocaleString()}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				{/if}
			</div>

			<!-- Footer -->
			<div class="dialog-footer">
				<span class="curve-count">
					{filteredCurves.length} curve{filteredCurves.length === 1 ? '' : 's'}
					{#if allowedTypes && allowedTypes.length > 0}
						(filtered)
					{/if}
				</span>
				<button type="button" class="cancel-button" onclick={onClose}>
					Cancel
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.dialog-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.dialog {
		width: 90%;
		max-width: 800px;
		max-height: 80vh;
		background: var(--color-bg, #ffffff);
		border-radius: 12px;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.dialog-header {
		padding: 16px 20px;
		border-bottom: 1px solid var(--color-border, #e5e7eb);
		position: relative;
	}

	.dialog-header h2 {
		margin: 0;
		font-size: 18px;
		font-weight: 600;
		color: var(--color-text, #111827);
	}

	.dialog-subtitle {
		margin: 4px 0 0 0;
		font-size: 13px;
		color: var(--color-text-secondary, #6b7280);
	}

	.dialog-close {
		position: absolute;
		top: 12px;
		right: 12px;
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		background: transparent;
		border-radius: 6px;
		cursor: pointer;
		color: var(--color-text-tertiary, #9ca3af);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.dialog-close:hover {
		background: var(--color-bg-hover, #f3f4f6);
		color: var(--color-text, #111827);
	}

	.dialog-search {
		padding: 12px 20px;
		border-bottom: 1px solid var(--color-border, #e5e7eb);
		display: flex;
		align-items: center;
		gap: 10px;
		background: var(--color-bg-secondary, #f9fafb);
	}

	.search-icon {
		flex-shrink: 0;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.search-input {
		flex: 1;
		border: none;
		background: transparent;
		font-size: 14px;
		color: var(--color-text, #111827);
		outline: none;
	}

	.search-input::placeholder {
		color: var(--color-text-tertiary, #9ca3af);
	}

	.search-clear {
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		background: var(--color-bg-hover, #e5e7eb);
		border-radius: 4px;
		cursor: pointer;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.search-clear:hover {
		background: var(--color-border, #d1d5db);
		color: var(--color-text, #111827);
	}

	.dialog-content {
		flex: 1;
		overflow-y: auto;
		min-height: 200px;
		max-height: 400px;
	}

	.loading-state,
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 48px 24px;
		color: var(--color-text-tertiary, #9ca3af);
		text-align: center;
	}

	.loading-state svg,
	.empty-state svg {
		margin-bottom: 16px;
		opacity: 0.5;
	}

	.empty-state h3 {
		margin: 0 0 8px 0;
		font-size: 14px;
		font-weight: 600;
		color: var(--color-text-secondary, #6b7280);
	}

	.empty-state p {
		margin: 0;
		font-size: 13px;
	}

	.spinner {
		width: 32px;
		height: 32px;
		animation: spin 1s linear infinite;
	}

	.spinner-track {
		opacity: 0.2;
	}

	.spinner-head {
		stroke: var(--color-primary, #3b82f6);
	}

	@keyframes spin {
		100% {
			transform: rotate(360deg);
		}
	}

	.curves-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 13px;
	}

	.curves-table thead {
		position: sticky;
		top: 0;
		background: var(--color-bg-secondary, #f9fafb);
		z-index: 1;
	}

	.curves-table th {
		padding: 10px 12px;
		text-align: left;
		font-weight: 500;
		color: var(--color-text-secondary, #6b7280);
		border-bottom: 1px solid var(--color-border, #e5e7eb);
	}

	.sort-button {
		display: flex;
		align-items: center;
		gap: 4px;
		border: none;
		background: transparent;
		font: inherit;
		color: inherit;
		cursor: pointer;
		padding: 0;
	}

	.sort-button:hover {
		color: var(--color-text, #111827);
	}

	.sort-indicator {
		font-size: 10px;
	}

	.curve-row {
		cursor: pointer;
		transition: background-color 0.1s ease;
	}

	.curve-row:hover {
		background: var(--color-bg-hover, #f3f4f6);
	}

	.curve-row.selected {
		background: var(--color-primary-light, #eff6ff);
	}

	.curves-table td {
		padding: 10px 12px;
		border-bottom: 1px solid var(--color-border-light, #f3f4f6);
		color: var(--color-text, #111827);
	}

	.col-well {
		width: 25%;
	}

	.col-mnemonic {
		width: 25%;
	}

	.col-type {
		width: 20%;
	}

	.col-unit {
		width: 15%;
	}

	.col-rows {
		width: 15%;
		text-align: right;
	}

	.mnemonic {
		font-weight: 500;
	}

	.curve-type-badge {
		display: inline-block;
		padding: 2px 8px;
		background: var(--color-bg-secondary, #f3f4f6);
		border-radius: 4px;
		font-size: 11px;
		font-weight: 500;
		color: var(--color-text-secondary, #6b7280);
	}

	.unknown-type {
		color: var(--color-text-tertiary, #9ca3af);
	}

	.dialog-footer {
		padding: 12px 20px;
		border-top: 1px solid var(--color-border, #e5e7eb);
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: var(--color-bg-secondary, #f9fafb);
	}

	.curve-count {
		font-size: 12px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.cancel-button {
		padding: 8px 16px;
		border: 1px solid var(--color-border, #e5e7eb);
		background: var(--color-bg, #ffffff);
		border-radius: 6px;
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.15s ease;
	}

	.cancel-button:hover {
		background: var(--color-bg-hover, #f3f4f6);
	}
</style>
