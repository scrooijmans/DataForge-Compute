/**
 * Chart Interaction Store - Manages chart interaction modes
 *
 * Architecture inspired by:
 * - QGIS: Mode-based editing (paint, select, etc.)
 * - Napari: Layer interaction modes with event propagation
 * - ParaView: Property-based control with signal updates
 * - SciChart: Cursor modifiers for interactive behaviors
 *
 * This store provides:
 * - Global interaction mode state
 * - Chart-type-aware mode availability
 * - Event-driven mode changes
 */

import { writable, derived, get, type Readable } from 'svelte/store';
import { PaneType } from '$lib/panes/layout-model';
import { workspaceManager } from '$lib/panes/workspace-manager';

// ============================================================================
// Interaction Mode Types
// ============================================================================

/**
 * CursorMode - Determines how mouse interactions behave on charts
 */
export type CursorMode = 'pointer' | 'crosshair' | 'zoom-in' | 'zoom-out' | 'pan';

/**
 * SelectionMode - Brush selection types for point selection
 */
export type SelectionMode = 'none' | 'rect' | 'polygon';

/**
 * ChartInteractionState - Global interaction state
 */
export interface ChartInteractionState {
	/** Current cursor/interaction mode */
	cursorMode: CursorMode;
	/** Current selection/brush mode */
	selectionMode: SelectionMode;
	/** Whether interaction tools are enabled */
	enabled: boolean;
}

/**
 * Chart types that support interaction modes
 */
export const INTERACTIVE_CHART_TYPES = new Set<PaneType>([
	PaneType.LineChart,
	PaneType.ScatterChart,
	PaneType.CrossPlot,
	PaneType.WellLog,
]);

/**
 * Mode configuration with metadata
 */
export interface CursorModeConfig {
	id: CursorMode;
	label: string;
	icon: string;
	tooltip: string;
	/** CSS cursor value for this mode */
	cursor: string;
}

/**
 * Selection mode configuration
 */
export interface SelectionModeConfig {
	id: SelectionMode;
	label: string;
	tooltip: string;
	/** CSS cursor value for this mode */
	cursor: string;
}

/**
 * Available selection modes (for crossplot only)
 */
export const SELECTION_MODES: SelectionModeConfig[] = [
	{
		id: 'rect',
		label: 'Rectangle Select',
		tooltip: 'Draw rectangle to select points',
		cursor: 'crosshair',
	},
	{
		id: 'polygon',
		label: 'Lasso Select',
		tooltip: 'Draw freeform polygon to select points',
		cursor: 'crosshair',
	},
];

/**
 * Available cursor modes with their configurations
 */
export const CURSOR_MODES: CursorModeConfig[] = [
	{
		id: 'pointer',
		label: 'Pointer',
		icon: 'cursor-default',
		tooltip: 'Default pointer - hover for tooltips',
		cursor: 'default',
	},
	{
		id: 'crosshair',
		label: 'Crosshair',
		icon: 'crosshairs',
		tooltip: 'Crosshair cursor - precise data inspection',
		cursor: 'crosshair',
	},
	{
		id: 'zoom-in',
		label: 'Zoom In',
		icon: 'magnify-plus',
		tooltip: 'Click or drag to zoom in',
		cursor: 'zoom-in',
	},
	{
		id: 'zoom-out',
		label: 'Zoom Out',
		icon: 'magnify-minus',
		tooltip: 'Click to zoom out',
		cursor: 'zoom-out',
	},
	{
		id: 'pan',
		label: 'Pan',
		icon: 'hand-back-left',
		tooltip: 'Click and drag to pan the view',
		cursor: 'grab',
	},
];

// ============================================================================
// Store Implementation
// ============================================================================

function createChartInteractionStore() {
	const initialState: ChartInteractionState = {
		cursorMode: 'pointer',
		selectionMode: 'none',
		enabled: true,
	};

	const { subscribe, set, update } = writable<ChartInteractionState>(initialState);

	return {
		subscribe,

		/**
		 * Set the cursor mode
		 */
		setCursorMode(mode: CursorMode): void {
			update((state) => ({
				...state,
				cursorMode: mode,
				// Clear selection mode when changing cursor mode
				selectionMode: 'none',
			}));
		},

		/**
		 * Set the selection mode (for brush selection)
		 */
		setSelectionMode(mode: SelectionMode): void {
			update((state) => ({
				...state,
				selectionMode: mode,
			}));
		},

		/**
		 * Toggle selection mode (rect -> polygon -> none -> rect)
		 */
		toggleSelectionMode(): void {
			update((state) => {
				const modes: SelectionMode[] = ['none', 'rect', 'polygon'];
				const currentIndex = modes.indexOf(state.selectionMode);
				const nextIndex = (currentIndex + 1) % modes.length;
				return {
					...state,
					selectionMode: modes[nextIndex],
				};
			});
		},

		/**
		 * Clear selection mode
		 */
		clearSelectionMode(): void {
			update((state) => ({
				...state,
				selectionMode: 'none',
			}));
		},

		/**
		 * Enable/disable interaction tools
		 */
		setEnabled(enabled: boolean): void {
			update((state) => ({
				...state,
				enabled,
			}));
		},

		/**
		 * Reset to default state
		 */
		reset(): void {
			set(initialState);
		},

		/**
		 * Get the current cursor mode configuration
		 */
		getCurrentModeConfig(): CursorModeConfig {
			const state = get({ subscribe });
			return CURSOR_MODES.find((m) => m.id === state.cursorMode) ?? CURSOR_MODES[0];
		},
	};
}

export const chartInteractionStore = createChartInteractionStore();

// ============================================================================
// Derived Stores
// ============================================================================

/**
 * Current cursor mode
 */
export const cursorMode: Readable<CursorMode> = derived(
	chartInteractionStore,
	($state) => $state.cursorMode
);

/**
 * Whether interactions are enabled
 */
export const interactionsEnabled: Readable<boolean> = derived(
	chartInteractionStore,
	($state) => $state.enabled
);

/**
 * Current cursor mode configuration
 */
export const currentModeConfig: Readable<CursorModeConfig> = derived(
	chartInteractionStore,
	($state) => CURSOR_MODES.find((m) => m.id === $state.cursorMode) ?? CURSOR_MODES[0]
);

/**
 * Current selection mode
 */
export const selectionMode: Readable<SelectionMode> = derived(
	chartInteractionStore,
	($state) => $state.selectionMode
);

/**
 * Whether the active pane supports interaction modes
 */
export const activePaneSupportsInteraction: Readable<boolean> = derived(
	workspaceManager.activePane,
	($activePane) => {
		if (!$activePane) return false;
		return INTERACTIVE_CHART_TYPES.has($activePane.paneType);
	}
);

/**
 * Active pane type (for conditional UI)
 */
export const activePaneType: Readable<PaneType | null> = derived(
	workspaceManager.activePane,
	($activePane) => $activePane?.paneType ?? null
);

/**
 * Whether the active pane supports brush selection (crossplot only)
 */
export const activePaneSupportsBrushSelection: Readable<boolean> = derived(
	workspaceManager.activePane,
	($activePane) => {
		if (!$activePane) return false;
		// Only crossplot supports brush selection
		return $activePane.paneType === PaneType.CrossPlot;
	}
);

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Get CSS cursor value for current mode
 */
export function getCursorStyle(mode: CursorMode): string {
	const config = CURSOR_MODES.find((m) => m.id === mode);
	return config?.cursor ?? 'default';
}

/**
 * Check if a pane type supports interaction modes
 */
export function supportsInteraction(paneType: PaneType): boolean {
	return INTERACTIVE_CHART_TYPES.has(paneType);
}

/**
 * Get available modes for a specific chart type
 * Can be extended later to return different modes per chart type
 */
export function getAvailableModes(paneType: PaneType): CursorModeConfig[] {
	if (!supportsInteraction(paneType)) {
		return [];
	}

	// For now, all interactive charts support all modes
	// This can be customized per chart type in the future
	return CURSOR_MODES;
}
