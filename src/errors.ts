export type RegistryErrorCode =
	| 'quill_not_found'
	| 'version_not_found'
	| 'load_error'
	| 'source_unavailable';

function toJsonSafeValue(value: unknown, seen: WeakSet<object>): unknown {
	if (
		value === null ||
		typeof value === 'string' ||
		typeof value === 'number' ||
		typeof value === 'boolean'
	) {
		return value;
	}

	if (typeof value === 'bigint') {
		return `${value.toString()}n`;
	}

	if (typeof value === 'undefined') {
		return '[undefined]';
	}

	if (typeof value === 'function') {
		return `[Function ${value.name || 'anonymous'}]`;
	}

	if (value instanceof Map) {
		return {
			type: 'Map',
			size: value.size,
			entries: Array.from(value.entries(), ([k, v]) => [
				toJsonSafeValue(k, seen),
				toJsonSafeValue(v, seen),
			]),
		};
	}

	if (value instanceof Set) {
		return {
			type: 'Set',
			size: value.size,
			values: Array.from(value.values(), (v) => toJsonSafeValue(v, seen)),
		};
	}

	if (value instanceof Uint8Array) {
		return {
			type: 'Uint8Array',
			length: value.length,
			bytes: Array.from(value),
		};
	}

	if (value instanceof Error) {
		const errorRecord: Record<string, unknown> = {
			name: value.name,
			message: value.message,
		};
		const causeValue = (value as Error & { cause?: unknown }).cause;
		if (causeValue !== undefined) {
			errorRecord.cause = toJsonSafeValue(causeValue, seen);
		}
		return errorRecord;
	}

	if (Array.isArray(value)) {
		return value.map((item) => toJsonSafeValue(item, seen));
	}

	if (typeof value === 'object') {
		const obj = value as Record<string, unknown>;
		if (seen.has(obj)) return '[Circular]';
		seen.add(obj);
		const out: Record<string, unknown> = {};
		for (const [k, v] of Object.entries(obj)) {
			out[k] = toJsonSafeValue(v, seen);
		}
		return out;
	}

	return String(value);
}

/**
 * Converts unknown thrown values into a readable, stable string.
 *
 * Keeps plain Error messages concise while preserving structure for
 * non-Error payloads (e.g. Map diagnostics from WASM bindings).
 */
export function formatUnknownError(err: unknown): string {
	if (err instanceof Error) {
		const base = err.message || err.name;
		const causeValue = (err as Error & { cause?: unknown }).cause;
		if (causeValue === undefined) return base;
		return `${base}. Cause: ${formatUnknownError(causeValue)}`;
	}

	const safeValue = toJsonSafeValue(err, new WeakSet());
	try {
		const serialized = JSON.stringify(safeValue, null, 2);
		if (serialized && serialized !== '{}') {
			return serialized;
		}
	} catch {
		// Fall through to string coercion for non-serializable values.
	}

	return String(err);
}

export class RegistryError extends Error {
	code: RegistryErrorCode;
	quillName?: string;
	version?: string;

	constructor(
		code: RegistryErrorCode,
		message: string,
		options?: { quillName?: string; version?: string; cause?: unknown },
	) {
		super(message, options?.cause ? { cause: options.cause } : undefined);
		this.name = 'RegistryError';
		this.code = code;
		this.quillName = options?.quillName;
		this.version = options?.version;
	}
}
