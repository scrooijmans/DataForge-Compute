/**
 * TypeScript types matching Rust backend structs
 */

// DataForge status and connection
export interface DataForgeStatus {
	connected: boolean;
	data_dir: string | null;
	db_exists: boolean;
	provider_count: number;
	udf_count: number;
	error: string | null;
}

// Data entities
export interface WorkspaceInfo {
	id: string;
	name: string;
	created_at: string;
}

export interface WellInfo {
	id: string;
	name: string;
	uwi: string | null;
	field: string | null;
	curve_count: number;
}

export interface CurveInfo {
	id: string;
	mnemonic: string;
	unit: string | null;
	description: string | null;
	main_curve_type: string | null;
	min_depth: number | null;
	max_depth: number | null;
	row_count: number;
}

/** Curve info with associated well data - for curve selector dialogs */
export interface CurveInfoWithWell extends CurveInfo {
	well_id: string;
	well_name: string;
}

export interface CurveDataPoint {
	depth: number;
	value: number | null;
}

export interface CurveData {
	curve_id: string;
	mnemonic: string;
	unit: string | null;
	data: CurveDataPoint[];
}

// UDF types
export interface ProviderInfo {
	id: string;
	name: string;
	version: string;
	description: string;
	udf_count: number;
}

export interface UdfInfo {
	full_id: string;
	provider_id: string;
	name: string;
	category: string;
	description: string;
	version: string;
	tags: string[];
}

export interface ParameterDefinition {
	name: string;
	label: string;
	description: string;
	type: 'curve' | 'number' | 'boolean' | 'string';
	required: boolean;
	default?: unknown;
	min?: number;
	max?: number;
	unit?: string;
	allowed_types?: string[];
	min_length?: number;
	allow_nulls?: boolean;
}

export interface ValidationError {
	field: string;
	message: string;
	suggestion?: string;
}

export interface ExecuteUdfRequest {
	udf_id: string;
	well_id: string;
	workspace_id: string;
	parameters: Record<string, unknown>;
	save_result?: boolean;
}

export interface ExecuteUdfResult {
	success: boolean;
	execution_id: string;
	output_mnemonic: string | null;
	output_curve_id: string | null;
	output_data: CurveDataPoint[] | null;
	warnings: string[];
	error: string | null;
	saved: boolean;
}

// Provenance types
export interface InputReference {
	curve_id: string;
	mnemonic: string;
	parquet_hash: string;
	version: number;
}

export interface CurveProvenance {
	id: string;
	udf_id: string;
	udf_version: string;
	inputs: InputReference[];
	parameters: Record<string, unknown>;
	output_curve_id: string | null;
	output_parquet_hash: string | null;
	started_at: string;
	completed_at: string | null;
	compute_app_version: string;
	status: string;
	error_message: string | null;
}
