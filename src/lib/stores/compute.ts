/**
 * Svelte stores for compute state management
 */
import { writable, derived, get, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type {
	DataForgeStatus,
	WorkspaceInfo,
	WellInfo,
	CurveInfo,
	CurveInfoWithWell,
	CurveData,
	ProviderInfo,
	UdfInfo,
	ParameterDefinition,
	ExecuteUdfResult,
	CurveDataPoint
} from '$lib/types';
import { workspaceManager } from '$lib/panes/workspace-manager';
import { PaneType } from '$lib/panes/layout-model';

// Connection and data stores
export const status = writable<DataForgeStatus | null>(null);
export const workspaces = writable<WorkspaceInfo[]>([]);
export const wells = writable<WellInfo[]>([]);
export const curves = writable<CurveInfo[]>([]);
export const allWorkspaceCurves = writable<CurveInfoWithWell[]>([]);
export const curveData = writable<CurveData | null>(null);

// Selection stores
export const selectedWorkspaceId = writable<string | null>(null);
export const selectedWellId = writable<string | null>(null);
export const selectedCurveId = writable<string | null>(null);

/**
 * Tracks the previous workspace ID for layout persistence.
 *
 * This store persists across component unmount/remount cycles (e.g., when navigating
 * from /workbench to /workspace and back). Without this, WorkspaceContainer's local
 * previousWorkspaceId variable would reset to null on unmount, causing layouts to be
 * saved to the wrong workspace key during switches.
 */
export const previousWorkspaceIdForLayout = writable<string | null>(null);

// UDF stores
export const providers = writable<ProviderInfo[]>([]);
export const udfs = writable<UdfInfo[]>([]);
export const selectedUdfId = writable<string | null>(null);
export const udfParameters = writable<ParameterDefinition[]>([]);
export const parameterValues = writable<Record<string, unknown>>({});

// Execution stores
export const isExecuting = writable(false);
export const isSaving = writable(false);
export const executionResult = writable<ExecuteUdfResult | null>(null);
export const validationErrors = writable<Record<string, string>>({});

// UI stores
export const isLoading = writable(false);
export const error = writable<string | null>(null);

// Derived stores
export const selectedWorkspace: Readable<WorkspaceInfo | undefined> = derived(
	[workspaces, selectedWorkspaceId],
	([$workspaces, $selectedWorkspaceId]) =>
		$workspaces.find((w) => w.id === $selectedWorkspaceId)
);

export const selectedWell: Readable<WellInfo | undefined> = derived(
	[wells, selectedWellId],
	([$wells, $selectedWellId]) => $wells.find((w) => w.id === $selectedWellId)
);

export const selectedCurve: Readable<CurveInfo | undefined> = derived(
	[curves, selectedCurveId],
	([$curves, $selectedCurveId]) => $curves.find((c) => c.id === $selectedCurveId)
);

export const selectedUdf: Readable<UdfInfo | undefined> = derived(
	[udfs, selectedUdfId],
	([$udfs, $selectedUdfId]) => $udfs.find((u) => u.full_id === $selectedUdfId)
);

export const udfsByCategory: Readable<Map<string, UdfInfo[]>> = derived(udfs, ($udfs) => {
	const map = new Map<string, UdfInfo[]>();
	for (const udf of $udfs) {
		const category = udf.category || 'Other';
		if (!map.has(category)) {
			map.set(category, []);
		}
		map.get(category)!.push(udf);
	}
	return map;
});

// Curve type filter for UDF parameters
export const curvesMatchingType = (allowedTypes: string[] | undefined): Readable<CurveInfo[]> =>
	derived(curves, ($curves) => {
		if (!allowedTypes || allowedTypes.length === 0) {
			return $curves;
		}
		return $curves.filter((c) => {
			if (!c.main_curve_type) return true; // Unknown type - allow selection
			return allowedTypes.includes(c.main_curve_type);
		});
	});

// Actions
export async function loadStatus() {
	try {
		const result = await invoke<DataForgeStatus>('get_dataforge_status');
		status.set(result);
		if (result.connected) {
			await loadWorkspaces();
			await loadProviders();
			await loadUdfs();
		}
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
	}
}

export async function loadWorkspaces() {
	try {
		const result = await invoke<WorkspaceInfo[]>('list_workspaces');
		workspaces.set(result);
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
	}
}

export async function loadProviders() {
	try {
		const result = await invoke<ProviderInfo[]>('list_providers');
		providers.set(result);
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
	}
}

export async function loadUdfs() {
	try {
		const result = await invoke<UdfInfo[]>('list_udfs');
		udfs.set(result);
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
	}
}

export async function selectWorkspace(id: string) {
	console.log('[selectWorkspace] Starting workspace switch to:', id);
	console.log('[selectWorkspace] Previous workspace ID:', get(selectedWorkspaceId));

	// IMPORTANT: Set the new workspace ID FIRST
	// The WorkspaceContainer's $effect will:
	// 1. Detect the change (previousWorkspaceId !== currentWorkspaceId)
	// 2. Save the OLD workspace's layout (using previousWorkspaceId)
	// 3. Update previousWorkspaceId to currentWorkspaceId
	// 4. Restore the NEW workspace's layout
	//
	// We do NOT call resetLayout() here - the WorkspaceContainer handles it
	// during the restore process. Calling it here would corrupt the old layout
	// before it can be saved.

	selectedWorkspaceId.set(id);
	console.log('[selectWorkspace] Set selectedWorkspaceId to:', id);

	selectedWellId.set(null);
	selectedCurveId.set(null);
	wells.set([]);
	curves.set([]);
	allWorkspaceCurves.set([]);
	curveData.set(null);
	executionResult.set(null);

	try {
		// Load wells and all curves for the workspace in parallel
		const [wellsResult, allCurvesResult] = await Promise.all([
			invoke<WellInfo[]>('list_wells', { workspaceId: id }),
			invoke<CurveInfoWithWell[]>('list_all_curves_for_workspace', { workspaceId: id })
		]);
		wells.set(wellsResult);
		allWorkspaceCurves.set(allCurvesResult);
		console.log('[selectWorkspace] Loaded wells and curves for workspace:', id);
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
		console.error('[selectWorkspace] Error loading workspace data:', e);
	}
}

/**
 * Load all curves for the current workspace (for curve selector dialogs)
 */
export async function loadAllWorkspaceCurves(): Promise<CurveInfoWithWell[]> {
	const workspaceId = get(selectedWorkspaceId);
	if (!workspaceId) {
		return [];
	}

	try {
		const result = await invoke<CurveInfoWithWell[]>('list_all_curves_for_workspace', { workspaceId });
		allWorkspaceCurves.set(result);
		return result;
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
		return [];
	}
}

export async function selectWell(id: string) {
	selectedWellId.set(id);
	selectedCurveId.set(null);
	curves.set([]);
	curveData.set(null);
	executionResult.set(null);

	try {
		const result = await invoke<CurveInfo[]>('list_curves', { wellId: id });
		curves.set(result);
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
	}
}

export async function selectCurve(id: string) {
	selectedCurveId.set(id);
	curveData.set(null);
	executionResult.set(null);
	isLoading.set(true);

	try {
		const result = await invoke<CurveData>('get_curve_data', { curveId: id });
		curveData.set(result);
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
	} finally {
		isLoading.set(false);
	}
}

export async function selectUdf(id: string) {
	selectedUdfId.set(id);
	parameterValues.set({});
	validationErrors.set({});
	executionResult.set(null);

	try {
		const result = await invoke<ParameterDefinition[]>('get_udf_parameters', { udfId: id });
		udfParameters.set(result);

		// Set defaults
		const defaults: Record<string, unknown> = {};
		for (const param of result) {
			if (param.default !== undefined && param.default !== null) {
				defaults[param.name] = param.default;
			}
		}
		parameterValues.set(defaults);
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
	}
}

export function setParameterValue(name: string, value: unknown) {
	parameterValues.update((values) => ({
		...values,
		[name]: value
	}));
	// Clear validation error for this field
	validationErrors.update((errors) => {
		const newErrors = { ...errors };
		delete newErrors[name];
		return newErrors;
	});
}

export async function executeUdf(saveResult = false) {
	// Get current values using Svelte's get() helper
	const currentWorkspaceId = get(selectedWorkspaceId);
	const currentWellId = get(selectedWellId);
	const currentUdfId = get(selectedUdfId);
	const currentParams = get(parameterValues);
	const currentUdf = get(selectedUdf);

	if (!currentWorkspaceId || !currentWellId || !currentUdfId) {
		error.set('Please select a workspace, well, and UDF');
		return;
	}

	// Get the UDF name for the output pane title
	const udfName = currentUdf?.name || 'UDF Output';

	isExecuting.set(true);
	error.set(null);
	executionResult.set(null);

	try {
		const result = await invoke<ExecuteUdfResult>('execute_udf', {
			request: {
				udf_id: currentUdfId,
				well_id: currentWellId,
				workspace_id: currentWorkspaceId,
				parameters: currentParams,
				save_result: saveResult
			}
		});

		executionResult.set(result);

		if (!result.success && result.error) {
			error.set(result.error);
		} else if (result.success && result.output_data) {
			// Create a table pane to display the output
			createOutputTablePane(result, udfName);
		}
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
	} finally {
		isExecuting.set(false);
	}
}

/**
 * Create a table pane to display UDF output data
 */
function createOutputTablePane(result: ExecuteUdfResult, udfName: string): void {
	if (!result.output_data || result.output_data.length === 0) {
		return;
	}

	// Create the table pane with output data
	const title = result.output_mnemonic
		? `${udfName}: ${result.output_mnemonic}`
		: `${udfName} Output`;

	workspaceManager.addPane(
		PaneType.Table,
		{
			options: {
				mnemonic: result.output_mnemonic,
				executionId: result.execution_id,
				udfName: udfName,
				data: result.output_data
			}
		},
		{
			title,
			position: 'right',
			activate: true
		}
	);
}

export function clearError() {
	error.set(null);
}

export async function saveOutputCurve() {
	// Get current values using Svelte's get() helper
	const currentResult = get(executionResult);
	const currentWellId = get(selectedWellId);
	const currentWorkspaceId = get(selectedWorkspaceId);

	if (!currentResult || !currentResult.success || !currentResult.output_data) {
		error.set('No successful execution result to save');
		return false;
	}

	if (!currentWellId || !currentWorkspaceId) {
		error.set('No well or workspace selected');
		return false;
	}

	isSaving.set(true);
	error.set(null);

	try {
		const saveResponse = await invoke<{ success: boolean; curve_id: string | null; error: string | null }>(
			'save_output_curve',
			{
				request: {
					execution_id: currentResult.execution_id,
					well_id: currentWellId,
					workspace_id: currentWorkspaceId,
					mnemonic: currentResult.output_mnemonic,
					output_data: currentResult.output_data
				}
			}
		);

		if (saveResponse.success) {
			// Update the execution result to show it's saved
			executionResult.update((r) => (r ? { ...r, saved: true, output_curve_id: saveResponse.curve_id } : r));
			// Refresh curves list to show the new curve
			const updatedCurves = await invoke<CurveInfo[]>('list_curves', { wellId: currentWellId });
			curves.set(updatedCurves);
			return true;
		} else {
			error.set(saveResponse.error || 'Failed to save curve');
			return false;
		}
	} catch (e) {
		error.set(e instanceof Error ? e.message : String(e));
		return false;
	} finally {
		isSaving.set(false);
	}
}
