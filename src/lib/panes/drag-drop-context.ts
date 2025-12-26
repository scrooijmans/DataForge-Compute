/**
 * Drag and Drop Context for Pane Management
 *
 * Implements drag and drop following best practices from:
 * - GoldenLayout: Segment-based hit testing and tree restructuring
 * - VS Code: State management and atomic layout operations
 * - JupyterLab: Widget state preservation during moves
 *
 * Key principles:
 * 1. State preservation: Pane state is never destroyed during drag
 * 2. Atomic operations: Layout changes happen in single transactions
 * 3. Visual feedback: Clear drop zone indicators during drag
 * 4. Hit testing: Segment-based detection for drop targets
 */

import { writable, get } from 'svelte/store';
import type { Writable, Readable } from 'svelte/store';
import type { PaneNode } from './layout-model';

// ============================================================================
// Types
// ============================================================================

/**
 * Drop zone segment - where on a pane the drop would occur
 * Inspired by GoldenLayout's Stack.Segment
 */
export type DropSegment = 'left' | 'right' | 'top' | 'bottom' | 'center' | 'tab';

/**
 * Current drag state
 */
export interface DragState {
	/** Whether a drag operation is in progress */
	isDragging: boolean;
	/** The pane being dragged */
	draggedPane: PaneNode | null;
	/** Current drop target pane ID */
	dropTargetId: string | null;
	/** Which segment of the drop target */
	dropSegment: DropSegment | null;
	/** Mouse position during drag */
	mouseX: number;
	mouseY: number;
}

/**
 * Drop zone geometry for visual feedback
 */
export interface DropZoneGeometry {
	x: number;
	y: number;
	width: number;
	height: number;
	segment: DropSegment;
}

/**
 * Hit test result
 */
export interface HitTestResult {
	paneId: string;
	segment: DropSegment;
	bounds: DOMRect;
}

// ============================================================================
// Constants
// ============================================================================

/** Threshold for edge detection (pixels from edge) */
const EDGE_THRESHOLD = 40;

/** Threshold for header detection (pixels from top) */
const HEADER_THRESHOLD = 36;

/** Z-index for drag preview */
export const DRAG_PREVIEW_Z_INDEX = 1000;

/** Z-index for drop indicator */
export const DROP_INDICATOR_Z_INDEX = 999;

// ============================================================================
// Drag Drop Context Implementation
// ============================================================================

class DragDropContextImpl {
	/** Current drag state */
	private _state: Writable<DragState>;

	/** Registry of pane elements for hit testing */
	private _paneElements: Map<string, HTMLElement> = new Map();

	/** Expose state as readable */
	readonly state: Readable<DragState>;

	constructor() {
		this._state = writable<DragState>({
			isDragging: false,
			draggedPane: null,
			dropTargetId: null,
			dropSegment: null,
			mouseX: 0,
			mouseY: 0,
		});

		this.state = { subscribe: this._state.subscribe };
	}

	// =========================================================================
	// Public API
	// =========================================================================

	/**
	 * Register a pane element for hit testing
	 */
	registerPane(paneId: string, element: HTMLElement): void {
		this._paneElements.set(paneId, element);
	}

	/**
	 * Unregister a pane element
	 */
	unregisterPane(paneId: string): void {
		this._paneElements.delete(paneId);
	}

	/**
	 * Start a drag operation
	 */
	startDrag(pane: PaneNode, mouseX: number, mouseY: number): void {
		this._state.set({
			isDragging: true,
			draggedPane: pane,
			dropTargetId: null,
			dropSegment: null,
			mouseX,
			mouseY,
		});

		// Add global event listeners
		document.addEventListener('mousemove', this.handleMouseMove);
		document.addEventListener('mouseup', this.handleMouseUp);

		// Prevent text selection during drag
		document.body.style.userSelect = 'none';
		document.body.style.cursor = 'grabbing';
	}

	/**
	 * Update drag position and perform hit testing
	 */
	updateDrag(mouseX: number, mouseY: number): void {
		const currentState = get(this._state);
		if (!currentState.isDragging || !currentState.draggedPane) return;

		// Perform hit testing
		const hitResult = this.hitTest(mouseX, mouseY, currentState.draggedPane.id);

		this._state.set({
			...currentState,
			mouseX,
			mouseY,
			dropTargetId: hitResult?.paneId ?? null,
			dropSegment: hitResult?.segment ?? null,
		});
	}

	/**
	 * End the drag operation
	 */
	endDrag(): { dropTargetId: string | null; dropSegment: DropSegment | null; draggedPane: PaneNode | null } {
		const currentState = get(this._state);
		const result = {
			dropTargetId: currentState.dropTargetId,
			dropSegment: currentState.dropSegment,
			draggedPane: currentState.draggedPane,
		};

		// Reset state
		this._state.set({
			isDragging: false,
			draggedPane: null,
			dropTargetId: null,
			dropSegment: null,
			mouseX: 0,
			mouseY: 0,
		});

		// Remove global event listeners
		document.removeEventListener('mousemove', this.handleMouseMove);
		document.removeEventListener('mouseup', this.handleMouseUp);

		// Restore user select
		document.body.style.userSelect = '';
		document.body.style.cursor = '';

		return result;
	}

	/**
	 * Cancel the drag operation
	 */
	cancelDrag(): void {
		this.endDrag();
	}

	/**
	 * Get current drag state
	 */
	getState(): DragState {
		return get(this._state);
	}

	/**
	 * Calculate drop zone geometry for visual indicator
	 */
	getDropZoneGeometry(paneId: string, segment: DropSegment): DropZoneGeometry | null {
		const element = this._paneElements.get(paneId);
		if (!element) return null;

		const bounds = element.getBoundingClientRect();
		const headerHeight = HEADER_THRESHOLD;

		switch (segment) {
			case 'left':
				return {
					x: bounds.left,
					y: bounds.top,
					width: bounds.width / 2,
					height: bounds.height,
					segment,
				};
			case 'right':
				return {
					x: bounds.left + bounds.width / 2,
					y: bounds.top,
					width: bounds.width / 2,
					height: bounds.height,
					segment,
				};
			case 'top':
				return {
					x: bounds.left,
					y: bounds.top + headerHeight,
					width: bounds.width,
					height: (bounds.height - headerHeight) / 2,
					segment,
				};
			case 'bottom':
				return {
					x: bounds.left,
					y: bounds.top + headerHeight + (bounds.height - headerHeight) / 2,
					width: bounds.width,
					height: (bounds.height - headerHeight) / 2,
					segment,
				};
			case 'center':
			case 'tab':
				return {
					x: bounds.left,
					y: bounds.top,
					width: bounds.width,
					height: bounds.height,
					segment,
				};
			default:
				return null;
		}
	}

	// =========================================================================
	// Private Methods
	// =========================================================================

	/**
	 * Handle mouse move during drag
	 */
	private handleMouseMove = (event: MouseEvent): void => {
		this.updateDrag(event.clientX, event.clientY);
	};

	/**
	 * Handle mouse up to end drag
	 */
	private handleMouseUp = (): void => {
		// The actual drop handling is done by the drop target component
		// This just ensures cleanup happens
		const state = get(this._state);
		if (state.isDragging) {
			// Emit a custom event for the drop target to handle
			const dropEvent = new CustomEvent('pane-drop', {
				detail: {
					draggedPane: state.draggedPane,
					dropTargetId: state.dropTargetId,
					dropSegment: state.dropSegment,
				},
				bubbles: true,
			});
			document.dispatchEvent(dropEvent);

			this.endDrag();
		}
	};

	/**
	 * Perform hit testing to find drop target and segment
	 * Inspired by GoldenLayout's segment-based hit testing
	 */
	private hitTest(mouseX: number, mouseY: number, excludePaneId: string): HitTestResult | null {
		for (const [paneId, element] of this._paneElements) {
			// Don't allow dropping on self
			if (paneId === excludePaneId) continue;

			const bounds = element.getBoundingClientRect();

			// Check if mouse is within this pane
			if (
				mouseX >= bounds.left &&
				mouseX <= bounds.right &&
				mouseY >= bounds.top &&
				mouseY <= bounds.bottom
			) {
				// Determine which segment
				const segment = this.determineSegment(mouseX, mouseY, bounds);
				return { paneId, segment, bounds };
			}
		}

		return null;
	}

	/**
	 * Determine which segment of the pane the mouse is over
	 */
	private determineSegment(mouseX: number, mouseY: number, bounds: DOMRect): DropSegment {
		const relX = mouseX - bounds.left;
		const relY = mouseY - bounds.top;
		const width = bounds.width;
		const height = bounds.height;

		// Check if over header (tab area)
		if (relY < HEADER_THRESHOLD) {
			return 'tab';
		}

		// Calculate distances to edges
		const distLeft = relX;
		const distRight = width - relX;
		const distTop = relY - HEADER_THRESHOLD;
		const distBottom = height - relY;

		// Check if within edge threshold
		const nearLeft = distLeft < EDGE_THRESHOLD;
		const nearRight = distRight < EDGE_THRESHOLD;
		const nearTop = distTop < EDGE_THRESHOLD;
		const nearBottom = distBottom < EDGE_THRESHOLD;

		// Determine segment based on closest edge
		if (nearLeft && distLeft <= Math.min(distRight, distTop, distBottom)) {
			return 'left';
		}
		if (nearRight && distRight <= Math.min(distLeft, distTop, distBottom)) {
			return 'right';
		}
		if (nearTop && distTop <= Math.min(distLeft, distRight, distBottom)) {
			return 'top';
		}
		if (nearBottom && distBottom <= Math.min(distLeft, distRight, distTop)) {
			return 'bottom';
		}

		// Default to center (replace/swap)
		return 'center';
	}
}

// ============================================================================
// Singleton Export
// ============================================================================

export const dragDropContext = new DragDropContextImpl();

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Convert drop segment to split direction for workspace manager
 */
export function segmentToSplitDirection(segment: DropSegment): 'left' | 'right' | 'top' | 'bottom' | null {
	switch (segment) {
		case 'left':
		case 'right':
		case 'top':
		case 'bottom':
			return segment;
		default:
			return null;
	}
}

/**
 * Get cursor style for drag state
 */
export function getDragCursor(segment: DropSegment | null): string {
	if (!segment) return 'no-drop';
	switch (segment) {
		case 'left':
		case 'right':
			return 'col-resize';
		case 'top':
		case 'bottom':
			return 'row-resize';
		case 'center':
		case 'tab':
			return 'copy';
		default:
			return 'move';
	}
}
