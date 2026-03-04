export type RegistryErrorCode =
	| 'quill_not_found'
	| 'version_not_found'
	| 'load_error'
	| 'source_unavailable';

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
