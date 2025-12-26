/**
 * ChartManager - Coordinates multiple chart instances for linked interactions
 *
 * Provides:
 * - Chart group registration using ECharts connect() API
 * - Cursor synchronization across linked charts
 * - Viewport synchronization for coordinated pan/zoom
 * - Event emission for cross-chart communication
 *
 * See DFC-chart-implementation.md Section 6 for architecture details.
 */

import * as echarts from 'echarts/core';
import type { Viewport, CursorPosition } from './types';

/**
 * Chart registration info
 */
interface ChartRegistration {
	id: string;
	instance: echarts.ECharts;
	groupId: string;
	options: ChartRegistrationOptions;
}

/**
 * Options for chart registration
 */
export interface ChartRegistrationOptions {
	/** Sync cursor/tooltip across charts in group */
	syncCursor?: boolean;
	/** Sync viewport (pan/zoom) across charts in group */
	syncViewport?: boolean;
	/** Axis to sync for viewport (default: 'y' for depth-based well logs) */
	syncAxis?: 'x' | 'y' | 'both';
}

/**
 * Chart group info
 */
interface ChartGroup {
	id: string;
	chartIds: Set<string>;
	options: ChartRegistrationOptions;
}

/**
 * Cursor event data for cross-chart sync
 */
export interface CursorEvent {
	sourceChartId: string;
	groupId: string;
	position: CursorPosition | null;
	dataIndex?: number;
}

/**
 * Viewport event data for cross-chart sync
 */
export interface ViewportEvent {
	sourceChartId: string;
	groupId: string;
	viewport: Viewport;
}

type CursorListener = (event: CursorEvent) => void;
type ViewportListener = (event: ViewportEvent) => void;

/**
 * ChartManager singleton for coordinating linked charts
 */
class ChartManagerImpl {
	/** Registered charts by ID */
	private charts = new Map<string, ChartRegistration>();

	/** Chart groups by group ID */
	private groups = new Map<string, ChartGroup>();

	/** Cursor event listeners */
	private cursorListeners = new Set<CursorListener>();

	/** Viewport event listeners */
	private viewportListeners = new Set<ViewportListener>();

	/** Flag to prevent recursive updates */
	private isUpdating = false;

	/**
	 * Register a chart instance with the manager
	 */
	registerChart(
		id: string,
		instance: echarts.ECharts,
		groupId: string,
		options: ChartRegistrationOptions = {}
	): void {
		// Default options
		const opts: ChartRegistrationOptions = {
			syncCursor: true,
			syncViewport: false,
			syncAxis: 'y',
			...options
		};

		// Store registration
		this.charts.set(id, { id, instance, groupId, options: opts });

		// Add to group
		if (!this.groups.has(groupId)) {
			this.groups.set(groupId, {
				id: groupId,
				chartIds: new Set(),
				options: opts
			});
		}
		this.groups.get(groupId)!.chartIds.add(id);

		// Use ECharts native connect for cursor sync
		// This automatically syncs tooltips/axis pointers
		if (opts.syncCursor) {
			echarts.connect(groupId);
		}

		// Setup event listeners for this chart
		this.setupChartListeners(id, instance, groupId, opts);
	}

	/**
	 * Unregister a chart from the manager
	 */
	unregisterChart(id: string): void {
		const registration = this.charts.get(id);
		if (!registration) return;

		const { groupId } = registration;

		// Remove from group
		const group = this.groups.get(groupId);
		if (group) {
			group.chartIds.delete(id);
			if (group.chartIds.size === 0) {
				this.groups.delete(groupId);
				// Disconnect empty group
				echarts.disconnect(groupId);
			}
		}

		// Remove chart
		this.charts.delete(id);
	}

	/**
	 * Get all charts in a group
	 */
	getGroupCharts(groupId: string): echarts.ECharts[] {
		const group = this.groups.get(groupId);
		if (!group) return [];

		return Array.from(group.chartIds)
			.map((id) => this.charts.get(id)?.instance)
			.filter((c): c is echarts.ECharts => c !== undefined);
	}

	/**
	 * Setup event listeners for a chart
	 */
	private setupChartListeners(
		chartId: string,
		instance: echarts.ECharts,
		groupId: string,
		options: ChartRegistrationOptions
	): void {
		// DataZoom event for viewport sync
		if (options.syncViewport) {
			instance.on('datazoom', () => {
				if (this.isUpdating) return;
				this.handleDataZoom(chartId, instance, groupId, options);
			});
		}

		// Mouse events for custom cursor handling (beyond ECharts connect)
		instance.on('mousemove', (params: unknown) => {
			if (this.isUpdating) return;
			this.handleMouseMove(chartId, groupId, params);
		});

		instance.on('mouseout', () => {
			if (this.isUpdating) return;
			this.handleMouseOut(chartId, groupId);
		});
	}

	/**
	 * Handle dataZoom event for viewport sync
	 */
	private handleDataZoom(
		sourceChartId: string,
		sourceInstance: echarts.ECharts,
		groupId: string,
		options: ChartRegistrationOptions
	): void {
		const option = sourceInstance.getOption() as Record<string, unknown>;
		const dataZoom = option.dataZoom as Array<{ start?: number; end?: number; startValue?: number; endValue?: number }> | undefined;

		if (!dataZoom || dataZoom.length === 0) return;

		// Get viewport from dataZoom state
		const xZoom = dataZoom.find((dz, i) => {
			const xAxisIndex = (dz as Record<string, unknown>).xAxisIndex;
			return xAxisIndex === 0 || i === 0;
		});
		const yZoom = dataZoom.find((dz, i) => {
			const yAxisIndex = (dz as Record<string, unknown>).yAxisIndex;
			return yAxisIndex === 0 || i === 1;
		});

		// Sync to other charts in group
		this.isUpdating = true;
		try {
			const group = this.groups.get(groupId);
			if (!group) return;

			for (const chartId of group.chartIds) {
				if (chartId === sourceChartId) continue;

				const chart = this.charts.get(chartId);
				if (!chart) continue;

				const syncOption: Record<string, unknown> = { dataZoom: [] };
				const zoomUpdates: Array<Record<string, unknown>> = [];

				// Sync based on axis configuration
				if (options.syncAxis === 'x' || options.syncAxis === 'both') {
					if (xZoom) {
						zoomUpdates.push({
							xAxisIndex: 0,
							start: xZoom.start,
							end: xZoom.end,
							startValue: xZoom.startValue,
							endValue: xZoom.endValue
						});
					}
				}
				if (options.syncAxis === 'y' || options.syncAxis === 'both') {
					if (yZoom) {
						zoomUpdates.push({
							yAxisIndex: 0,
							start: yZoom.start,
							end: yZoom.end,
							startValue: yZoom.startValue,
							endValue: yZoom.endValue
						});
					}
				}

				if (zoomUpdates.length > 0) {
					syncOption.dataZoom = zoomUpdates;
					chart.instance.setOption(syncOption);
				}
			}
		} finally {
			this.isUpdating = false;
		}

		// Emit viewport event
		const viewport = this.getChartViewport(sourceInstance);
		if (viewport) {
			this.emitViewportEvent({
				sourceChartId,
				groupId,
				viewport
			});
		}
	}

	/**
	 * Handle mouse move for cursor events
	 */
	private handleMouseMove(chartId: string, groupId: string, params: unknown): void {
		if (!params || typeof params !== 'object') return;

		const p = params as Record<string, unknown>;
		const data = p.data as [number, number] | undefined;
		const dataIndex = p.dataIndex as number | undefined;

		if (!data) return;

		this.emitCursorEvent({
			sourceChartId: chartId,
			groupId,
			position: {
				x: data[0],
				y: data[1],
				dataIndex
			},
			dataIndex
		});
	}

	/**
	 * Handle mouse out for cursor events
	 */
	private handleMouseOut(chartId: string, groupId: string): void {
		this.emitCursorEvent({
			sourceChartId: chartId,
			groupId,
			position: null
		});
	}

	/**
	 * Get current viewport from chart
	 */
	private getChartViewport(instance: echarts.ECharts): Viewport | null {
		try {
			const option = instance.getOption() as Record<string, unknown>;
			const xAxis = (option.xAxis as Array<{ min?: number; max?: number }> | undefined)?.[0];
			const yAxis = (option.yAxis as Array<{ min?: number; max?: number }> | undefined)?.[0];

			if (!xAxis || !yAxis) return null;

			return {
				xMin: xAxis.min ?? 0,
				xMax: xAxis.max ?? 1,
				yMin: yAxis.min ?? 0,
				yMax: yAxis.max ?? 1
			};
		} catch {
			return null;
		}
	}

	/**
	 * Emit cursor event to listeners
	 */
	private emitCursorEvent(event: CursorEvent): void {
		for (const listener of this.cursorListeners) {
			listener(event);
		}
	}

	/**
	 * Emit viewport event to listeners
	 */
	private emitViewportEvent(event: ViewportEvent): void {
		for (const listener of this.viewportListeners) {
			listener(event);
		}
	}

	/**
	 * Subscribe to cursor events
	 */
	onCursor(listener: CursorListener): () => void {
		this.cursorListeners.add(listener);
		return () => this.cursorListeners.delete(listener);
	}

	/**
	 * Subscribe to viewport events
	 */
	onViewport(listener: ViewportListener): () => void {
		this.viewportListeners.add(listener);
		return () => this.viewportListeners.delete(listener);
	}

	/**
	 * Programmatically sync cursor to a position across a group
	 */
	syncCursorToGroup(groupId: string, position: CursorPosition | null): void {
		const group = this.groups.get(groupId);
		if (!group) return;

		this.isUpdating = true;
		try {
			for (const chartId of group.chartIds) {
				const chart = this.charts.get(chartId);
				if (!chart) continue;

				if (position === null) {
					chart.instance.dispatchAction({ type: 'hideTip' });
				} else {
					// Convert to pixel coordinates
					const pixel = chart.instance.convertToPixel('grid', [position.x, position.y]);
					if (pixel) {
						chart.instance.dispatchAction({
							type: 'showTip',
							x: pixel[0],
							y: pixel[1]
						});
					}
				}
			}
		} finally {
			this.isUpdating = false;
		}
	}

	/**
	 * Get chart instance by ID
	 */
	getChart(id: string): echarts.ECharts | undefined {
		return this.charts.get(id)?.instance;
	}

	/**
	 * Check if a group exists
	 */
	hasGroup(groupId: string): boolean {
		return this.groups.has(groupId);
	}

	/**
	 * Get group size
	 */
	getGroupSize(groupId: string): number {
		return this.groups.get(groupId)?.chartIds.size ?? 0;
	}

	/**
	 * Clear all registrations
	 */
	clear(): void {
		// Disconnect all groups
		for (const groupId of this.groups.keys()) {
			echarts.disconnect(groupId);
		}
		this.charts.clear();
		this.groups.clear();
		this.cursorListeners.clear();
		this.viewportListeners.clear();
	}
}

// Singleton instance
export const chartManager = new ChartManagerImpl();
