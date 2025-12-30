/**
 * Layout Model - Tree-based layout structure for the pane system
 *
 * Architecture inspired by:
 * - GoldenLayout's ContentItem hierarchy
 * - VS Code SplitView
 * - Lumino DockPanel
 * - JupyterLab's Widget system
 *
 * See DFC-chart-implementation.md Section 12 for design details.
 */

import type { ChartConfig, ChartDataFrame } from '../charts/types';
import type { SegmentedCurveData } from '$lib/types';

// ============================================================================
// Layout Node Types
// ============================================================================

/**
 * LayoutNode - Union type for all nodes in the layout tree
 * Forms a recursive tree structure where:
 * - SplitNode divides space between children
 * - TabNode holds multiple panes as tabs
 * - PaneNode is the leaf containing actual content
 */
export type LayoutNode = SplitNode | TabNode | PaneNode;

/**
 * SplitNode - Divides space between children (horizontal or vertical)
 * Inspired by VS Code SplitView and Lumino SplitPanel
 */
export interface SplitNode {
	type: 'split';
	id: string;
	orientation: 'horizontal' | 'vertical';
	/** Child nodes - can be any LayoutNode type */
	children: LayoutNode[];
	/** Relative sizes of children (should sum to 1.0) */
	sizes: number[];
	/** Minimum size in pixels for this container */
	minSize?: number;
}

/**
 * TabNode - Contains multiple panes in tabs
 * Inspired by VS Code EditorGroup and GoldenLayout Stack
 */
export interface TabNode {
	type: 'tab';
	id: string;
	/** Only PaneNodes can be children of TabNode */
	children: PaneNode[];
	/** Index of the currently active tab */
	activeIndex: number;
}

/**
 * PaneNode - Leaf node containing a single pane
 * Inspired by JupyterLab's Widget and Lumino's ContentItem
 */
export interface PaneNode {
	type: 'pane';
	id: string;
	/** Pane type determines which factory creates it */
	paneType: PaneType;
	/** Pane-specific configuration */
	config: PaneConfig;
	/** Display title shown in tab/header */
	title: string;
	/** Icon identifier for the pane */
	icon?: string;
	/** Whether the pane can be closed */
	closable?: boolean;
}

// ============================================================================
// Pane Types
// ============================================================================

/**
 * PaneType - Enum of all supported pane types
 * Used by PaneFactory to create appropriate pane instances
 */
export enum PaneType {
	LineChart = 'line-chart',
	ScatterChart = 'scatter-chart',
	Histogram = 'histogram',
	CrossPlot = 'crossplot',
	WellLog = 'welllog',
	DataGrid = 'data-grid',
	LinkedCharts = 'linked-charts',
	Table = 'table',
	Empty = 'empty',
}

/**
 * PaneConfig - Configuration for a pane instance
 */
export interface PaneConfig {
	/** Data binding - which data frame to display */
	dataFrameId?: string;
	/** For well log displays - which curves to show */
	curveIds?: string[];
	/** Chart-specific configuration */
	chartConfig?: ChartConfig;
	/** Loaded chart data frame for rendering (legacy format with nulls) */
	chartData?: ChartDataFrame;
	/** Segmented chart data for rendering (new segment-based format - no nulls) */
	segmentedChartData?: SegmentedCurveData;
	/** Linked group for cursor/viewport sync */
	linkedGroup?: string;
	/** Additional pane-specific options */
	options?: Record<string, unknown>;
}

// ============================================================================
// Workspace Layout
// ============================================================================

/**
 * WorkspaceLayout - Complete workspace layout state
 * Inspired by GoldenLayout's ILayoutConfig
 */
export interface WorkspaceLayout {
	/** Root of the layout tree */
	root: LayoutNode;
	/** Version for migration support */
	version: number;
	/** ID of the currently active pane */
	activePaneId?: string;
	/** Timestamp of last modification */
	lastModified?: number;
}

// ============================================================================
// Add Pane Options
// ============================================================================

/**
 * Position options for adding a new pane
 */
export type AddPanePosition = 'left' | 'right' | 'top' | 'bottom' | 'tab';

/**
 * Options for adding a new pane
 */
export interface AddPaneOptions {
	/** Title for the new pane */
	title?: string;
	/** Reference pane ID for relative positioning */
	referenceId?: string;
	/** Position relative to reference pane */
	position?: AddPanePosition;
	/** Whether to activate the pane after adding */
	activate?: boolean;
	/** Initial size ratio (0-1) when splitting */
	size?: number;
}

// ============================================================================
// Layout Events
// ============================================================================

/**
 * WorkspaceEvent - All layout changes are communicated via events
 * Inspired by Theia's event-driven architecture and Lumino MessageLoop
 */
export type WorkspaceEvent =
	| { type: 'pane-added'; paneId: string; paneNode: PaneNode }
	| { type: 'pane-removed'; paneId: string }
	| { type: 'pane-moved'; paneId: string; fromContainerId: string; toContainerId: string }
	| { type: 'pane-config-updated'; paneId: string; config: PaneConfig }
	| { type: 'layout-changed'; layout: WorkspaceLayout }
	| { type: 'pane-activated'; paneId: string }
	| { type: 'split-resized'; splitId: string; sizes: number[] }
	| { type: 'tab-activated'; tabId: string; activeIndex: number };

// ============================================================================
// Pane Instance Interface
// ============================================================================

/**
 * IPaneInstance - Interface for pane instance lifecycle management
 * Inspired by JupyterLab Widget and Lumino Widget
 */
export interface IPaneInstance {
	/** Unique identifier matching PaneNode.id */
	readonly id: string;
	/** Pane type */
	readonly paneType: PaneType;
	/** Root HTML element for this pane */
	readonly element: HTMLElement | null;
	/** Whether the pane has been disposed */
	readonly isDisposed: boolean;

	/** Mount the pane into a container element */
	mount(container: HTMLElement): void;
	/** Update the pane with new configuration */
	update(config: PaneConfig): void;
	/** Called when pane is resized */
	resize(width: number, height: number): void;
	/** Called when pane becomes visible */
	show(): void;
	/** Called when pane becomes hidden */
	hide(): void;
	/** Called when pane receives focus */
	focus(): void;
	/** Called when pane loses focus */
	blur(): void;
	/** Dispose of the pane and cleanup resources */
	dispose(): void;
}

// ============================================================================
// Type Guards
// ============================================================================

/**
 * Type guard for SplitNode
 */
export function isSplitNode(node: LayoutNode): node is SplitNode {
	return node.type === 'split';
}

/**
 * Type guard for TabNode
 */
export function isTabNode(node: LayoutNode): node is TabNode {
	return node.type === 'tab';
}

/**
 * Type guard for PaneNode
 */
export function isPaneNode(node: LayoutNode): node is PaneNode {
	return node.type === 'pane';
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Generate a unique ID for layout nodes
 */
export function generateLayoutId(): string {
	return `pane-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

/**
 * Get default title for a pane type
 */
export function getDefaultPaneTitle(paneType: PaneType): string {
	switch (paneType) {
		case PaneType.LineChart:
			return 'Line Chart';
		case PaneType.ScatterChart:
			return 'Scatter Chart';
		case PaneType.Histogram:
			return 'Histogram';
		case PaneType.CrossPlot:
			return 'Cross Plot';
		case PaneType.WellLog:
			return 'Well Log';
		case PaneType.DataGrid:
			return 'Data Grid';
		case PaneType.LinkedCharts:
			return 'Linked Charts';
		case PaneType.Table:
			return 'Output Table';
		case PaneType.Empty:
		default:
			return 'New Pane';
	}
}

/**
 * Create a default empty pane node
 */
export function createEmptyPane(id?: string): PaneNode {
	return {
		type: 'pane',
		id: id ?? generateLayoutId(),
		paneType: PaneType.Empty,
		config: {},
		title: 'New Pane',
		closable: true,
	};
}

/**
 * Create a default workspace layout with single empty pane
 */
export function createDefaultLayout(): WorkspaceLayout {
	return {
		root: createEmptyPane(),
		version: 1,
		lastModified: Date.now(),
	};
}

/**
 * Deep clone a layout node (for immutable updates)
 */
export function cloneLayoutNode<T extends LayoutNode>(node: T): T {
	if (isPaneNode(node)) {
		return {
			...node,
			config: { ...node.config },
		} as T;
	}

	if (isTabNode(node)) {
		return {
			...node,
			children: node.children.map((child) => cloneLayoutNode(child)),
		} as T;
	}

	if (isSplitNode(node)) {
		return {
			...node,
			children: node.children.map((child) => cloneLayoutNode(child)),
			sizes: [...node.sizes],
		} as T;
	}

	return { ...node } as T;
}

/**
 * Normalize sizes array to sum to 1.0
 */
export function normalizeSizes(sizes: number[]): number[] {
	const sum = sizes.reduce((a, b) => a + b, 0);
	if (sum === 0) {
		// Equal distribution
		return sizes.map(() => 1 / sizes.length);
	}
	return sizes.map((s) => s / sum);
}

/**
 * Redistribute sizes when adding a new child
 */
export function redistributeSizes(currentSizes: number[], additionalCount: number): number[] {
	const totalItems = currentSizes.length + additionalCount;
	const newItemSize = 1 / totalItems;
	const scaleFactor = (totalItems - additionalCount) / totalItems;

	const newSizes = currentSizes.map((s) => s * scaleFactor);
	for (let i = 0; i < additionalCount; i++) {
		newSizes.push(newItemSize);
	}

	return normalizeSizes(newSizes);
}
