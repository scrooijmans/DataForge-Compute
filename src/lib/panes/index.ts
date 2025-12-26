/**
 * Pane System - Public API
 *
 * This module exports the windowing and pane system for managing
 * multiple charts and views in a dynamic workspace.
 *
 * See DFC-chart-implementation.md Section 12 for design details.
 */

// Layout Model Types
export {
	type LayoutNode,
	type SplitNode,
	type TabNode,
	type PaneNode,
	type PaneConfig,
	type WorkspaceLayout,
	type AddPaneOptions,
	type AddPanePosition,
	type WorkspaceEvent,
	type IPaneInstance,
	PaneType,
	isPaneNode,
	isSplitNode,
	isTabNode,
	generateLayoutId,
	getDefaultPaneTitle,
	createEmptyPane,
	createDefaultLayout,
	cloneLayoutNode,
	normalizeSizes,
	redistributeSizes,
} from './layout-model';

// Workspace Manager
export { workspaceManager } from './workspace-manager';

// Pane Factory
export {
	paneFactory,
	BasePaneInstance,
	EmptyPaneInstance,
	ChartPaneInstance,
	DataGridPaneInstance,
	registerChartPaneFactories,
	type PaneInstanceFactory,
} from './pane-factory';

// Selection Context
export {
	selectionContext,
	type SelectionType,
	type SelectionContext,
	type UdfSelection,
	type PaneSelection,
	type CurveSelection,
	type NoSelection,
} from './selection-context';

// Drag and Drop Context
export {
	dragDropContext,
	type DragState,
	type DropSegment,
	type DropZoneGeometry,
	type HitTestResult,
	DRAG_PREVIEW_Z_INDEX,
	DROP_INDICATOR_Z_INDEX,
	segmentToSplitDirection,
	getDragCursor,
} from './drag-drop-context';

// Chart Configurations
export {
	type AxisBinding,
	type SeriesStyle,
	type CommonChartOptions,
	type LineChartConfig,
	type ScatterChartConfig,
	type HistogramConfig,
	type CrossPlotConfig,
	type WellLogConfig,
	type WellLogTrackConfig,
	type WellLogCurveConfig,
	type ChartConfiguration,
	DEFAULT_SERIES_STYLE,
	DEFAULT_AXIS_BINDING,
	DEFAULT_COMMON_OPTIONS,
	createDefaultLineChartConfig,
	createDefaultScatterChartConfig,
	createDefaultHistogramConfig,
	createDefaultCrossPlotConfig,
	createDefaultWellLogConfig,
	CURVE_TYPE_RESTRICTIONS,
	isCurveTypeAllowed,
	getChartTypeName,
	COLOR_PRESETS,
} from './chart-configs';
