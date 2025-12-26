/**
 * WorkspaceManager - Single source of truth for workspace layout
 *
 * Design patterns used:
 * - VS Code ModelService: Single source of truth for layout state
 * - JupyterLab LayoutRestorer: Persistence and restoration
 * - Lumino DockPanel: Tree-based layout structure
 * - GoldenLayout EventEmitter: Event-driven updates
 * - Svelte Stores: Reactive state management
 *
 * See DFC-chart-implementation.md Section 12 for design details.
 */

import { writable, derived, get } from 'svelte/store';
import type { Writable, Readable } from 'svelte/store';
import {
	type WorkspaceLayout,
	type LayoutNode,
	type PaneNode,
	type SplitNode,
	type TabNode,
	type PaneConfig,
	type AddPaneOptions,
	type AddPanePosition,
	type WorkspaceEvent,
	PaneType,
	isPaneNode,
	isSplitNode,
	isTabNode,
	generateLayoutId,
	getDefaultPaneTitle,
	createDefaultLayout,
	cloneLayoutNode,
	normalizeSizes,
	redistributeSizes,
} from './layout-model';

// ============================================================================
// Workspace Manager Implementation
// ============================================================================

class WorkspaceManagerImpl {
	/** Current layout state */
	private _layout: Writable<WorkspaceLayout>;

	/** Event emitter for layout changes */
	private _events: Writable<WorkspaceEvent | null>;

	/** Expose layout as readable store */
	readonly layout: Readable<WorkspaceLayout>;

	/** Expose events for subscribers */
	readonly events: Readable<WorkspaceEvent | null>;

	/** Derived store for all pane nodes in the layout */
	readonly panes: Readable<PaneNode[]>;

	/** Derived store for active pane */
	readonly activePane: Readable<PaneNode | null>;

	/** Derived store for active pane ID */
	readonly activePaneId: Readable<string | null>;

	constructor() {
		this._layout = writable<WorkspaceLayout>(createDefaultLayout());
		this._events = writable<WorkspaceEvent | null>(null);

		// Create readable proxies
		this.layout = { subscribe: this._layout.subscribe };
		this.events = { subscribe: this._events.subscribe };

		// Derived stores
		this.panes = derived(this._layout, ($layout) => this.collectPanes($layout.root));

		this.activePane = derived(this._layout, ($layout) => {
			if (!$layout.activePaneId) return null;
			return this.findPane($layout.root, $layout.activePaneId);
		});

		this.activePaneId = derived(this._layout, ($layout) => $layout.activePaneId ?? null);
	}

	// =========================================================================
	// Public API
	// =========================================================================

	/**
	 * Add a new pane to the workspace
	 */
	addPane(paneType: PaneType, config: PaneConfig = {}, options: AddPaneOptions = {}): string {
		const paneId = generateLayoutId();
		const paneNode: PaneNode = {
			type: 'pane',
			id: paneId,
			paneType,
			config,
			title: options.title ?? getDefaultPaneTitle(paneType),
			closable: true,
		};

		this._layout.update((layout) => {
			const newLayout = this.insertPane(layout, paneNode, options);
			return {
				...newLayout,
				lastModified: Date.now(),
			};
		});

		this.emitEvent({ type: 'pane-added', paneId, paneNode });

		if (options.activate !== false) {
			this.activatePane(paneId);
		}

		return paneId;
	}

	/**
	 * Remove a pane from the workspace
	 */
	removePane(paneId: string): void {
		const currentLayout = get(this._layout);
		const pane = this.findPane(currentLayout.root, paneId);
		if (!pane) return;

		this._layout.update((layout) => {
			const newLayout = this.removePaneFromLayout(layout, paneId);
			return {
				...newLayout,
				lastModified: Date.now(),
			};
		});

		this.emitEvent({ type: 'pane-removed', paneId });
	}

	/**
	 * Activate a pane (bring to focus)
	 */
	activatePane(paneId: string): void {
		this._layout.update((layout) => {
			const pane = this.findPane(layout.root, paneId);
			if (!pane) return layout;

			// Also update tab activeIndex if pane is in a tab container
			const newRoot = this.updateTabActiveIndex(layout.root, paneId);

			return {
				...layout,
				root: newRoot,
				activePaneId: paneId,
				lastModified: Date.now(),
			};
		});

		this.emitEvent({ type: 'pane-activated', paneId });
	}

	/**
	 * Update pane configuration
	 */
	updatePaneConfig(paneId: string, config: Partial<PaneConfig>): void {
		this._layout.update((layout) => {
			const newRoot = this.updatePaneInLayout(layout.root, paneId, (pane) => ({
				...pane,
				config: { ...pane.config, ...config },
			}));

			return {
				...layout,
				root: newRoot,
				lastModified: Date.now(),
			};
		});

		const pane = this.findPane(get(this._layout).root, paneId);
		if (pane) {
			this.emitEvent({ type: 'pane-config-updated', paneId, config: pane.config });
		}
	}

	/**
	 * Update pane title
	 */
	updatePaneTitle(paneId: string, title: string): void {
		this._layout.update((layout) => {
			const newRoot = this.updatePaneInLayout(layout.root, paneId, (pane) => ({
				...pane,
				title,
			}));

			return {
				...layout,
				root: newRoot,
				lastModified: Date.now(),
			};
		});
	}

	/**
	 * Resize a split container
	 */
	resizeSplit(splitId: string, sizes: number[]): void {
		const normalizedSizes = normalizeSizes(sizes);

		this._layout.update((layout) => {
			const newRoot = this.updateNodeInLayout(layout.root, splitId, (node) => {
				if (!isSplitNode(node)) return node;
				return { ...node, sizes: normalizedSizes };
			});

			return {
				...layout,
				root: newRoot,
				lastModified: Date.now(),
			};
		});

		this.emitEvent({ type: 'split-resized', splitId, sizes: normalizedSizes });
	}

	/**
	 * Split a pane horizontally or vertically
	 */
	splitPane(
		paneId: string,
		direction: 'left' | 'right' | 'top' | 'bottom',
		newPaneType: PaneType = PaneType.Empty,
		newPaneConfig: PaneConfig = {}
	): string {
		const newPaneId = generateLayoutId();
		const newPane: PaneNode = {
			type: 'pane',
			id: newPaneId,
			paneType: newPaneType,
			config: newPaneConfig,
			title: getDefaultPaneTitle(newPaneType),
			closable: true,
		};

		this._layout.update((layout) => {
			const newRoot = this.splitPaneInLayout(layout.root, paneId, direction, newPane);
			return {
				...layout,
				root: newRoot,
				lastModified: Date.now(),
			};
		});

		this.emitEvent({ type: 'pane-added', paneId: newPaneId, paneNode: newPane });

		return newPaneId;
	}

	/**
	 * Add a pane as a tab to an existing pane
	 */
	addTab(existingPaneId: string, paneType: PaneType, config: PaneConfig = {}): string {
		return this.addPane(paneType, config, {
			referenceId: existingPaneId,
			position: 'tab',
		});
	}

	/**
	 * Move a pane to a new location
	 * Implements atomic move operation following VS Code pattern
	 */
	movePane(
		paneId: string,
		targetPaneId: string,
		position: 'left' | 'right' | 'top' | 'bottom' | 'tab' | 'center'
	): void {
		const currentLayout = get(this._layout);
		const pane = this.findPane(currentLayout.root, paneId);
		const targetPane = this.findPane(currentLayout.root, targetPaneId);

		if (!pane || !targetPane || paneId === targetPaneId) return;

		// Handle center as swap operation
		if (position === 'center') {
			this.swapPanes(paneId, targetPaneId);
			return;
		}

		// Atomic operation: remove then add
		this._layout.update((layout) => {
			// Step 1: Remove pane from current location (preserves pane object)
			const afterRemove = this.removePaneFromLayout(layout, paneId);

			// Step 2: Clone the pane node (state is preserved)
			const paneToMove: PaneNode = {
				...pane,
				config: { ...pane.config },
			};

			// Step 3: Add pane to new location relative to target
			const finalLayout = this.addPaneRelativeTo(afterRemove, paneToMove, targetPaneId, position);

			return {
				...finalLayout,
				activePaneId: paneId,
				lastModified: Date.now(),
			};
		});

		this.emitEvent({
			type: 'pane-moved',
			paneId,
			fromContainerId: '',
			toContainerId: targetPaneId,
		});
	}

	/**
	 * Swap two panes (for center drop)
	 */
	swapPanes(paneId1: string, paneId2: string): void {
		if (paneId1 === paneId2) return;

		this._layout.update((layout) => {
			const newRoot = this.swapNodesInLayout(layout.root, paneId1, paneId2);
			return {
				...layout,
				root: newRoot,
				activePaneId: paneId1,
				lastModified: Date.now(),
			};
		});

		this.emitEvent({ type: 'layout-changed', layout: get(this._layout) });
	}

	/**
	 * Save layout for persistence
	 */
	saveLayout(): WorkspaceLayout {
		return get(this._layout);
	}

	/**
	 * Restore layout from saved state
	 */
	restoreLayout(layout: WorkspaceLayout): void {
		// Validate and migrate if needed
		const migratedLayout = this.migrateLayout(layout);

		this._layout.set(migratedLayout);
		this.emitEvent({ type: 'layout-changed', layout: migratedLayout });
	}

	/**
	 * Reset to default layout
	 */
	resetLayout(): void {
		const defaultLayout = createDefaultLayout();
		this._layout.set(defaultLayout);
		this.emitEvent({ type: 'layout-changed', layout: defaultLayout });
	}

	/**
	 * Get the current layout tree
	 */
	getLayoutTree(): LayoutNode {
		return get(this._layout).root;
	}

	/**
	 * Find a pane by ID
	 */
	getPaneById(paneId: string): PaneNode | null {
		return this.findPane(get(this._layout).root, paneId);
	}

	/**
	 * Get all panes of a specific type
	 */
	getPanesByType(paneType: PaneType): PaneNode[] {
		return this.collectPanes(get(this._layout).root).filter((p) => p.paneType === paneType);
	}

	/**
	 * Check if a pane exists
	 */
	hasPane(paneId: string): boolean {
		return this.findPane(get(this._layout).root, paneId) !== null;
	}

	// =========================================================================
	// Private Helper Methods
	// =========================================================================

	private emitEvent(event: WorkspaceEvent): void {
		this._events.set(event);
	}

	private insertPane(
		layout: WorkspaceLayout,
		paneNode: PaneNode,
		options: AddPaneOptions
	): WorkspaceLayout {
		const { referenceId, position = 'right' } = options;

		// If no reference, add to root
		if (!referenceId) {
			return this.addPaneToRoot(layout, paneNode, position);
		}

		// Find reference node and insert relative to it
		return this.addPaneRelativeTo(layout, paneNode, referenceId, position);
	}

	private addPaneToRoot(
		layout: WorkspaceLayout,
		paneNode: PaneNode,
		position: AddPanePosition
	): WorkspaceLayout {
		const root = layout.root;

		// If root is a single pane, convert to split or tab
		if (isPaneNode(root)) {
			if (position === 'tab') {
				return {
					...layout,
					root: {
						type: 'tab',
						id: generateLayoutId(),
						children: [root, paneNode],
						activeIndex: 1,
					},
				};
			} else {
				const orientation: 'horizontal' | 'vertical' =
					position === 'left' || position === 'right' ? 'horizontal' : 'vertical';
				const children =
					position === 'left' || position === 'top' ? [paneNode, root] : [root, paneNode];

				return {
					...layout,
					root: {
						type: 'split',
						id: generateLayoutId(),
						orientation,
						children,
						sizes: [0.5, 0.5],
					},
				};
			}
		}

		// If root is a tab container
		if (isTabNode(root)) {
			if (position === 'tab') {
				return {
					...layout,
					root: {
						...root,
						children: [...root.children, paneNode],
						activeIndex: root.children.length,
					},
				};
			}
			// Wrap tab in split
			const orientation: 'horizontal' | 'vertical' =
				position === 'left' || position === 'right' ? 'horizontal' : 'vertical';
			const children = position === 'left' || position === 'top' ? [paneNode, root] : [root, paneNode];

			return {
				...layout,
				root: {
					type: 'split',
					id: generateLayoutId(),
					orientation,
					children,
					sizes: [0.5, 0.5],
				},
			};
		}

		// Root is split - add as new child
		if (isSplitNode(root)) {
			const newSizes = redistributeSizes(root.sizes, 1);
			const insertIndex = position === 'left' || position === 'top' ? 0 : root.children.length;
			const newChildren = [...root.children];
			newChildren.splice(insertIndex, 0, paneNode);

			return {
				...layout,
				root: {
					...root,
					children: newChildren,
					sizes: newSizes,
				},
			};
		}

		return layout;
	}

	private addPaneRelativeTo(
		layout: WorkspaceLayout,
		paneNode: PaneNode,
		referenceId: string,
		position: AddPanePosition
	): WorkspaceLayout {
		const newRoot = this.modifyTree(layout.root, (node, parent) => {
			if (!isPaneNode(node) || node.id !== referenceId) {
				return node;
			}

			// Found reference pane - create appropriate container
			if (position === 'tab') {
				// Check if parent is already a tab container
				if (parent && isTabNode(parent)) {
					// Will be handled by parent modification
					return node;
				}

				// Wrap in new tab container
				return {
					type: 'tab',
					id: generateLayoutId(),
					children: [node, paneNode],
					activeIndex: 1,
				} as TabNode;
			} else {
				// Wrap in split container
				const orientation: 'horizontal' | 'vertical' =
					position === 'left' || position === 'right' ? 'horizontal' : 'vertical';
				const children =
					position === 'left' || position === 'top' ? [paneNode, node] : [node, paneNode];

				return {
					type: 'split',
					id: generateLayoutId(),
					orientation,
					children,
					sizes: [0.5, 0.5],
				} as SplitNode;
			}
		});

		// Handle tab insertion if parent is already a tab container
		if (position === 'tab') {
			const finalRoot = this.insertIntoTabContainer(newRoot, referenceId, paneNode);
			return { ...layout, root: finalRoot };
		}

		return { ...layout, root: newRoot };
	}

	private insertIntoTabContainer(
		node: LayoutNode,
		referenceId: string,
		paneNode: PaneNode
	): LayoutNode {
		if (isTabNode(node)) {
			const refIndex = node.children.findIndex((c) => c.id === referenceId);
			if (refIndex !== -1) {
				return {
					...node,
					children: [...node.children, paneNode],
					activeIndex: node.children.length,
				};
			}
		}

		if (isSplitNode(node)) {
			return {
				...node,
				children: node.children.map((child) =>
					this.insertIntoTabContainer(child, referenceId, paneNode)
				),
			};
		}

		if (isTabNode(node)) {
			return {
				...node,
				children: node.children.map(
					(child) => this.insertIntoTabContainer(child, referenceId, paneNode) as PaneNode
				),
			};
		}

		return node;
	}

	private removePaneFromLayout(layout: WorkspaceLayout, paneId: string): WorkspaceLayout {
		const newRoot = this.removeNodeFromTree(layout.root, paneId);

		if (!newRoot) {
			// Last pane removed - create default
			return createDefaultLayout();
		}

		// Clean up empty containers
		const cleanedRoot = this.cleanupEmptyContainers(newRoot);

		// Update active pane if needed
		let newActivePaneId = layout.activePaneId;
		if (newActivePaneId === paneId) {
			const panes = this.collectPanes(cleanedRoot);
			newActivePaneId = panes.length > 0 ? panes[0].id : undefined;
		}

		return {
			...layout,
			root: cleanedRoot,
			activePaneId: newActivePaneId,
		};
	}

	private removeNodeFromTree(node: LayoutNode, paneId: string): LayoutNode | null {
		if (isPaneNode(node)) {
			return node.id === paneId ? null : node;
		}

		if (isTabNode(node)) {
			const newChildren = node.children.filter((c) => c.id !== paneId);
			if (newChildren.length === 0) return null;
			if (newChildren.length === 1) return newChildren[0];

			const newActiveIndex = Math.min(node.activeIndex, newChildren.length - 1);
			return {
				...node,
				children: newChildren,
				activeIndex: newActiveIndex,
			};
		}

		if (isSplitNode(node)) {
			const newChildren: LayoutNode[] = [];
			const newSizes: number[] = [];

			for (let i = 0; i < node.children.length; i++) {
				const child = node.children[i];
				const processedChild = this.removeNodeFromTree(child, paneId);
				if (processedChild) {
					newChildren.push(processedChild);
					newSizes.push(node.sizes[i]);
				}
			}

			if (newChildren.length === 0) return null;
			if (newChildren.length === 1) return newChildren[0];

			return {
				...node,
				children: newChildren,
				sizes: normalizeSizes(newSizes),
			};
		}

		return node;
	}

	private cleanupEmptyContainers(node: LayoutNode): LayoutNode {
		if (isPaneNode(node)) {
			return node;
		}

		if (isTabNode(node)) {
			if (node.children.length === 1) {
				return node.children[0];
			}
			return node;
		}

		if (isSplitNode(node)) {
			const cleanedChildren = node.children.map((c) => this.cleanupEmptyContainers(c));

			if (cleanedChildren.length === 1) {
				return cleanedChildren[0];
			}

			return {
				...node,
				children: cleanedChildren,
			};
		}

		return node;
	}

	private splitPaneInLayout(
		root: LayoutNode,
		paneId: string,
		direction: 'left' | 'right' | 'top' | 'bottom',
		newPane: PaneNode
	): LayoutNode {
		return this.modifyTree(root, (node) => {
			if (!isPaneNode(node) || node.id !== paneId) {
				return node;
			}

			const orientation: 'horizontal' | 'vertical' =
				direction === 'left' || direction === 'right' ? 'horizontal' : 'vertical';
			const children =
				direction === 'left' || direction === 'top' ? [newPane, node] : [node, newPane];

			return {
				type: 'split',
				id: generateLayoutId(),
				orientation,
				children,
				sizes: [0.5, 0.5],
			} as SplitNode;
		});
	}

	private updateTabActiveIndex(node: LayoutNode, paneId: string): LayoutNode {
		if (isPaneNode(node)) {
			return node;
		}

		if (isTabNode(node)) {
			const index = node.children.findIndex((c) => c.id === paneId);
			if (index !== -1) {
				return { ...node, activeIndex: index };
			}
			return node;
		}

		if (isSplitNode(node)) {
			return {
				...node,
				children: node.children.map((c) => this.updateTabActiveIndex(c, paneId)),
			};
		}

		return node;
	}

	private updatePaneInLayout(
		node: LayoutNode,
		paneId: string,
		updater: (pane: PaneNode) => PaneNode
	): LayoutNode {
		if (isPaneNode(node)) {
			return node.id === paneId ? updater(node) : node;
		}

		if (isTabNode(node)) {
			return {
				...node,
				children: node.children.map((c) =>
					c.id === paneId ? updater(c) : c
				),
			};
		}

		if (isSplitNode(node)) {
			return {
				...node,
				children: node.children.map((c) =>
					this.updatePaneInLayout(c, paneId, updater)
				),
			};
		}

		return node;
	}

	private updateNodeInLayout(
		node: LayoutNode,
		nodeId: string,
		updater: (node: LayoutNode) => LayoutNode
	): LayoutNode {
		if (node.id === nodeId) {
			return updater(node);
		}

		if (isTabNode(node)) {
			return {
				...node,
				children: node.children.map((c) =>
					this.updateNodeInLayout(c, nodeId, updater) as PaneNode
				),
			};
		}

		if (isSplitNode(node)) {
			return {
				...node,
				children: node.children.map((c) => this.updateNodeInLayout(c, nodeId, updater)),
			};
		}

		return node;
	}

	private modifyTree(
		node: LayoutNode,
		modifier: (node: LayoutNode, parent?: LayoutNode) => LayoutNode,
		parent?: LayoutNode
	): LayoutNode {
		const modified = modifier(node, parent);

		// If the node was replaced, don't recurse into the replacement
		if (modified !== node) {
			return modified;
		}

		if (isTabNode(node)) {
			return {
				...node,
				children: node.children.map(
					(c) => this.modifyTree(c, modifier, node) as PaneNode
				),
			};
		}

		if (isSplitNode(node)) {
			return {
				...node,
				children: node.children.map((c) => this.modifyTree(c, modifier, node)),
			};
		}

		return node;
	}

	private collectPanes(node: LayoutNode): PaneNode[] {
		if (isPaneNode(node)) {
			return [node];
		}

		if (isTabNode(node)) {
			return node.children;
		}

		if (isSplitNode(node)) {
			return node.children.flatMap((c) => this.collectPanes(c));
		}

		return [];
	}

	private findPane(node: LayoutNode, paneId: string): PaneNode | null {
		if (isPaneNode(node)) {
			return node.id === paneId ? node : null;
		}

		if (isTabNode(node)) {
			return node.children.find((c) => c.id === paneId) ?? null;
		}

		if (isSplitNode(node)) {
			for (const child of node.children) {
				const found = this.findPane(child, paneId);
				if (found) return found;
			}
		}

		return null;
	}

	private migrateLayout(layout: WorkspaceLayout): WorkspaceLayout {
		// Handle version migrations here
		// For now, just validate the layout
		if (!layout.root) {
			return createDefaultLayout();
		}

		// Ensure version is set
		if (!layout.version) {
			layout = { ...layout, version: 1 };
		}

		return {
			...layout,
			lastModified: Date.now(),
		};
	}

	/**
	 * Swap two pane nodes in the layout tree
	 */
	private swapNodesInLayout(node: LayoutNode, paneId1: string, paneId2: string): LayoutNode {
		if (isPaneNode(node)) {
			if (node.id === paneId1) {
				// Return placeholder that will be replaced
				return { ...node, id: `__swap_placeholder_1__` } as PaneNode;
			}
			if (node.id === paneId2) {
				return { ...node, id: `__swap_placeholder_2__` } as PaneNode;
			}
			return node;
		}

		if (isTabNode(node)) {
			const newChildren = node.children.map((c) => {
				if (c.id === paneId1) return { ...c, id: `__swap_placeholder_1__` };
				if (c.id === paneId2) return { ...c, id: `__swap_placeholder_2__` };
				return c;
			});

			// Now swap the placeholders back
			const finalChildren = newChildren.map((c) => {
				if (c.id === `__swap_placeholder_1__`) return { ...c, id: paneId2 };
				if (c.id === `__swap_placeholder_2__`) return { ...c, id: paneId1 };
				return c;
			});

			return { ...node, children: finalChildren };
		}

		if (isSplitNode(node)) {
			// First pass: mark for swap
			const markedChildren = node.children.map((c) =>
				this.swapNodesInLayout(c, paneId1, paneId2)
			);

			// Second pass: complete swap
			const finalChildren = markedChildren.map((c) => {
				if (isPaneNode(c)) {
					if (c.id === `__swap_placeholder_1__`) return { ...c, id: paneId2 };
					if (c.id === `__swap_placeholder_2__`) return { ...c, id: paneId1 };
				}
				return c;
			});

			return { ...node, children: finalChildren };
		}

		return node;
	}
}

// ============================================================================
// Singleton Export
// ============================================================================

export const workspaceManager = new WorkspaceManagerImpl();

// ============================================================================
// Convenience Exports
// ============================================================================

export { PaneType } from './layout-model';
export type {
	WorkspaceLayout,
	LayoutNode,
	PaneNode,
	SplitNode,
	TabNode,
	PaneConfig,
	AddPaneOptions,
	AddPanePosition,
	WorkspaceEvent,
	IPaneInstance,
} from './layout-model';
