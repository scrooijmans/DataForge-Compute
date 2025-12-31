<script lang="ts">
	/**
	 * PaneContainer - Individual pane wrapper component
	 *
	 * Wraps a single pane with:
	 * - Header with title and toolbar (draggable)
	 * - Content area for the pane instance
	 * - Resize observer for responsive updates
	 * - Focus management
	 * - Selection context integration
	 * - Drag and drop support
	 *
	 * Design inspired by:
	 * - JupyterLab Widget (state preservation during drag)
	 * - VS Code EditorPane (drag preview and drop targets)
	 * - GoldenLayout ComponentContainer (segment-based hit testing)
	 *
	 * See DFC-chart-implementation.md Section 12 for design details.
	 */
	import { onMount, onDestroy } from 'svelte';
	import type { PaneNode } from '$lib/panes/layout-model';
	import { PaneType } from '$lib/panes/layout-model';
	import { workspaceManager } from '$lib/panes/workspace-manager';
	import { paneFactory } from '$lib/panes/pane-factory';
	import { selectionContext } from '$lib/panes/selection-context';
	import { dragDropContext, type DropSegment, DRAG_PREVIEW_Z_INDEX } from '$lib/panes/drag-drop-context';
	import type { IPaneInstance } from '$lib/panes/layout-model';
	import type { ChartConfiguration } from '$lib/panes/chart-configs';

	// Import chart components for rendering
	import EChartsChart from '$lib/components/charts/EChartsChart.svelte';
	import LinkedChartsView from '$lib/components/charts/LinkedChartsView.svelte';
	import TablePane from '$lib/components/panes/TablePane.svelte';
	import WellCorrelationPanel from '$lib/components/correlation/WellCorrelationPanel.svelte';

	// Import correlation types
	import type { CorrelationConfig, CorrelationCurveData } from '$lib/charts/correlation-types';
	import { calculateGlobalDepthRange } from '$lib/charts/correlation-types';

	interface Props {
		/** Pane node data */
		pane: PaneNode;
		/** Whether pane is visible (for tab containers) */
		visible?: boolean;
	}

	let { pane, visible = true }: Props = $props();

	/** Container element for drag registration */
	let containerEl: HTMLDivElement;

	/** Content container element */
	let contentEl: HTMLDivElement;

	/** Pane instance created by factory */
	let paneInstance: IPaneInstance | null = null;

	/** Current dimensions */
	let width = $state(0);
	let height = $state(0);

	/** Resize observer */
	let resizeObserver: ResizeObserver | null = null;

	/** Active pane ID from workspace manager */
	let activePaneId = workspaceManager.activePaneId;

	/** Check if this pane is focused */
	let isFocused = $derived($activePaneId === pane.id);

	/** Drag state */
	let dragState = dragDropContext.state;

	/** Whether this pane is being dragged */
	let isDragging = $derived($dragState.isDragging && $dragState.draggedPane?.id === pane.id);

	/** Whether this pane is a drop target */
	let isDropTarget = $derived($dragState.dropTargetId === pane.id);

	/** Current drop segment for this pane */
	let dropSegment = $derived<DropSegment | null>(isDropTarget ? $dragState.dropSegment : null);

	/** Extract chart config reactively from pane - with defensive null check */
	let chartConfig = $derived(pane?.config?.chartConfig as ChartConfiguration | undefined);

	/** Extract chart data reactively from pane - with defensive null check */
	let chartData = $derived(pane?.config?.chartData as import('$lib/charts/types').ChartDataFrame | undefined);

	/** Extract segmented chart data reactively from pane (new segment-based architecture) */
	let segmentedChartData = $derived(pane?.config?.segmentedChartData as import('$lib/types').SegmentedCurveData | undefined);

	/** Extract correlation config for correlation pane type */
	let correlationConfig = $derived.by(() => {
		if (pane?.paneType !== PaneType.Correlation) return null;
		const config = pane?.config?.chartConfig as unknown as CorrelationConfig | null;
		console.log('[PaneContainer] Extracted correlationConfig:', {
			paneId: pane?.id,
			hasConfig: !!config,
			wellCount: config?.wells?.length
		});
		return config;
	});

	/** Extract correlation curve data map */
	let correlationCurveData = $derived.by(() => {
		if (!correlationConfig) return new Map<string, CorrelationCurveData>();
		const data = pane?.config?.options?.correlationCurveData;
		console.log('[PaneContainer] Extracting correlationCurveData:', {
			paneId: pane?.id,
			hasData: !!data,
			isMap: data instanceof Map,
			rawDataType: typeof data,
			rawDataKeys: data instanceof Map ? Array.from(data.keys()) : 'not a map'
		});
		if (data instanceof Map) return data as Map<string, CorrelationCurveData>;
		return new Map<string, CorrelationCurveData>();
	});

	/** Calculate depth range for correlation panel */
	let correlationDepthRange = $derived.by(() => {
		if (!correlationConfig) return { min: 0, max: 1000 };
		if (correlationConfig.depthRange.autoScale) {
			const range = calculateGlobalDepthRange(correlationCurveData);
			console.log('[PaneContainer] Auto-calculated depth range:', range, 'from', correlationCurveData.size, 'curves');
			return range;
		}
		return {
			min: correlationConfig.depthRange.min ?? 0,
			max: correlationConfig.depthRange.max ?? 1000
		};
	});

	/** Extract series config for EChartsChart */
	let seriesConfig = $derived.by(() => {
		if (!chartConfig) return undefined;
		// Extract style from config to pass as series override
		const config = chartConfig as { style?: { color?: string; lineWidth?: number; pointSize?: number } };
		if (config.style) {
			return [{
				field: '', // Will apply to all series
				color: config.style.color,
				width: config.style.lineWidth,
				pointSize: config.style.pointSize
			}];
		}
		return undefined;
	});

	/** Extract showRegression from crossplot config */
	let showRegression = $derived.by(() => {
		if (!chartConfig) return false;
		const config = chartConfig as { type?: string; showRegression?: boolean };
		return config.type === 'crossplot' && config.showRegression === true;
	});

	/**
	 * Handle pane focus - also updates selection context
	 */
	function handleFocus(): void {
		if (!pane) return;
		workspaceManager.activatePane(pane.id);

		// Update selection context to show chart config in toolbar
		const config = pane.config?.chartConfig as ChartConfiguration | undefined;
		selectionContext.selectPane(pane.id, pane, config);
	}

	/**
	 * Handle pane click (in addition to focus)
	 */
	function handleClick(event: MouseEvent): void {
		// Don't override focus if clicking on toolbar buttons
		const target = event.target as HTMLElement;
		if (target.closest('.pane-toolbar')) return;

		handleFocus();
	}

	/**
	 * Handle drag start on header
	 */
	function handleDragStart(event: MouseEvent): void {
		// Only start drag from the header title area, not toolbar buttons
		const target = event.target as HTMLElement;
		if (target.closest('.pane-toolbar')) return;

		// Prevent text selection
		event.preventDefault();

		// Start drag operation
		dragDropContext.startDrag(pane, event.clientX, event.clientY);
	}

	/**
	 * Handle close action
	 */
	function handleClose(): void {
		if (pane.closable !== false) {
			workspaceManager.removePane(pane.id);
		}
	}

	/**
	 * Handle pane drop event
	 */
	function handlePaneDrop(event: CustomEvent): void {
		const { draggedPane, dropTargetId, dropSegment } = event.detail;

		// Only handle if we're the drop target
		if (dropTargetId !== pane.id || !draggedPane || !dropSegment) return;

		// Move the pane
		workspaceManager.movePane(draggedPane.id, pane.id, dropSegment);
	}

	/**
	 * Notify pane instance of visibility change
	 */
	$effect(() => {
		if (paneInstance) {
			if (visible) {
				paneInstance.show();
			} else {
				paneInstance.hide();
			}
		}
	});

	/**
	 * Notify pane instance of resize
	 */
	$effect(() => {
		if (paneInstance && width > 0 && height > 0) {
			paneInstance.resize(width, height);
		}
	});

	onMount(() => {
		// Register pane for hit testing
		if (containerEl) {
			dragDropContext.registerPane(pane.id, containerEl);
		}

		// Setup resize observer
		resizeObserver = new ResizeObserver((entries) => {
			for (const entry of entries) {
				width = entry.contentRect.width;
				height = entry.contentRect.height;
			}
		});

		if (contentEl) {
			resizeObserver.observe(contentEl);
		}

		// Create pane instance for non-Svelte pane types
		if (pane.paneType === PaneType.Empty || pane.paneType === PaneType.DataGrid) {
			paneInstance = paneFactory.create(pane);
			if (contentEl && paneInstance) {
				paneInstance.mount(contentEl);
			}
		}

		// Listen for drop events
		document.addEventListener('pane-drop', handlePaneDrop as EventListener);

		return () => {
			// Unregister pane
			dragDropContext.unregisterPane(pane.id);

			resizeObserver?.disconnect();
			if (paneInstance) {
				paneFactory.dispose(pane.id);
				paneInstance = null;
			}

			document.removeEventListener('pane-drop', handlePaneDrop as EventListener);
		};
	});
</script>

<div
	bind:this={containerEl}
	class="pane-container"
	class:focused={isFocused}
	class:hidden={!visible}
	class:dragging={isDragging}
	class:drop-target={isDropTarget}
	class:drop-left={dropSegment === 'left'}
	class:drop-right={dropSegment === 'right'}
	class:drop-top={dropSegment === 'top'}
	class:drop-bottom={dropSegment === 'bottom'}
	class:drop-center={dropSegment === 'center'}
	class:drop-tab={dropSegment === 'tab'}
	data-pane-id={pane.id}
	data-pane-type={pane.paneType}
	role="region"
	aria-label={pane.title}
	tabindex="0"
	onfocus={handleFocus}
	onclick={handleClick}
>
	<!-- Pane Header (Draggable) -->
	<div
		class="pane-header"
		onmousedown={handleDragStart}
		role="toolbar"
		aria-label="Pane header - drag to move"
	>
		<div class="pane-title">
			<!-- Drag handle icon -->
			<span class="pane-drag-handle" aria-hidden="true">
				<svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor" opacity="0.4">
					<circle cx="3" cy="3" r="1.5" />
					<circle cx="9" cy="3" r="1.5" />
					<circle cx="3" cy="9" r="1.5" />
					<circle cx="9" cy="9" r="1.5" />
				</svg>
			</span>
			{#if pane.icon}
				<span class="pane-icon">{pane.icon}</span>
			{/if}
			<span class="pane-title-text">{pane.title}</span>
		</div>

		<div class="pane-toolbar">
			<!-- Close button -->
			{#if pane.closable !== false}
				<button
					class="pane-toolbar-button pane-toolbar-close"
					aria-label="Close pane"
					title="Close"
					onclick={handleClose}
				>
					<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M2 2L10 10M10 2L2 10" />
					</svg>
				</button>
			{/if}
		</div>
	</div>

	<!-- Pane Content -->
	<div class="pane-content" bind:this={contentEl}>
		{#if pane.paneType === PaneType.LineChart || pane.paneType === PaneType.ScatterChart || pane.paneType === PaneType.CrossPlot || pane.paneType === PaneType.WellLog || pane.paneType === PaneType.Histogram}
			<!-- Render chart using EChartsChart component -->
			<!-- chartConfig, chartData, and segmentedChartData are extracted reactively via $derived -->
			<div class="pane-chart-wrapper">
				<EChartsChart
					id={pane.id}
					data={chartData ?? null}
					segmentedData={segmentedChartData ?? null}
					type={pane.paneType === PaneType.Histogram ? 'histogram' : (pane.paneType === PaneType.WellLog ? 'welllog' : (pane.paneType === PaneType.ScatterChart || pane.paneType === PaneType.CrossPlot ? 'scatter' : 'line'))}
					height={height - 36}
					title={chartConfig?.title}
					series={seriesConfig}
					showCursor={chartConfig?.showCursor ?? true}
					enableZoom={chartConfig?.enableZoom ?? true}
					linkGroup={(chartConfig as { linkedGroup?: string } | undefined)?.linkedGroup ?? pane.config?.linkedGroup}
					{showRegression}
				/>
			</div>
		{:else if pane.paneType === PaneType.LinkedCharts}
			<!-- Render linked charts view -->
			<LinkedChartsView tracks={[]} height={height - 36} />
		{:else if pane.paneType === PaneType.Correlation}
			<!-- Render well correlation panel -->
			{#if correlationConfig}
				<WellCorrelationPanel
					config={correlationConfig}
					curveData={correlationCurveData}
					depthRange={correlationDepthRange}
					height={height - 36}
				/>
			{:else}
				<div class="pane-empty">
					<div class="pane-empty-content">
						<div class="pane-empty-icon">📊</div>
						<p class="pane-empty-text">Configure wells and curves in the settings panel</p>
					</div>
				</div>
			{/if}
		{:else if pane.paneType === PaneType.Table}
			<!-- Render table pane for UDF output -->
			<TablePane
				config={{
					mnemonic: pane.config?.options?.mnemonic as string | undefined,
					unit: pane.config?.options?.unit as string | undefined,
					executionId: pane.config?.options?.executionId as string | undefined,
					udfName: pane.config?.options?.udfName as string | undefined,
					data: pane.config?.options?.data as import('$lib/types').CurveDataPoint[] | undefined
				}}
				height={height - 36}
			/>
		{:else if pane.paneType === PaneType.Empty}
			<!-- Empty pane placeholder - handled by paneInstance -->
		{:else}
			<!-- Other pane types - handled by paneInstance -->
		{/if}
	</div>

	<!-- Drop Zone Indicator Overlays -->
	{#if isDropTarget && dropSegment}
		<div class="drop-zone-indicator drop-zone-{dropSegment}"></div>
	{/if}
</div>

<style>
	.pane-container {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		overflow: hidden;
		background: hsl(var(--background));
		border: 1px solid hsl(var(--border));
		border-radius: 4px;
		transition: border-color 0.15s ease, opacity 0.15s ease;
		position: relative;
	}

	.pane-container.focused {
		border-color: hsl(var(--primary));
	}

	.pane-container.hidden {
		display: none;
	}

	.pane-container:focus {
		outline: none;
	}

	/* Dragging state */
	.pane-container.dragging {
		opacity: 0.5;
		border-style: dashed;
	}

	/* Drop target states */
	.pane-container.drop-target {
		border-color: hsl(var(--primary));
		border-width: 2px;
	}

	.pane-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 36px;
		padding: 0 8px;
		background: hsl(var(--muted));
		border-bottom: 1px solid hsl(var(--border));
		flex-shrink: 0;
		cursor: grab;
		user-select: none;
	}

	.pane-header:active {
		cursor: grabbing;
	}

	.pane-title {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 13px;
		font-weight: 500;
		color: hsl(var(--foreground));
		overflow: hidden;
	}

	.pane-drag-handle {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 16px;
		height: 16px;
		color: hsl(var(--muted-foreground));
	}

	.pane-icon {
		flex-shrink: 0;
		width: 16px;
		height: 16px;
	}

	.pane-title-text {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.pane-toolbar {
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.pane-toolbar-button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		border: none;
		background: transparent;
		border-radius: 4px;
		cursor: pointer;
		color: hsl(var(--muted-foreground));
		transition:
			background-color 0.15s ease,
			color 0.15s ease;
	}

	.pane-toolbar-button:hover {
		background: hsl(var(--accent));
		color: hsl(var(--foreground));
	}

	.pane-toolbar-button:focus-visible {
		outline: 2px solid hsl(var(--ring));
		outline-offset: -2px;
	}

	.pane-toolbar-close:hover {
		background: hsl(var(--destructive) / 0.1);
		color: hsl(var(--destructive));
	}

	.pane-content {
		flex: 1;
		overflow: hidden;
		position: relative;
		background: hsl(var(--background));
	}

	.pane-chart-wrapper {
		width: 100%;
		height: 100%;
	}

	/* Drop zone indicator overlays */
	.drop-zone-indicator {
		position: absolute;
		background: hsl(var(--primary) / 0.15);
		border: 2px dashed hsl(var(--primary));
		pointer-events: none;
		z-index: 10;
	}

	.drop-zone-left {
		left: 0;
		top: 36px;
		width: 50%;
		bottom: 0;
	}

	.drop-zone-right {
		right: 0;
		top: 36px;
		width: 50%;
		bottom: 0;
	}

	.drop-zone-top {
		left: 0;
		top: 36px;
		right: 0;
		height: calc(50% - 18px);
	}

	.drop-zone-bottom {
		left: 0;
		bottom: 0;
		right: 0;
		height: calc(50% - 18px);
	}

	.drop-zone-center {
		left: 0;
		top: 36px;
		right: 0;
		bottom: 0;
		background: hsl(var(--accent) / 0.2);
		border: 2px solid hsl(var(--primary));
	}

	.drop-zone-tab {
		left: 0;
		top: 0;
		right: 0;
		height: 36px;
		background: hsl(var(--primary) / 0.1);
		border-bottom: 3px solid hsl(var(--primary));
		border-left: none;
		border-right: none;
		border-top: none;
	}

	/* Empty pane styles */
	:global(.pane-empty) {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		height: 100%;
		background: hsl(var(--background));
	}

	:global(.pane-empty-content) {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		padding: 24px;
		text-align: center;
		color: hsl(var(--muted-foreground));
	}

	:global(.pane-empty-icon) {
		width: 48px;
		height: 48px;
		border: 2px dashed hsl(var(--border));
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 24px;
		color: hsl(var(--muted-foreground));
	}

	:global(.pane-empty-text) {
		font-size: 13px;
		max-width: 200px;
	}
</style>
