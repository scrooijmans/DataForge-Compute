/**
 * Selection Store - Synchronized selection state across all views
 *
 * Implements the selection synchronization from the implementation plan.
 * When selection changes in any view, all other views are notified.
 */

import { writable, derived, type Readable } from 'svelte/store';
import { emitWellSelected, emitCurveSelected, emitSelectionChanged } from './events';

// Selection state
export interface SelectionState {
	activeWorkspaceId: string | null;
	activeWellId: string | null;
	activeCurveId: string | null;
	selectedCurveIds: Set<string>;
	activeUdfId: string | null;
}

// Create the selection store
function createSelectionStore() {
	const { subscribe, set, update } = writable<SelectionState>({
		activeWorkspaceId: null,
		activeWellId: null,
		activeCurveId: null,
		selectedCurveIds: new Set(),
		activeUdfId: null
	});

	return {
		subscribe,

		// Workspace selection
		setActiveWorkspace(workspaceId: string | null) {
			update((state) => ({
				...state,
				activeWorkspaceId: workspaceId,
				// Clear downstream selections when workspace changes
				activeWellId: null,
				activeCurveId: null,
				selectedCurveIds: new Set()
			}));
			if (workspaceId) {
				emitSelectionChanged('workspace', workspaceId);
			}
		},

		// Well selection
		setActiveWell(wellId: string | null) {
			update((state) => ({
				...state,
				activeWellId: wellId,
				// Clear curve selection when well changes
				activeCurveId: null,
				selectedCurveIds: new Set()
			}));
			if (wellId) {
				emitWellSelected(wellId);
			}
		},

		// Single curve selection (for primary/active curve)
		setActiveCurve(curveId: string | null) {
			update((state) => ({
				...state,
				activeCurveId: curveId
			}));
			if (curveId) {
				emitCurveSelected(curveId);
			}
		},

		// Multi-curve selection
		selectCurve(curveId: string) {
			update((state) => {
				const newSet = new Set(state.selectedCurveIds);
				newSet.add(curveId);
				return { ...state, selectedCurveIds: newSet };
			});
		},

		deselectCurve(curveId: string) {
			update((state) => {
				const newSet = new Set(state.selectedCurveIds);
				newSet.delete(curveId);
				return { ...state, selectedCurveIds: newSet };
			});
		},

		toggleCurveSelection(curveId: string) {
			update((state) => {
				const newSet = new Set(state.selectedCurveIds);
				if (newSet.has(curveId)) {
					newSet.delete(curveId);
				} else {
					newSet.add(curveId);
				}
				return { ...state, selectedCurveIds: newSet };
			});
		},

		setSelectedCurves(curveIds: string[]) {
			update((state) => ({
				...state,
				selectedCurveIds: new Set(curveIds)
			}));
		},

		clearCurveSelection() {
			update((state) => ({
				...state,
				selectedCurveIds: new Set()
			}));
		},

		// UDF selection
		setActiveUdf(udfId: string | null) {
			update((state) => ({
				...state,
				activeUdfId: udfId
			}));
			if (udfId) {
				emitSelectionChanged('udf', udfId);
			}
		},

		// Reset all selections
		reset() {
			set({
				activeWorkspaceId: null,
				activeWellId: null,
				activeCurveId: null,
				selectedCurveIds: new Set(),
				activeUdfId: null
			});
		}
	};
}

export const selectionStore = createSelectionStore();

// Derived stores for convenience
export const activeWorkspaceId: Readable<string | null> = derived(
	selectionStore,
	($s) => $s.activeWorkspaceId
);

export const activeWellId: Readable<string | null> = derived(
	selectionStore,
	($s) => $s.activeWellId
);

export const activeCurveId: Readable<string | null> = derived(
	selectionStore,
	($s) => $s.activeCurveId
);

export const selectedCurveIds: Readable<Set<string>> = derived(
	selectionStore,
	($s) => $s.selectedCurveIds
);

export const activeUdfId: Readable<string | null> = derived(
	selectionStore,
	($s) => $s.activeUdfId
);

export const hasSelection: Readable<boolean> = derived(
	selectionStore,
	($s) => $s.activeWorkspaceId !== null || $s.activeWellId !== null || $s.activeCurveId !== null
);

export const hasCurveSelection: Readable<boolean> = derived(
	selectionStore,
	($s) => $s.selectedCurveIds.size > 0
);
