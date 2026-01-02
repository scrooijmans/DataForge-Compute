<script lang="ts">
	/**
	 * ChartInteractionBar - Vertical toolbar for chart interaction modes
	 *
	 * Displays interaction mode buttons based on the currently selected chart type.
	 * Inspired by TradingView's left-side vertical toolbar.
	 *
	 * Features:
	 * - Vertical layout on left side of charts
	 * - Toggle between cursor modes (pointer, crosshair, zoom, pan)
	 * - Visual feedback for active mode
	 * - Extensible for future chart-specific tools
	 */

	import {
		chartInteractionStore,
		cursorMode,
		selectionMode,
		activePaneSupportsInteraction,
		activePaneSupportsBrushSelection,
		CURSOR_MODES,
		SELECTION_MODES,
		type CursorMode,
		type SelectionMode,
	} from '$lib/stores/chartInteraction';

	// Current mode from store
	let currentMode = $derived($cursorMode);
	let currentSelectionMode = $derived($selectionMode);
	let showBar = $derived($activePaneSupportsInteraction);
	let showSelectionTools = $derived($activePaneSupportsBrushSelection);

	/**
	 * Handle mode button click
	 */
	function selectMode(mode: CursorMode): void {
		chartInteractionStore.setCursorMode(mode);
	}

	/**
	 * Handle selection mode button click
	 */
	function selectSelectionMode(mode: SelectionMode): void {
		// Toggle off if already selected
		if (currentSelectionMode === mode) {
			chartInteractionStore.setSelectionMode('none');
		} else {
			chartInteractionStore.setSelectionMode(mode);
		}
	}

	/**
	 * Get icon SVG path for each cursor mode
	 */
	function getIconPath(mode: CursorMode): string {
		switch (mode) {
			case 'pointer':
				// Cursor arrow
				return 'M13.64 21.97C13.14 22.21 12.54 22 12.31 21.5L10.13 16.76L7.62 18.78C7.45 18.92 7.24 19 7 19C6.45 19 6 18.55 6 18V3C6 2.45 6.45 2 7 2C7.24 2 7.47 2.09 7.64 2.23L7.65 2.22L19.14 11.86C19.57 12.22 19.62 12.85 19.27 13.27C19.12 13.45 18.91 13.57 18.69 13.61L15.54 14.23L17.74 18.96C17.97 19.46 17.76 20.06 17.26 20.3L13.64 21.97Z';
			case 'crosshair':
				// Crosshairs
				return 'M12 2C12.55 2 13 2.45 13 3V4.07C16.39 4.56 19 7.47 19 11H20C20.55 11 21 11.45 21 12C21 12.55 20.55 13 20 13H19C19 16.53 16.39 19.44 13 19.93V21C13 21.55 12.55 22 12 22C11.45 22 11 21.55 11 21V19.93C7.61 19.44 5 16.53 5 13H4C3.45 13 3 12.55 3 12C3 11.45 3.45 11 4 11H5C5 7.47 7.61 4.56 11 4.07V3C11 2.45 11.45 2 12 2ZM12 6C8.69 6 6 8.69 6 12C6 15.31 8.69 18 12 18C15.31 18 18 15.31 18 12C18 8.69 15.31 6 12 6ZM12 8C14.21 8 16 9.79 16 12C16 14.21 14.21 16 12 16C9.79 16 8 14.21 8 12C8 9.79 9.79 8 12 8ZM12 10C10.9 10 10 10.9 10 12C10 13.1 10.9 14 12 14C13.1 14 14 13.1 14 12C14 10.9 13.1 10 12 10Z';
			case 'zoom-in':
				// Magnify plus
				return 'M15.5 14L20.5 19L19 20.5L14 15.5V14.71L13.73 14.43C12.59 15.41 11.11 16 9.5 16C5.91 16 3 13.09 3 9.5C3 5.91 5.91 3 9.5 3C13.09 3 16 5.91 16 9.5C16 11.11 15.41 12.59 14.43 13.73L14.71 14H15.5ZM9.5 14C11.99 14 14 11.99 14 9.5C14 7.01 11.99 5 9.5 5C7.01 5 5 7.01 5 9.5C5 11.99 7.01 14 9.5 14ZM10 7H9V9H7V10H9V12H10V10H12V9H10V7Z';
			case 'zoom-out':
				// Magnify minus
				return 'M15.5 14L20.5 19L19 20.5L14 15.5V14.71L13.73 14.43C12.59 15.41 11.11 16 9.5 16C5.91 16 3 13.09 3 9.5C3 5.91 5.91 3 9.5 3C13.09 3 16 5.91 16 9.5C16 11.11 15.41 12.59 14.43 13.73L14.71 14H15.5ZM9.5 14C11.99 14 14 11.99 14 9.5C14 7.01 11.99 5 9.5 5C7.01 5 5 7.01 5 9.5C5 11.99 7.01 14 9.5 14ZM7 9H12V10H7V9Z';
			case 'pan':
				// Hand / Move
				return 'M13 6V11H18V7.75L22.25 12L18 16.25V13H13V18H16.25L12 22.25L7.75 18H11V13H6V16.25L1.75 12L6 7.75V11H11V6H7.75L12 1.75L16.25 6H13Z';
			default:
				return '';
		}
	}

	/**
	 * Get icon SVG path for selection modes
	 */
	function getSelectionIconPath(mode: SelectionMode): string {
		switch (mode) {
			case 'rect':
				// Rectangle selection - dashed rectangle with cursor
				return 'M4 3H20C20.55 3 21 3.45 21 4V20C21 20.55 20.55 21 20 21H4C3.45 21 3 20.55 3 20V4C3 3.45 3.45 3 4 3ZM5 5V19H19V5H5ZM7 7H17V9H7V7ZM7 15H17V17H7V15ZM7 11H9V13H7V11ZM15 11H17V13H15V11Z';
			case 'polygon':
				// Lasso/polygon selection - freeform shape
				return 'M12 2L15.09 8.26L22 9.27L17 14.14L18.18 21.02L12 17.77L5.82 21.02L7 14.14L2 9.27L8.91 8.26L12 2ZM12 5.4L9.91 9.64L5.3 10.31L8.65 13.56L7.82 18.14L12 15.9L16.18 18.14L15.35 13.56L18.7 10.31L14.09 9.64L12 5.4Z';
			default:
				return '';
		}
	}
</script>

<div class="chart-interaction-bar">
	<!-- Cursor mode buttons - vertical stack -->
	<div class="mode-buttons">
		{#each CURSOR_MODES as mode (mode.id)}
			<button
				class="mode-button"
				class:active={currentMode === mode.id}
				class:disabled={!showBar}
				onclick={() => selectMode(mode.id)}
				title={mode.tooltip}
				aria-label={mode.label}
				aria-pressed={currentMode === mode.id}
				disabled={!showBar}
			>
				<svg viewBox="0 0 24 24" class="mode-icon" aria-hidden="true">
					<path fill="currentColor" d={getIconPath(mode.id)} />
				</svg>
			</button>
		{/each}
	</div>

	<!-- Selection tools (crossplot only) -->
	{#if showSelectionTools}
		<div class="separator"></div>

		<div class="mode-buttons">
			{#each SELECTION_MODES as mode (mode.id)}
				<button
					class="mode-button"
					class:active={currentSelectionMode === mode.id}
					onclick={() => selectSelectionMode(mode.id)}
					title={mode.tooltip}
					aria-label={mode.label}
					aria-pressed={currentSelectionMode === mode.id}
				>
					<svg viewBox="0 0 24 24" class="mode-icon" aria-hidden="true">
						<path fill="currentColor" d={getSelectionIconPath(mode.id)} />
					</svg>
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.chart-interaction-bar {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
		padding: 8px 4px;
		background: hsl(var(--muted));
		border-right: 1px solid hsl(var(--border));
		width: 40px;
		flex-shrink: 0;
	}

	.separator {
		width: 24px;
		height: 1px;
		background: hsl(var(--border));
		margin: 4px 0;
	}

	.mode-buttons {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.mode-button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		padding: 0;
		border: 1px solid transparent;
		border-radius: 4px;
		background: transparent;
		color: hsl(var(--muted-foreground));
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease,
			color 0.15s ease;
	}

	.mode-button:hover {
		background: hsl(var(--accent));
		color: hsl(var(--accent-foreground));
	}

	.mode-button.active {
		background: hsl(var(--primary));
		color: hsl(var(--primary-foreground));
		border-color: hsl(var(--primary));
	}

	.mode-button.disabled,
	.mode-button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.mode-button.disabled:hover,
	.mode-button:disabled:hover {
		background: transparent;
		color: hsl(var(--muted-foreground));
	}

	.mode-button:focus-visible {
		outline: 2px solid hsl(var(--ring));
		outline-offset: 1px;
	}

	.mode-icon {
		width: 18px;
		height: 18px;
	}
</style>
