/**
 * Selection Context - Tracks the currently selected/focused object
 *
 * This store manages which object is currently selected in the workspace,
 * enabling context-sensitive toolbars and property panels.
 *
 * Selection types:
 * - 'udf': A UDF is selected from the toolbox
 * - 'pane': A pane/chart is focused in the workspace
 * - 'curve': A curve is selected in the data tree
 * - 'none': Nothing is selected
 */

import { writable, derived, get } from 'svelte/store';
import type { Writable, Readable } from 'svelte/store';
import type { PaneNode } from './layout-model';
import type { UdfInfo } from '$lib/types';
import type { ChartConfiguration } from './chart-configs';

// ============================================================================
// Selection Types
// ============================================================================

export type SelectionType = 'udf' | 'pane' | 'curve' | 'none';

/**
 * UDF selection context
 */
export interface UdfSelection {
	type: 'udf';
	udf: UdfInfo;
}

/**
 * Pane/Chart selection context
 */
export interface PaneSelection {
	type: 'pane';
	paneId: string;
	paneNode: PaneNode;
	chartConfig?: ChartConfiguration;
}

/**
 * Curve selection context
 */
export interface CurveSelection {
	type: 'curve';
	curveId: string;
	mnemonic: string;
	wellId: string;
}

/**
 * No selection
 */
export interface NoSelection {
	type: 'none';
}

export type SelectionContext = UdfSelection | PaneSelection | CurveSelection | NoSelection;

// ============================================================================
// Selection Context Store
// ============================================================================

class SelectionContextManager {
	private _selection: Writable<SelectionContext>;

	/** Current selection state */
	readonly selection: Readable<SelectionContext>;

	/** Derived: current selection type */
	readonly selectionType: Readable<SelectionType>;

	/** Derived: is a UDF selected */
	readonly isUdfSelected: Readable<boolean>;

	/** Derived: is a pane selected */
	readonly isPaneSelected: Readable<boolean>;

	/** Derived: is a curve selected */
	readonly isCurveSelected: Readable<boolean>;

	/** Derived: selected UDF (if any) */
	readonly selectedUdf: Readable<UdfInfo | null>;

	/** Derived: selected pane (if any) */
	readonly selectedPane: Readable<PaneSelection | null>;

	/** Derived: selected curve (if any) */
	readonly selectedCurve: Readable<CurveSelection | null>;

	constructor() {
		this._selection = writable<SelectionContext>({ type: 'none' });
		this.selection = { subscribe: this._selection.subscribe };

		this.selectionType = derived(this._selection, ($sel) => $sel.type);

		this.isUdfSelected = derived(this._selection, ($sel) => $sel.type === 'udf');
		this.isPaneSelected = derived(this._selection, ($sel) => $sel.type === 'pane');
		this.isCurveSelected = derived(this._selection, ($sel) => $sel.type === 'curve');

		this.selectedUdf = derived(this._selection, ($sel) =>
			$sel.type === 'udf' ? $sel.udf : null
		);

		this.selectedPane = derived(this._selection, ($sel) =>
			$sel.type === 'pane' ? $sel : null
		);

		this.selectedCurve = derived(this._selection, ($sel) =>
			$sel.type === 'curve' ? $sel : null
		);
	}

	/**
	 * Select a UDF
	 */
	selectUdf(udf: UdfInfo): void {
		this._selection.set({ type: 'udf', udf });
	}

	/**
	 * Select a pane/chart
	 */
	selectPane(paneId: string, paneNode: PaneNode, chartConfig?: ChartConfiguration): void {
		this._selection.set({
			type: 'pane',
			paneId,
			paneNode,
			chartConfig,
		});
	}

	/**
	 * Update chart config for currently selected pane
	 */
	updatePaneConfig(chartConfig: ChartConfiguration): void {
		const current = get(this._selection);
		if (current.type === 'pane') {
			this._selection.set({
				...current,
				chartConfig,
			});
		}
	}

	/**
	 * Select a curve
	 */
	selectCurve(curveId: string, mnemonic: string, wellId: string): void {
		this._selection.set({
			type: 'curve',
			curveId,
			mnemonic,
			wellId,
		});
	}

	/**
	 * Clear selection
	 */
	clearSelection(): void {
		this._selection.set({ type: 'none' });
	}

	/**
	 * Get current selection
	 */
	getSelection(): SelectionContext {
		return get(this._selection);
	}

	/**
	 * Check if a specific pane is selected
	 */
	isPaneSelectedById(paneId: string): boolean {
		const current = get(this._selection);
		return current.type === 'pane' && current.paneId === paneId;
	}

	/**
	 * Check if a specific UDF is selected
	 */
	isUdfSelectedById(udfId: string): boolean {
		const current = get(this._selection);
		return current.type === 'udf' && current.udf.full_id === udfId;
	}
}

// ============================================================================
// Singleton Export
// ============================================================================

export const selectionContext = new SelectionContextManager();
