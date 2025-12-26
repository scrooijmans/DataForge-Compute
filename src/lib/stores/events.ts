/**
 * Event system for data change notifications
 *
 * Implements the event-driven architecture from the implementation plan.
 * Views subscribe to events and update when relevant data changes.
 */

// Event types matching the implementation plan
export type DataChangeType = 'curve' | 'well' | 'workspace' | 'tool_result' | 'selection';
export type ChangeKind = 'added' | 'updated' | 'removed' | 'refreshed';

export interface DataChangeEvent {
	dataType: DataChangeType;
	itemId: string;
	changeKind: ChangeKind;
	context?: Record<string, unknown>;
}

// View capabilities declaration
export interface ViewCapabilities {
	acceptedDataTypes: DataChangeType[];
	needsCurveData: boolean;
	respondsToSelection: boolean;
}

// Event listener type
type EventListener = (event: DataChangeEvent) => void;

/**
 * Central event bus for data change notifications.
 * Views register with their capabilities and receive filtered events.
 */
class DataEventBus {
	private listeners: Map<string, { callback: EventListener; capabilities: ViewCapabilities }> =
		new Map();
	private eventHistory: DataChangeEvent[] = [];
	private maxHistorySize = 100;

	/**
	 * Register a view to receive events.
	 */
	register(viewId: string, capabilities: ViewCapabilities, callback: EventListener): void {
		this.listeners.set(viewId, { callback, capabilities });
	}

	/**
	 * Unregister a view.
	 */
	unregister(viewId: string): void {
		this.listeners.delete(viewId);
	}

	/**
	 * Emit an event to all registered views.
	 * Views only receive events matching their capabilities.
	 */
	emit(event: DataChangeEvent): void {
		// Store in history
		this.eventHistory.push(event);
		if (this.eventHistory.length > this.maxHistorySize) {
			this.eventHistory.shift();
		}

		// Notify listeners with matching capabilities
		for (const [viewId, { callback, capabilities }] of this.listeners) {
			if (this.matchesCapabilities(event, capabilities)) {
				try {
					callback(event);
				} catch (error) {
					console.error(`Error in event handler for view ${viewId}:`, error);
				}
			}
		}
	}

	/**
	 * Check if an event matches a view's capabilities.
	 */
	private matchesCapabilities(event: DataChangeEvent, capabilities: ViewCapabilities): boolean {
		// Check if the view accepts this data type
		if (!capabilities.acceptedDataTypes.includes(event.dataType)) {
			return false;
		}

		// Selection events only go to views that respond to selection
		if (event.dataType === 'selection' && !capabilities.respondsToSelection) {
			return false;
		}

		return true;
	}

	/**
	 * Get recent event history.
	 */
	getHistory(count = 10): DataChangeEvent[] {
		return this.eventHistory.slice(-count);
	}

	/**
	 * Clear all listeners (for cleanup/testing).
	 */
	clear(): void {
		this.listeners.clear();
		this.eventHistory = [];
	}
}

// Singleton instance
export const eventBus = new DataEventBus();

// Convenience functions for common events
export function emitCurveAdded(curveId: string, context?: Record<string, unknown>): void {
	eventBus.emit({ dataType: 'curve', itemId: curveId, changeKind: 'added', context });
}

export function emitCurveUpdated(curveId: string, context?: Record<string, unknown>): void {
	eventBus.emit({ dataType: 'curve', itemId: curveId, changeKind: 'updated', context });
}

export function emitCurveRemoved(curveId: string): void {
	eventBus.emit({ dataType: 'curve', itemId: curveId, changeKind: 'removed' });
}

export function emitToolResultAdded(executionId: string, context?: Record<string, unknown>): void {
	eventBus.emit({ dataType: 'tool_result', itemId: executionId, changeKind: 'added', context });
}

export function emitSelectionChanged(selectionType: string, itemId: string): void {
	eventBus.emit({
		dataType: 'selection',
		itemId,
		changeKind: 'updated',
		context: { selectionType }
	});
}

export function emitWellSelected(wellId: string): void {
	emitSelectionChanged('well', wellId);
}

export function emitCurveSelected(curveId: string): void {
	emitSelectionChanged('curve', curveId);
}
