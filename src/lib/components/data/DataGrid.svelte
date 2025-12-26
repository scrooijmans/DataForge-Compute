<script lang="ts">
	/**
	 * Reusable AG Grid component for displaying tabular data
	 *
	 * Based on AG Grid Community Edition with Client-Side Row Model.
	 * See: docs/context7/ag-grid/ag-grid-sql-table-architecture.md
	 *
	 * Features:
	 * - Cell editing (single-click)
	 * - Multi-row selection (Finder-like: Cmd+Click to add/remove)
	 * - Row transactions (add, remove, update via API)
	 * - Keyboard shortcuts (Delete to remove selected rows)
	 */
	import { browser } from '$app/environment'
	import type {
		ColDef,
		GridApi,
		GridOptions,
		CellValueChangedEvent,
		SelectionChangedEvent,
		RowDataTransaction
	} from 'ag-grid-community'

	// Props
	interface Props {
		/** Column definitions for the grid */
		columnDefs: ColDef[]
		/** Row data to display */
		rowData: Record<string, unknown>[]
		/** Optional: Default column definition */
		defaultColDef?: ColDef
		/** Optional: Enable pagination */
		pagination?: boolean
		/** Optional: Page size for pagination */
		paginationPageSize?: number
		/** Optional: Auto-size columns on data change */
		autoSizeColumns?: boolean
		/** Optional: Grid height (CSS value) */
		height?: string
		/** Optional: Additional grid options */
		gridOptions?: Partial<GridOptions>
		/** Optional: Callback when grid is ready */
		onGridReady?: (api: GridApi) => void
		/** Optional: Callback when a cell value changes (for editing) */
		onCellValueChanged?: (event: CellValueChangedEvent) => void
		/** Optional: Callback when selection changes */
		onSelectionChanged?: (event: SelectionChangedEvent) => void
		/** Optional: Callback when rows are deleted (returns deleted rows) */
		onRowsDeleted?: (deletedRows: Record<string, unknown>[]) => void
		/** Optional: Callback when rows are added (returns added rows) */
		onRowsAdded?: (addedRows: Record<string, unknown>[]) => void
		/** Optional: Enable cell editing */
		editable?: boolean
		/** Optional: Enable multi-row selection (Finder-like) */
		multiSelect?: boolean
		/** Optional: Show checkboxes for selection */
		showCheckboxes?: boolean
		/** Optional: Function to get unique row ID (required for transactions) */
		getRowId?: (data: Record<string, unknown>) => string
		/** Optional: Enable keyboard delete (Delete/Backspace to remove selected rows) */
		enableKeyboardDelete?: boolean
	}

	let {
		columnDefs,
		rowData,
		defaultColDef = {
			flex: 1,
			minWidth: 100,
			sortable: true,
			filter: true,
			resizable: true
		},
		pagination = false, // Disabled by default - use virtual scrolling instead
		paginationPageSize = 100,
		autoSizeColumns = true,
		height = '100%',
		gridOptions = {},
		onGridReady,
		onCellValueChanged,
		onSelectionChanged,
		onRowsDeleted,
		onRowsAdded,
		editable = false,
		multiSelect = true,
		showCheckboxes = false,
		getRowId,
		enableKeyboardDelete = true
	}: Props = $props()

	let gridContainer: HTMLDivElement | undefined = $state()
	let gridApi: GridApi | null = $state(null)
	let gridInitialized = $state(false)

	// Handle keyboard events for delete functionality
	function handleKeyDown(event: KeyboardEvent) {
		if (!gridApi || !enableKeyboardDelete || !editable) return

		// Delete or Backspace to remove selected rows
		if (event.key === 'Delete' || event.key === 'Backspace') {
			// Don't delete if we're editing a cell
			const focusedCell = gridApi.getFocusedCell()
			if (focusedCell) {
				const editingCells = gridApi.getEditingCells()
				if (editingCells.length > 0) return
			}

			const selectedRows = gridApi.getSelectedRows()
			if (selectedRows.length > 0) {
				event.preventDefault()
				deleteSelectedRows()
			}
		}
	}

	// Wrapper callbacks that always call the current prop value
	// This ensures we don't capture stale references at init time
	function handleInternalCellValueChanged(event: CellValueChangedEvent) {
		onCellValueChanged?.(event)
	}

	function handleInternalSelectionChanged(event: SelectionChangedEvent) {
		onSelectionChanged?.(event)
	}

	// Wrapper for getRowId to always use current prop value
	function handleInternalGetRowId(params: { data: Record<string, unknown> }): string {
		if (getRowId) {
			return getRowId(params.data)
		}
		// Fallback: use JSON stringification
		return JSON.stringify(params.data)
	}

	// Initialize grid on mount using $effect
	$effect(() => {
		if (!browser || !gridContainer) return

		let api: GridApi | null = null

		// Async initialization inside effect
		const initGrid = async () => {
			// Dynamic import to avoid SSR issues
			const {
				ClientSideRowModelModule,
				ClientSideRowModelApiModule,
				RowApiModule,
				ModuleRegistry,
				createGrid,
				themeQuartz,
				PaginationModule,
				RowSelectionModule,
				TextFilterModule,
				TooltipModule,
				CellStyleModule,
				ColumnAutoSizeModule,
				TextEditorModule,
				NumberEditorModule,
				DateEditorModule,
				DateFilterModule,
				SelectEditorModule,
				ValidationModule
			} = await import('ag-grid-community')

			// Register AG Grid modules (granular modules required in v35+)
			// ClientSideRowModelApiModule is required for applyTransaction (add/remove/update rows)
			// RowApiModule is required for forEachNode and other row iteration APIs
			// SelectEditorModule is required for agSelectCellEditor (dropdown)
			ModuleRegistry.registerModules([
				ClientSideRowModelModule,
				ClientSideRowModelApiModule,
				RowApiModule,
				PaginationModule,
				RowSelectionModule,
				TextFilterModule,
				DateFilterModule,
				TooltipModule,
				CellStyleModule,
				ColumnAutoSizeModule,
				TextEditorModule,
				NumberEditorModule,
				DateEditorModule,
				SelectEditorModule,
				ValidationModule
			])

			// Create custom dark theme
			const darkTheme = themeQuartz.withParams({
				backgroundColor: 'var(--background)',
				foregroundColor: 'var(--foreground)',
				borderColor: 'var(--border)',
				headerBackgroundColor: 'var(--muted)',
				headerTextColor: 'var(--foreground)',
				oddRowBackgroundColor: 'var(--background)',
				rowHoverColor: 'var(--muted)',
				selectedRowBackgroundColor: 'var(--accent)',
				fontFamily: 'inherit',
				fontSize: '13px',
				headerFontSize: '13px',
				cellHorizontalPadding: '12px'
			})

			// Merge editable into defaultColDef if enabled
			const mergedDefaultColDef = editable
				? { ...defaultColDef, editable: true }
				: defaultColDef

			// Configure row selection mode (Finder-like multi-select or single)
			const rowSelectionConfig = multiSelect
				? {
						mode: 'multiRow' as const,
						enableClickSelection: true,
						checkboxes: showCheckboxes,
						headerCheckbox: showCheckboxes
					}
				: {
						mode: 'singleRow' as const,
						enableClickSelection: true
					}

			const options: GridOptions = {
				columnDefs,
				rowData,
				defaultColDef: mergedDefaultColDef,
				// Pagination settings (disabled by default for virtual scrolling)
				pagination,
				paginationPageSize,
				paginationPageSizeSelector: [50, 100, 250, 500],
				// Virtual scrolling optimizations (when pagination is disabled)
				rowBuffer: 20, // Render 20 rows outside viewport for smooth scrolling
				suppressColumnVirtualisation: false, // Enable column virtualization
				suppressRowVirtualisation: false, // Enable row virtualization (default)
				animateRows: false, // Disable animations for better performance with large datasets
				debounceVerticalScrollbar: true, // Debounce scrollbar updates for smoother scrolling
				// Row selection
				rowSelection: rowSelectionConfig,
				suppressRowClickSelection: false,
				theme: darkTheme,
				// Single-click editing (no need to double-click)
				singleClickEdit: editable,
				// Stop editing when focus leaves cell
				stopEditingWhenCellsLoseFocus: true,
				// Row ID for transactions - use wrapper to avoid stale closure
				getRowId: handleInternalGetRowId,
				onGridReady: (params) => {
					if (autoSizeColumns && rowData.length > 0) {
						params.api.autoSizeAllColumns()
					}
					onGridReady?.(params.api)
				},
				// Use wrapper functions to avoid capturing stale prop references
				onCellValueChanged: handleInternalCellValueChanged,
				onSelectionChanged: handleInternalSelectionChanged,
				...gridOptions
			}

			api = createGrid(gridContainer!, options)
			gridApi = api
			gridInitialized = true

			// Add keyboard event listener for delete
			if (enableKeyboardDelete && editable) {
				gridContainer!.addEventListener('keydown', handleKeyDown)
			}
		}

		initGrid()

		// Cleanup function runs when effect re-runs or component unmounts
		return () => {
			if (gridContainer) {
				gridContainer.removeEventListener('keydown', handleKeyDown)
			}
			api?.destroy()
			gridApi = null
			gridInitialized = false
		}
	})

	// Update row data when it changes (separate effect to avoid re-initializing grid)
	$effect(() => {
		if (gridApi && gridInitialized && rowData) {
			gridApi.setGridOption('rowData', rowData)
			if (autoSizeColumns && rowData.length > 0) {
				// Small delay to let data render before auto-sizing
				setTimeout(() => gridApi?.autoSizeAllColumns(), 100)
			}
		}
	})

	// Update column definitions when they change
	$effect(() => {
		if (gridApi && gridInitialized && columnDefs) {
			gridApi.setGridOption('columnDefs', columnDefs)
		}
	})

	// ===== Exported API Methods =====

	/** Get the underlying AG Grid API */
	export function getApi(): GridApi | null {
		return gridApi
	}

	/** Get all selected rows */
	export function getSelectedRows(): Record<string, unknown>[] {
		return gridApi?.getSelectedRows() ?? []
	}

	/** Delete selected rows from the grid */
	export function deleteSelectedRows(): Record<string, unknown>[] {
		if (!gridApi) return []

		const selectedRows = gridApi.getSelectedRows()
		if (selectedRows.length === 0) return []

		const result = gridApi.applyTransaction({ remove: selectedRows })
		const deletedRows = result?.remove?.map((node) => node.data) ?? []

		// Notify parent component
		if (deletedRows.length > 0) {
			onRowsDeleted?.(deletedRows)
		}

		return deletedRows
	}

	/** Delete specific rows from the grid */
	export function deleteRows(rows: Record<string, unknown>[]): Record<string, unknown>[] {
		if (!gridApi || rows.length === 0) return []

		const result = gridApi.applyTransaction({ remove: rows })
		const deletedRows = result?.remove?.map((node) => node.data) ?? []

		if (deletedRows.length > 0) {
			onRowsDeleted?.(deletedRows)
		}

		return deletedRows
	}

	/** Add new rows to the grid */
	export function addRows(
		rows: Record<string, unknown>[],
		addIndex?: number
	): Record<string, unknown>[] {
		if (!gridApi) {
			console.warn('[DataGrid] addRows called but gridApi is not initialized')
			return []
		}
		if (rows.length === 0) {
			console.warn('[DataGrid] addRows called with empty rows array')
			return []
		}

		const transaction: RowDataTransaction = { add: rows }
		if (addIndex !== undefined) {
			transaction.addIndex = addIndex
		}

		console.log('[DataGrid] Applying transaction:', transaction)

		try {
			const result = gridApi.applyTransaction(transaction)
			console.log('[DataGrid] Transaction result:', result)

			const addedRows = result?.add?.map((node) => node.data) ?? []

			if (addedRows.length > 0) {
				onRowsAdded?.(addedRows)
			}

			return addedRows
		} catch (error) {
			console.error('[DataGrid] Transaction failed, falling back to setGridOption:', error)

			// Fallback: get current data and add rows manually
			const currentRows: Record<string, unknown>[] = []
			gridApi.forEachNode((node) => {
				if (node.data) currentRows.push(node.data)
			})

			// Insert at specified index or at beginning (index 0) by default
			const insertAt = addIndex ?? 0
			const newRowData = [
				...currentRows.slice(0, insertAt),
				...rows,
				...currentRows.slice(insertAt)
			]

			gridApi.setGridOption('rowData', newRowData)
			onRowsAdded?.(rows)

			return rows
		}
	}

	/** Add a single row at the end (or at specific index) */
	export function addRow(row: Record<string, unknown>, addIndex?: number): Record<string, unknown> | null {
		const added = addRows([row], addIndex)
		return added.length > 0 ? added[0] : null
	}

	/** Add a row after the currently selected row */
	export function addRowAfterSelected(row: Record<string, unknown>): Record<string, unknown> | null {
		if (!gridApi) return null

		const selectedNodes = gridApi.getSelectedNodes()
		if (selectedNodes.length === 0) {
			// No selection, add at end
			return addRow(row)
		}

		const selectedNode = selectedNodes[0]
		const rowIndex = selectedNode.rowIndex
		if (rowIndex !== null) {
			return addRow(row, rowIndex + 1)
		}

		return addRow(row)
	}

	/** Update existing rows in the grid */
	export function updateRows(rows: Record<string, unknown>[]): Record<string, unknown>[] {
		if (!gridApi || rows.length === 0) return []

		const result = gridApi.applyTransaction({ update: rows })
		return result?.update?.map((node) => node.data) ?? []
	}

	/** Select all rows */
	export function selectAll(): void {
		gridApi?.selectAll()
	}

	/** Deselect all rows */
	export function deselectAll(): void {
		gridApi?.deselectAll()
	}

	/** Get total row count */
	export function getRowCount(): number {
		let count = 0
		gridApi?.forEachNode(() => count++)
		return count
	}

	/** Get all row data */
	export function getAllRows(): Record<string, unknown>[] {
		const rows: Record<string, unknown>[] = []
		gridApi?.forEachNode((node) => {
			if (node.data) rows.push(node.data)
		})
		return rows
	}
</script>

{#if browser}
	<!-- tabindex makes the grid focusable for keyboard events -->
	<div
		bind:this={gridContainer}
		class="ag-theme-quartz-dark"
		style="height: {height}; width: 100%;"
		tabindex="0"
		role="grid"
	></div>
{:else}
	<div class="flex h-full items-center justify-center text-muted-foreground" style="height: {height};">
		Loading grid...
	</div>
{/if}

<style>
	/* Override AG Grid styles to match app theme */
	:global(.ag-theme-quartz-dark) {
		--ag-background-color: hsl(var(--background));
		--ag-foreground-color: hsl(var(--foreground));
		--ag-border-color: hsl(var(--border));
		--ag-header-background-color: hsl(var(--muted));
		--ag-odd-row-background-color: hsl(var(--background));
		--ag-row-hover-color: hsl(var(--muted));
		--ag-selected-row-background-color: hsl(var(--accent));
	}
</style>
