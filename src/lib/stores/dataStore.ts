/**
 * DataStore - Central data management with event emission
 *
 * This wraps data access and emits events when data changes,
 * allowing views to react to updates automatically.
 */

import { writable, derived, get, type Readable, type Writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type {
	DataForgeStatus,
	WorkspaceInfo,
	WellInfo,
	CurveInfo,
	CurveData,
	SegmentedCurveData,
	ProviderInfo,
	UdfInfo,
	ExecuteUdfResult
} from '$lib/types';
import { eventBus, emitCurveAdded, emitToolResultAdded } from './events';
import { selectionStore } from './selection';

// ============ Data Stores ============

// Connection status
export const connectionStatus: Writable<DataForgeStatus | null> = writable(null);
export const isConnected: Readable<boolean> = derived(
	connectionStatus,
	($status) => $status?.connected ?? false
);

// Workspace data
export const workspaces: Writable<WorkspaceInfo[]> = writable([]);

// Well data (for current workspace)
export const wells: Writable<WellInfo[]> = writable([]);

// Curve data (for current well)
export const curves: Writable<CurveInfo[]> = writable([]);

// Loaded curve data cache (legacy - array with nulls)
const curveDataCache: Writable<Map<string, CurveData>> = writable(new Map());

// Segmented curve data cache (new - segments without nulls)
const segmentedCurveCache: Writable<Map<string, SegmentedCurveData>> = writable(new Map());

// UDF providers and definitions
export const providers: Writable<ProviderInfo[]> = writable([]);
export const udfs: Writable<UdfInfo[]> = writable([]);

// Execution results
export const executionResults: Writable<Map<string, ExecuteUdfResult>> = writable(new Map());

// Loading states
export const isLoading: Writable<boolean> = writable(false);
export const loadingMessage: Writable<string | null> = writable(null);

// Errors
export const lastError: Writable<string | null> = writable(null);

// ============ Derived Stores ============

export const currentWorkspace: Readable<WorkspaceInfo | undefined> = derived(
	[workspaces, selectionStore],
	([$workspaces, $selection]) =>
		$workspaces.find((w) => w.id === $selection.activeWorkspaceId)
);

export const currentWell: Readable<WellInfo | undefined> = derived(
	[wells, selectionStore],
	([$wells, $selection]) => $wells.find((w) => w.id === $selection.activeWellId)
);

export const currentCurve: Readable<CurveInfo | undefined> = derived(
	[curves, selectionStore],
	([$curves, $selection]) => $curves.find((c) => c.id === $selection.activeCurveId)
);

export const currentCurveData: Readable<CurveData | null> = derived(
	[curveDataCache, selectionStore],
	([$cache, $selection]) =>
		$selection.activeCurveId ? $cache.get($selection.activeCurveId) ?? null : null
);

export const currentSegmentedCurveData: Readable<SegmentedCurveData | null> = derived(
	[segmentedCurveCache, selectionStore],
	([$cache, $selection]) =>
		$selection.activeCurveId ? $cache.get($selection.activeCurveId) ?? null : null
);

export const selectedCurvesData: Readable<Map<string, CurveData>> = derived(
	[curveDataCache, selectionStore],
	([$cache, $selection]) => {
		const result = new Map<string, CurveData>();
		for (const curveId of $selection.selectedCurveIds) {
			const data = $cache.get(curveId);
			if (data) {
				result.set(curveId, data);
			}
		}
		return result;
	}
);

export const selectedSegmentedCurvesData: Readable<Map<string, SegmentedCurveData>> = derived(
	[segmentedCurveCache, selectionStore],
	([$cache, $selection]) => {
		const result = new Map<string, SegmentedCurveData>();
		for (const curveId of $selection.selectedCurveIds) {
			const data = $cache.get(curveId);
			if (data) {
				result.set(curveId, data);
			}
		}
		return result;
	}
);

export const currentUdf: Readable<UdfInfo | undefined> = derived(
	[udfs, selectionStore],
	([$udfs, $selection]) => $udfs.find((u) => u.full_id === $selection.activeUdfId)
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

// ============ Actions ============

/**
 * Initialize the data store - load connection status and initial data.
 */
export async function initialize(): Promise<void> {
	isLoading.set(true);
	loadingMessage.set('Connecting to DataForge...');
	lastError.set(null);

	try {
		const status = await invoke<DataForgeStatus>('get_dataforge_status');
		connectionStatus.set(status);

		if (status.connected) {
			loadingMessage.set('Loading workspaces...');
			await loadWorkspaces();

			loadingMessage.set('Loading UDF providers...');
			await loadProviders();
			await loadUdfs();
		}
	} catch (error) {
		lastError.set(error instanceof Error ? error.message : String(error));
	} finally {
		isLoading.set(false);
		loadingMessage.set(null);
	}
}

/**
 * Load all workspaces.
 */
export async function loadWorkspaces(): Promise<void> {
	try {
		const result = await invoke<WorkspaceInfo[]>('list_workspaces');
		workspaces.set(result);
	} catch (error) {
		lastError.set(error instanceof Error ? error.message : String(error));
	}
}

/**
 * Load wells for a workspace.
 */
export async function loadWells(workspaceId: string): Promise<void> {
	try {
		const result = await invoke<WellInfo[]>('list_wells', { workspaceId });
		wells.set(result);
	} catch (error) {
		lastError.set(error instanceof Error ? error.message : String(error));
	}
}

/**
 * Load curves for a well.
 */
export async function loadCurves(wellId: string): Promise<void> {
	try {
		const result = await invoke<CurveInfo[]>('list_curves', { wellId });
		curves.set(result);
	} catch (error) {
		lastError.set(error instanceof Error ? error.message : String(error));
	}
}

/**
 * Load curve data by ID.
 */
export async function loadCurveData(curveId: string): Promise<CurveData | null> {
	console.log('[dataStore] loadCurveData called with curveId:', curveId);

	// Check cache first
	const cache = get(curveDataCache);
	if (cache.has(curveId)) {
		console.log('[dataStore] Returning cached data for:', curveId);
		return cache.get(curveId)!;
	}

	try {
		console.log('[dataStore] Invoking get_curve_data for:', curveId);
		const result = await invoke<CurveData>('get_curve_data', { curveId });
		console.log('[dataStore] get_curve_data result:', result ? `${result.data?.length ?? 0} points` : 'null');
		curveDataCache.update((c) => {
			c.set(curveId, result);
			return c;
		});
		return result;
	} catch (error) {
		console.error('[dataStore] get_curve_data error:', error);
		lastError.set(error instanceof Error ? error.message : String(error));
		return null;
	}
}

/**
 * Load segmented curve data by ID.
 *
 * This uses the new segment-based architecture where curves are represented
 * as a set of valid segments (no nulls), rather than arrays with missing data.
 * Segments are extracted at query time in the Rust backend.
 *
 * @param curveId - The curve ID to load
 * @param minSegmentPoints - Minimum points for a segment to be included (default: 2)
 * @returns Segmented curve data or null on error
 */
export async function loadSegmentedCurveData(
	curveId: string,
	minSegmentPoints: number = 2
): Promise<SegmentedCurveData | null> {
	console.log('[dataStore] loadSegmentedCurveData called with curveId:', curveId);

	// Check cache first
	const cache = get(segmentedCurveCache);
	if (cache.has(curveId)) {
		console.log('[dataStore] Returning cached segmented data for:', curveId);
		return cache.get(curveId)!;
	}

	try {
		console.log('[dataStore] Invoking get_curve_data_segmented for:', curveId);
		const result = await invoke<SegmentedCurveData>('get_curve_data_segmented', {
			curveId,
			minSegmentPoints
		});

		console.log(
			'[dataStore] get_curve_data_segmented result:',
			result
				? `${result.segments.length} segments, ${result.total_points} total points`
				: 'null'
		);

		// Cache the result
		segmentedCurveCache.update((c) => {
			c.set(curveId, result);
			return c;
		});

		return result;
	} catch (error) {
		console.error('[dataStore] get_curve_data_segmented error:', error);
		lastError.set(error instanceof Error ? error.message : String(error));
		return null;
	}
}

/**
 * Load providers.
 */
export async function loadProviders(): Promise<void> {
	try {
		const result = await invoke<ProviderInfo[]>('list_providers');
		providers.set(result);
	} catch (error) {
		lastError.set(error instanceof Error ? error.message : String(error));
	}
}

/**
 * Load UDFs.
 */
export async function loadUdfs(): Promise<void> {
	try {
		const result = await invoke<UdfInfo[]>('list_udfs');
		udfs.set(result);
	} catch (error) {
		lastError.set(error instanceof Error ? error.message : String(error));
	}
}

/**
 * Select a workspace and load its wells.
 */
export async function selectWorkspace(workspaceId: string): Promise<void> {
	selectionStore.setActiveWorkspace(workspaceId);
	wells.set([]);
	curves.set([]);
	await loadWells(workspaceId);
}

/**
 * Select a well and load its curves.
 */
export async function selectWell(wellId: string): Promise<void> {
	selectionStore.setActiveWell(wellId);
	curves.set([]);
	await loadCurves(wellId);
}

/**
 * Select a curve and load its data.
 */
export async function selectCurve(curveId: string): Promise<void> {
	selectionStore.setActiveCurve(curveId);
	await loadCurveData(curveId);
}

/**
 * Select a UDF.
 */
export function selectUdf(udfId: string): void {
	selectionStore.setActiveUdf(udfId);
}

/**
 * Store an execution result and emit event.
 */
export function addExecutionResult(executionId: string, result: ExecuteUdfResult): void {
	executionResults.update((m) => {
		m.set(executionId, result);
		return m;
	});

	// Emit event for views to react
	emitToolResultAdded(executionId, {
		success: result.success,
		udfId: result.output_mnemonic
	});

	// If a new curve was created, emit curve added event
	if (result.output_curve_id) {
		emitCurveAdded(result.output_curve_id, {
			mnemonic: result.output_mnemonic,
			executionId
		});
	}
}

/**
 * Clear error.
 */
export function clearError(): void {
	lastError.set(null);
}

/**
 * Clear curve data cache (both legacy and segmented).
 */
export function clearCurveCache(): void {
	curveDataCache.set(new Map());
	segmentedCurveCache.set(new Map());
}

/**
 * Clear only segmented curve cache.
 */
export function clearSegmentedCurveCache(): void {
	segmentedCurveCache.set(new Map());
}

/**
 * Get curve data from cache without loading.
 */
export function getCachedCurveData(curveId: string): CurveData | undefined {
	return get(curveDataCache).get(curveId);
}

/**
 * Get segmented curve data from cache without loading.
 */
export function getCachedSegmentedCurveData(curveId: string): SegmentedCurveData | undefined {
	return get(segmentedCurveCache).get(curveId);
}
