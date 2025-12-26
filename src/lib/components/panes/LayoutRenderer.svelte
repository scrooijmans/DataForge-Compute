<script lang="ts">
	/**
	 * LayoutRenderer - Recursive layout tree renderer
	 *
	 * Renders the layout tree by recursively dispatching to appropriate
	 * container components based on node type:
	 * - SplitNode -> SplitContainer
	 * - TabNode -> TabContainer
	 * - PaneNode -> PaneContainer
	 *
	 * See DFC-chart-implementation.md Section 12 for design details.
	 */
	import type { LayoutNode, SplitNode, TabNode, PaneNode } from '$lib/panes/layout-model';
	import { isPaneNode, isSplitNode, isTabNode } from '$lib/panes/layout-model';
	import SplitContainer from './SplitContainer.svelte';
	import TabContainer from './TabContainer.svelte';
	import PaneContainer from './PaneContainer.svelte';

	interface Props {
		/** Layout node to render */
		node: LayoutNode;
		/** Depth in the layout tree (for debugging) */
		depth?: number;
	}

	let { node, depth = 0 }: Props = $props();
</script>

{#if isPaneNode(node)}
	<PaneContainer pane={node} />
{:else if isTabNode(node)}
	<TabContainer tabNode={node} {depth} />
{:else if isSplitNode(node)}
	<SplitContainer splitNode={node} {depth} />
{/if}
