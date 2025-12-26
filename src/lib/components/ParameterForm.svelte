<script lang="ts">
	/**
	 * Parameter Form - Dynamic form for UDF parameters
	 */
	import {
		selectedUdf,
		udfParameters,
		parameterValues,
		validationErrors,
		curves,
		allWorkspaceCurves,
		setParameterValue,
		executeUdf,
		isExecuting,
		executionResult,
		saveOutputCurve,
		isSaving,
		selectedWellId,
		selectWell
	} from '$lib/stores/compute';
	import type { ParameterDefinition, CurveInfoWithWell } from '$lib/types';
	import CurveSelectorDialog from './CurveSelectorDialog.svelte';

	interface Props {
		onExecute?: () => void;
	}

	let { onExecute }: Props = $props();

	/** Track which curve parameter has the dialog open */
	let openDialogForParam = $state<string | null>(null);

	/** Get display info for a selected curve ID */
	function getSelectedCurveDisplay(curveId: string | null | undefined): { mnemonic: string; wellName: string; curveType: string | null } | null {
		if (!curveId) return null;
		const curve = $allWorkspaceCurves.find(c => c.id === curveId);
		if (curve) {
			return {
				mnemonic: curve.mnemonic,
				wellName: curve.well_name,
				curveType: curve.main_curve_type
			};
		}
		// Fallback to curves store if not found in allWorkspaceCurves
		const fallbackCurve = $curves.find(c => c.id === curveId);
		if (fallbackCurve) {
			return {
				mnemonic: fallbackCurve.mnemonic,
				wellName: '', // Unknown well
				curveType: fallbackCurve.main_curve_type
			};
		}
		return null;
	}

	/** Handle curve selection from dialog */
	function handleCurveSelect(param: ParameterDefinition, curve: CurveInfoWithWell) {
		setParameterValue(param.name, curve.id);
		// Also set the selected well if not already set, or if user selected a curve from a different well
		if (curve.well_id && $selectedWellId !== curve.well_id) {
			selectWell(curve.well_id);
		}
		openDialogForParam = null;
	}

	function handleParameterChange(param: ParameterDefinition, value: unknown) {
		setParameterValue(param.name, value);
	}

	function handleExecute() {
		executeUdf(false);
		onExecute?.();
	}

	function handleExecuteAndSave() {
		executeUdf(true);
		onExecute?.();
	}

	// Check if all required parameters are filled
	let canExecute = $derived.by(() => {
		for (const param of $udfParameters) {
			if (param.required) {
				const value = $parameterValues[param.name];
				if (value === undefined || value === null || value === '') {
					return false;
				}
			}
		}
		return true;
	});
</script>

{#if $selectedUdf}
	<div class="flex h-full flex-col">
		<!-- Header -->
		<div class="border-b p-3">
			<h2 class="text-sm font-semibold">{$selectedUdf.name}</h2>
			<p class="text-xs text-[hsl(var(--muted-foreground))]">{$selectedUdf.description}</p>
		</div>

		<!-- Parameters -->
		<div class="flex-1 overflow-y-auto p-3">
			<div class="space-y-4">
				{#each $udfParameters as param (param.name)}
					<div>
						<label class="mb-1 block text-sm font-medium" for={param.name}>
							{param.label}
							{#if param.required}
								<span class="text-red-500">*</span>
							{/if}
						</label>

						{#if param.type === 'curve'}
							<!-- Curve selector with dialog -->
							{@const selectedCurveInfo = getSelectedCurveDisplay($parameterValues[param.name] as string)}
							<button
								type="button"
								id={param.name}
								onclick={() => (openDialogForParam = param.name)}
								class="curve-select-button w-full rounded-md border bg-[hsl(var(--background))] px-3 py-2 text-sm text-left flex items-center justify-between gap-2 hover:bg-[hsl(var(--muted))] transition-colors"
							>
								{#if selectedCurveInfo}
									<span class="flex-1 min-w-0">
										<span class="font-medium">{selectedCurveInfo.mnemonic}</span>
										{#if selectedCurveInfo.wellName}
											<span class="text-[hsl(var(--muted-foreground))]"> - {selectedCurveInfo.wellName}</span>
										{/if}
										{#if selectedCurveInfo.curveType}
											<span class="ml-1 text-xs text-[hsl(var(--muted-foreground))]">({selectedCurveInfo.curveType})</span>
										{/if}
									</span>
								{:else}
									<span class="text-[hsl(var(--muted-foreground))]">Select a curve...</span>
								{/if}
								<svg class="w-4 h-4 text-[hsl(var(--muted-foreground))] shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l4-4 4 4m0 6l-4 4-4-4" />
								</svg>
							</button>
							{#if param.allowed_types && param.allowed_types.length > 0}
								<p class="mt-1 text-xs text-[hsl(var(--muted-foreground))]">
									Accepts: {param.allowed_types.join(', ')}
								</p>
							{/if}
							<!-- Curve Selector Dialog -->
							<CurveSelectorDialog
								open={openDialogForParam === param.name}
								allowedTypes={param.allowed_types}
								selectedCurveId={$parameterValues[param.name] as string | null}
								onSelect={(curve) => handleCurveSelect(param, curve)}
								onClose={() => (openDialogForParam = null)}
							/>
						{:else if param.type === 'number'}
							<!-- Numeric input -->
							<div class="flex items-center gap-2">
								<input
									id={param.name}
									type="number"
									value={$parameterValues[param.name] ?? param.default ?? ''}
									min={param.min}
									max={param.max}
									step="any"
									oninput={(e) => handleParameterChange(param, parseFloat(e.currentTarget.value))}
									class="w-full rounded-md border bg-[hsl(var(--background))] px-3 py-2 text-sm"
								/>
								{#if param.unit}
									<span class="text-sm text-[hsl(var(--muted-foreground))]">{param.unit}</span>
								{/if}
							</div>
							{#if param.min !== undefined || param.max !== undefined}
								<p class="mt-1 text-xs text-[hsl(var(--muted-foreground))]">
									Range: {param.min ?? '-∞'} to {param.max ?? '∞'}
								</p>
							{/if}
						{:else if param.type === 'boolean'}
							<!-- Boolean checkbox -->
							<label class="flex items-center gap-2">
								<input
									id={param.name}
									type="checkbox"
									checked={Boolean($parameterValues[param.name] ?? param.default ?? false)}
									onchange={(e) => handleParameterChange(param, e.currentTarget.checked)}
									class="rounded border"
								/>
								<span class="text-sm">{param.description}</span>
							</label>
						{:else}
							<!-- String input (fallback) -->
							<input
								id={param.name}
								type="text"
								value={$parameterValues[param.name] ?? param.default ?? ''}
								oninput={(e) => handleParameterChange(param, e.currentTarget.value)}
								class="w-full rounded-md border bg-[hsl(var(--background))] px-3 py-2 text-sm"
							/>
						{/if}

						{#if param.description && param.type !== 'boolean'}
							<p class="mt-1 text-xs text-[hsl(var(--muted-foreground))]">{param.description}</p>
						{/if}

						{#if $validationErrors[param.name]}
							<p class="mt-1 text-xs text-red-500">{$validationErrors[param.name]}</p>
						{/if}
					</div>
				{/each}
			</div>
		</div>

		<!-- Execute Button -->
		<div class="border-t p-3 space-y-2">
			<button
				onclick={handleExecute}
				disabled={!canExecute || $isExecuting || $isSaving}
				class="w-full rounded-md bg-[hsl(var(--primary))] px-4 py-2 text-sm font-medium text-[hsl(var(--primary-foreground))] transition-colors hover:opacity-90 disabled:opacity-50"
			>
				{#if $isExecuting}
					<span class="flex items-center justify-center gap-2">
						<svg class="h-4 w-4 animate-spin" fill="none" viewBox="0 0 24 24">
							<circle
								class="opacity-25"
								cx="12"
								cy="12"
								r="10"
								stroke="currentColor"
								stroke-width="4"
							></circle>
							<path
								class="opacity-75"
								fill="currentColor"
								d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
							></path>
						</svg>
						Executing...
					</span>
				{:else}
					Execute
				{/if}
			</button>

			<!-- Save Button (shown after successful execution) -->
			{#if $executionResult?.success && $executionResult.output_data && !$executionResult.saved}
				<button
					onclick={() => saveOutputCurve()}
					disabled={$isSaving || $isExecuting}
					class="w-full rounded-md border border-green-600 bg-green-600/10 px-4 py-2 text-sm font-medium text-green-600 transition-colors hover:bg-green-600/20 disabled:opacity-50 dark:text-green-400 dark:border-green-400"
				>
					{#if $isSaving}
						<span class="flex items-center justify-center gap-2">
							<svg class="h-4 w-4 animate-spin" fill="none" viewBox="0 0 24 24">
								<circle
									class="opacity-25"
									cx="12"
									cy="12"
									r="10"
									stroke="currentColor"
									stroke-width="4"
								></circle>
								<path
									class="opacity-75"
									fill="currentColor"
									d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
								></path>
							</svg>
							Saving...
						</span>
					{:else}
						<span class="flex items-center justify-center gap-2">
							<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"
								/>
							</svg>
							Save to DataForge
						</span>
					{/if}
				</button>
			{/if}

			<!-- Saved indicator -->
			{#if $executionResult?.saved}
				<div class="flex items-center justify-center gap-2 rounded-md bg-green-600/10 px-4 py-2 text-sm text-green-600 dark:text-green-400">
					<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M5 13l4 4L19 7"
						/>
					</svg>
					Saved to DataForge
				</div>
			{/if}
		</div>
	</div>
{:else}
	<div class="flex h-full items-center justify-center p-4 text-center">
		<div>
			<svg
				class="mx-auto mb-4 h-12 w-12 text-[hsl(var(--muted-foreground))] opacity-50"
				fill="none"
				stroke="currentColor"
				viewBox="0 0 24 24"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="1.5"
					d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
				/>
			</svg>
			<h3 class="font-medium">Select a Tool</h3>
			<p class="mt-1 text-sm text-[hsl(var(--muted-foreground))]">
				Choose a tool from the toolbox to configure its parameters
			</p>
		</div>
	</div>
{/if}
