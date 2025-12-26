<script lang="ts">
	/**
	 * CurveSelector - Component for selecting curves to bind to chart axes
	 *
	 * Shows available curves for the selected well and allows the user
	 * to select which curve to display on a given axis.
	 */
	import type { CurveInfo, WellInfo } from '$lib/types';
	import type { AxisBinding } from '$lib/panes/chart-configs';
	import { isCurveTypeAllowed } from '$lib/panes/chart-configs';

	interface Props {
		/** Current axis binding */
		binding: AxisBinding;
		/** Available curves for the selected well */
		curves: CurveInfo[];
		/** Selected well info */
		well: WellInfo | null;
		/** Label for this axis (e.g., "X Axis", "Y Axis") */
		label: string;
		/** Optional curve type restriction key */
		restrictionKey?: string;
		/** Callback when binding changes */
		onChange: (binding: AxisBinding) => void;
		/** Whether this binding is required */
		required?: boolean;
	}

	let {
		binding,
		curves,
		well,
		label,
		restrictionKey,
		onChange,
		required = false,
	}: Props = $props();

	/** Filter curves based on restrictions */
	let availableCurves = $derived(
		restrictionKey
			? curves.filter((c) => isCurveTypeAllowed(c.main_curve_type, restrictionKey))
			: curves
	);

	/** Currently selected curve info */
	let selectedCurve = $derived(
		binding.curveId ? curves.find((c) => c.id === binding.curveId) : null
	);

	/** Show dropdown state */
	let showDropdown = $state(false);

	/** Search filter */
	let searchFilter = $state('');

	/** Filtered curves based on search */
	let filteredCurves = $derived(
		searchFilter
			? availableCurves.filter(
					(c) =>
						c.mnemonic.toLowerCase().includes(searchFilter.toLowerCase()) ||
						c.main_curve_type?.toLowerCase().includes(searchFilter.toLowerCase())
				)
			: availableCurves
	);

	/**
	 * Handle curve selection
	 */
	function handleSelect(curve: CurveInfo): void {
		onChange({
			...binding,
			curveId: curve.id,
			mnemonic: curve.mnemonic,
		});
		showDropdown = false;
		searchFilter = '';
	}

	/**
	 * Clear selection
	 */
	function handleClear(): void {
		onChange({
			...binding,
			curveId: null,
			mnemonic: undefined,
		});
	}

	/**
	 * Close dropdown when clicking outside
	 */
	function handleClickOutside(event: MouseEvent): void {
		const target = event.target as HTMLElement;
		if (!target.closest('.curve-selector')) {
			showDropdown = false;
			searchFilter = '';
		}
	}

	/**
	 * Handle keyboard navigation
	 */
	function handleKeyDown(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			showDropdown = false;
			searchFilter = '';
		}
	}
</script>

<svelte:window onclick={handleClickOutside} onkeydown={handleKeyDown} />

<div class="curve-selector">
	<label class="selector-label">
		{label}
		{#if required}
			<span class="required">*</span>
		{/if}
	</label>

	{#if !well}
		<div class="no-well-message">
			Select a well first
		</div>
	{:else}
		<div class="selector-container">
			<!-- Selected value / trigger button -->
			<button
				type="button"
				class="selector-trigger"
				class:has-value={binding.curveId}
				onclick={() => (showDropdown = !showDropdown)}
				aria-haspopup="listbox"
				aria-expanded={showDropdown}
			>
				{#if selectedCurve}
					<span class="selected-value">
						<span class="mnemonic">{selectedCurve.mnemonic}</span>
						{#if selectedCurve.main_curve_type}
							<span class="curve-type">({selectedCurve.main_curve_type})</span>
						{/if}
					</span>
					<button
						type="button"
						class="clear-button"
						onclick={(e) => {
							e.stopPropagation();
							handleClear();
						}}
						aria-label="Clear selection"
					>
						<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M2 2L10 10M10 2L2 10" />
						</svg>
					</button>
				{:else}
					<span class="placeholder">Select curve...</span>
				{/if}
				<svg
					class="chevron"
					class:open={showDropdown}
					width="12"
					height="12"
					viewBox="0 0 12 12"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="M2 4L6 8L10 4" />
				</svg>
			</button>

			<!-- Dropdown -->
			{#if showDropdown}
				<div class="dropdown" role="listbox">
					<!-- Search input -->
					<div class="search-container">
						<input
							type="text"
							class="search-input"
							placeholder="Search curves..."
							bind:value={searchFilter}
						/>
					</div>

					<!-- Curve list -->
					<div class="curve-list">
						{#if filteredCurves.length === 0}
							<div class="no-curves">
								{#if searchFilter}
									No curves match "{searchFilter}"
								{:else if restrictionKey}
									No compatible curves available
								{:else}
									No curves available
								{/if}
							</div>
						{:else}
							{#each filteredCurves as curve (curve.id)}
								<button
									type="button"
									class="curve-option"
									class:selected={binding.curveId === curve.id}
									onclick={() => handleSelect(curve)}
									role="option"
									aria-selected={binding.curveId === curve.id}
								>
									<span class="option-mnemonic">{curve.mnemonic}</span>
									<span class="option-details">
										{#if curve.main_curve_type}
											<span class="option-type">{curve.main_curve_type}</span>
										{/if}
										{#if curve.unit}
											<span class="option-unit">{curve.unit}</span>
										{/if}
									</span>
								</button>
							{/each}
						{/if}
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.curve-selector {
		display: flex;
		flex-direction: column;
		gap: 4px;
		position: relative;
	}

	.selector-label {
		font-size: 12px;
		font-weight: 500;
		color: var(--color-text-secondary, #6b7280);
	}

	.required {
		color: var(--color-error, #ef4444);
	}

	.no-well-message {
		font-size: 12px;
		color: var(--color-text-tertiary, #9ca3af);
		font-style: italic;
		padding: 8px 0;
	}

	.selector-container {
		position: relative;
	}

	.selector-trigger {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 8px 12px;
		border: 1px solid var(--color-border, #e5e7eb);
		border-radius: 6px;
		background: var(--color-bg, #ffffff);
		cursor: pointer;
		font-size: 13px;
		text-align: left;
		transition: border-color 0.15s ease, box-shadow 0.15s ease;
	}

	.selector-trigger:hover {
		border-color: var(--color-border-hover, #d1d5db);
	}

	.selector-trigger:focus {
		outline: none;
		border-color: var(--color-primary, #3b82f6);
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
	}

	.selector-trigger.has-value {
		border-color: var(--color-primary, #3b82f6);
	}

	.selected-value {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.mnemonic {
		font-weight: 500;
		color: var(--color-text, #111827);
	}

	.curve-type {
		font-size: 11px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.placeholder {
		flex: 1;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.clear-button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 16px;
		height: 16px;
		border: none;
		background: transparent;
		border-radius: 3px;
		cursor: pointer;
		color: var(--color-text-tertiary, #9ca3af);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.clear-button:hover {
		background: var(--color-bg-hover, #f3f4f6);
		color: var(--color-text, #111827);
	}

	.chevron {
		flex-shrink: 0;
		transition: transform 0.15s ease;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.chevron.open {
		transform: rotate(180deg);
	}

	.dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		margin-top: 4px;
		background: var(--color-bg, #ffffff);
		border: 1px solid var(--color-border, #e5e7eb);
		border-radius: 8px;
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1);
		z-index: 50;
		overflow: hidden;
	}

	.search-container {
		padding: 8px;
		border-bottom: 1px solid var(--color-border, #e5e7eb);
	}

	.search-input {
		width: 100%;
		padding: 6px 10px;
		border: 1px solid var(--color-border, #e5e7eb);
		border-radius: 4px;
		font-size: 12px;
		background: var(--color-bg, #ffffff);
	}

	.search-input:focus {
		outline: none;
		border-color: var(--color-primary, #3b82f6);
	}

	.curve-list {
		max-height: 200px;
		overflow-y: auto;
	}

	.no-curves {
		padding: 12px;
		font-size: 12px;
		color: var(--color-text-tertiary, #9ca3af);
		text-align: center;
	}

	.curve-option {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: 8px 12px;
		border: none;
		background: transparent;
		cursor: pointer;
		font-size: 13px;
		text-align: left;
		transition: background-color 0.15s ease;
	}

	.curve-option:hover {
		background: var(--color-bg-hover, #f3f4f6);
	}

	.curve-option.selected {
		background: var(--color-primary-light, #eff6ff);
	}

	.option-mnemonic {
		font-weight: 500;
		color: var(--color-text, #111827);
	}

	.option-details {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.option-type,
	.option-unit {
		font-size: 11px;
		color: var(--color-text-tertiary, #9ca3af);
	}

	.option-type {
		background: var(--color-bg-secondary, #f3f4f6);
		padding: 2px 6px;
		border-radius: 4px;
	}
</style>
