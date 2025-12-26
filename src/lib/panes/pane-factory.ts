/**
 * PaneFactory - Factory pattern for creating pane instances
 *
 * Design patterns used:
 * - JupyterLab WidgetFactory: Factory for creating typed widgets
 * - VS Code EditorService: Reference counting for shared instances
 * - GoldenLayout ComponentRegistry: Registration-based creation
 *
 * See DFC-chart-implementation.md Section 12 for design details.
 */

import {
	type PaneNode,
	type PaneConfig,
	type IPaneInstance,
	PaneType,
} from './layout-model';

// ============================================================================
// Pane Factory Types
// ============================================================================

/**
 * Factory function type for creating pane instances
 */
export type PaneInstanceFactory = (paneId: string, config: PaneConfig) => IPaneInstance;

/**
 * Pane factory registration info
 */
interface PaneFactoryRegistration {
	paneType: PaneType;
	factory: PaneInstanceFactory;
	displayName: string;
	icon?: string;
}

// ============================================================================
// Pane Factory Implementation
// ============================================================================

class PaneFactoryImpl {
	/** Registered factories by pane type */
	private factories = new Map<PaneType, PaneFactoryRegistration>();

	/** Active pane instances (for reference counting) */
	private instances = new Map<string, { instance: IPaneInstance; refCount: number }>();

	constructor() {
		// Register built-in pane types
		this.registerBuiltInTypes();
	}

	/**
	 * Register a pane factory
	 */
	register(
		paneType: PaneType,
		factory: PaneInstanceFactory,
		displayName: string,
		icon?: string
	): void {
		this.factories.set(paneType, {
			paneType,
			factory,
			displayName,
			icon,
		});
	}

	/**
	 * Check if a pane type is registered
	 */
	isRegistered(paneType: PaneType): boolean {
		return this.factories.has(paneType);
	}

	/**
	 * Get all registered pane types
	 */
	getRegisteredTypes(): PaneType[] {
		return Array.from(this.factories.keys());
	}

	/**
	 * Get display info for a pane type
	 */
	getTypeInfo(paneType: PaneType): { displayName: string; icon?: string } | null {
		const registration = this.factories.get(paneType);
		if (!registration) return null;
		return {
			displayName: registration.displayName,
			icon: registration.icon,
		};
	}

	/**
	 * Create a pane instance
	 */
	create(paneNode: PaneNode): IPaneInstance {
		const registration = this.factories.get(paneNode.paneType);
		if (!registration) {
			console.warn(`No factory registered for pane type: ${paneNode.paneType}`);
			// Fall back to empty pane
			const emptyReg = this.factories.get(PaneType.Empty);
			if (emptyReg) {
				return emptyReg.factory(paneNode.id, paneNode.config);
			}
			throw new Error(`No factory registered for pane type: ${paneNode.paneType}`);
		}

		const instance = registration.factory(paneNode.id, paneNode.config);

		// Track instance for lifecycle management
		this.instances.set(paneNode.id, { instance, refCount: 1 });

		return instance;
	}

	/**
	 * Get an existing pane instance
	 */
	getInstance(paneId: string): IPaneInstance | null {
		const entry = this.instances.get(paneId);
		return entry?.instance ?? null;
	}

	/**
	 * Increment reference count for a pane instance
	 */
	retain(paneId: string): void {
		const entry = this.instances.get(paneId);
		if (entry) {
			entry.refCount++;
		}
	}

	/**
	 * Decrement reference count and dispose if zero
	 */
	release(paneId: string): void {
		const entry = this.instances.get(paneId);
		if (!entry) return;

		entry.refCount--;
		if (entry.refCount <= 0) {
			entry.instance.dispose();
			this.instances.delete(paneId);
		}
	}

	/**
	 * Dispose a pane instance immediately
	 */
	dispose(paneId: string): void {
		const entry = this.instances.get(paneId);
		if (entry) {
			entry.instance.dispose();
			this.instances.delete(paneId);
		}
	}

	/**
	 * Dispose all pane instances
	 */
	disposeAll(): void {
		for (const [, entry] of this.instances) {
			entry.instance.dispose();
		}
		this.instances.clear();
	}

	/**
	 * Get count of active instances
	 */
	getActiveInstanceCount(): number {
		return this.instances.size;
	}

	/**
	 * Register built-in pane types
	 */
	private registerBuiltInTypes(): void {
		// Empty pane (placeholder)
		this.register(
			PaneType.Empty,
			(id, config) => new EmptyPaneInstance(id, config),
			'Empty Pane',
			'square'
		);

		// Chart panes are registered separately by the chart system
		// This allows for lazy loading of chart components
	}
}

// ============================================================================
// Base Pane Instance Class
// ============================================================================

/**
 * BasePaneInstance - Abstract base class for pane instances
 * Provides common lifecycle management
 */
export abstract class BasePaneInstance implements IPaneInstance {
	readonly id: string;
	abstract readonly paneType: PaneType;

	protected _element: HTMLElement | null = null;
	protected _isDisposed = false;
	protected _config: PaneConfig;

	constructor(id: string, config: PaneConfig) {
		this.id = id;
		this._config = config;
	}

	get element(): HTMLElement | null {
		return this._element;
	}

	get isDisposed(): boolean {
		return this._isDisposed;
	}

	abstract mount(container: HTMLElement): void;

	update(config: PaneConfig): void {
		this._config = { ...this._config, ...config };
	}

	resize(_width: number, _height: number): void {
		// Override in subclasses if needed
	}

	show(): void {
		if (this._element) {
			this._element.style.display = '';
		}
	}

	hide(): void {
		if (this._element) {
			this._element.style.display = 'none';
		}
	}

	focus(): void {
		this._element?.focus();
	}

	blur(): void {
		this._element?.blur();
	}

	dispose(): void {
		if (this._isDisposed) return;

		this._isDisposed = true;
		if (this._element && this._element.parentNode) {
			this._element.parentNode.removeChild(this._element);
		}
		this._element = null;
	}
}

// ============================================================================
// Empty Pane Instance
// ============================================================================

/**
 * EmptyPaneInstance - Placeholder pane for empty slots
 */
export class EmptyPaneInstance extends BasePaneInstance {
	readonly paneType = PaneType.Empty;

	mount(container: HTMLElement): void {
		this._element = document.createElement('div');
		this._element.className = 'pane-empty';
		this._element.innerHTML = `
			<div class="pane-empty-content">
				<div class="pane-empty-icon">+</div>
				<div class="pane-empty-text">Drop content here or use the toolbar to add a view</div>
			</div>
		`;
		container.appendChild(this._element);
	}
}

// ============================================================================
// Chart Pane Instance
// ============================================================================

/**
 * ChartPaneInstance - Pane instance for chart visualizations
 * This is a placeholder - the actual implementation would integrate with ECharts
 */
export class ChartPaneInstance extends BasePaneInstance {
	readonly paneType: PaneType;

	constructor(id: string, paneType: PaneType, config: PaneConfig) {
		super(id, config);
		this.paneType = paneType;
	}

	mount(container: HTMLElement): void {
		this._element = document.createElement('div');
		this._element.className = 'pane-chart';
		this._element.style.width = '100%';
		this._element.style.height = '100%';

		// Chart component will be mounted here by Svelte
		// This is a placeholder for the chart container
		this._element.dataset.paneId = this.id;
		this._element.dataset.paneType = this.paneType;

		container.appendChild(this._element);
	}

	resize(width: number, height: number): void {
		// Notify chart of resize
		if (this._element) {
			this._element.style.width = `${width}px`;
			this._element.style.height = `${height}px`;
			// Dispatch custom event for chart component to handle
			this._element.dispatchEvent(
				new CustomEvent('pane-resize', {
					detail: { width, height },
					bubbles: true,
				})
			);
		}
	}
}

// ============================================================================
// Data Grid Pane Instance
// ============================================================================

/**
 * DataGridPaneInstance - Pane instance for data grid views
 */
export class DataGridPaneInstance extends BasePaneInstance {
	readonly paneType = PaneType.DataGrid;

	mount(container: HTMLElement): void {
		this._element = document.createElement('div');
		this._element.className = 'pane-data-grid';
		this._element.style.width = '100%';
		this._element.style.height = '100%';

		// Data grid component will be mounted here
		this._element.dataset.paneId = this.id;
		this._element.dataset.paneType = this.paneType;

		container.appendChild(this._element);
	}
}

// ============================================================================
// Factory Registration Helpers
// ============================================================================

/**
 * Register chart pane factories
 * Call this after the chart system is initialized
 */
export function registerChartPaneFactories(factory: PaneFactoryImpl): void {
	factory.register(
		PaneType.LineChart,
		(id, config) => new ChartPaneInstance(id, PaneType.LineChart, config),
		'Line Chart',
		'chart-line'
	);

	factory.register(
		PaneType.ScatterChart,
		(id, config) => new ChartPaneInstance(id, PaneType.ScatterChart, config),
		'Scatter Chart',
		'chart-scatter'
	);

	factory.register(
		PaneType.Histogram,
		(id, config) => new ChartPaneInstance(id, PaneType.Histogram, config),
		'Histogram',
		'chart-bar'
	);

	factory.register(
		PaneType.CrossPlot,
		(id, config) => new ChartPaneInstance(id, PaneType.CrossPlot, config),
		'Cross Plot',
		'chart-crossplot'
	);

	factory.register(
		PaneType.WellLog,
		(id, config) => new ChartPaneInstance(id, PaneType.WellLog, config),
		'Well Log',
		'chart-welllog'
	);

	factory.register(
		PaneType.LinkedCharts,
		(id, config) => new ChartPaneInstance(id, PaneType.LinkedCharts, config),
		'Linked Charts',
		'chart-linked'
	);

	factory.register(
		PaneType.DataGrid,
		(id, config) => new DataGridPaneInstance(id, config),
		'Data Grid',
		'table'
	);
}

// ============================================================================
// Singleton Export
// ============================================================================

export const paneFactory = new PaneFactoryImpl();

// Initialize chart pane factories
registerChartPaneFactories(paneFactory);
