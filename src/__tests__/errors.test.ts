import { describe, it, expect } from 'vitest';
import { RegistryError } from '../errors.js';
import type { RegistryErrorCode } from '../errors.js';

describe('RegistryError', () => {
	it('should create an error with code and message', () => {
		const err = new RegistryError('quill_not_found', 'Quill "foo" not found');
		expect(err).toBeInstanceOf(Error);
		expect(err).toBeInstanceOf(RegistryError);
		expect(err.name).toBe('RegistryError');
		expect(err.code).toBe('quill_not_found');
		expect(err.message).toBe('Quill "foo" not found');
		expect(err.quillName).toBeUndefined();
		expect(err.version).toBeUndefined();
	});

	it('should include quillName and version when provided', () => {
		const err = new RegistryError('version_not_found', 'Version not found', {
			quillName: 'usaf_memo',
			version: '1.0.0',
		});
		expect(err.code).toBe('version_not_found');
		expect(err.quillName).toBe('usaf_memo');
		expect(err.version).toBe('1.0.0');
	});

	it('should include cause when provided', () => {
		const cause = new Error('network failure');
		const err = new RegistryError('source_unavailable', 'Failed to fetch', { cause });
		expect(err.cause).toBe(cause);
	});

	it('should support all error codes', () => {
		const codes: RegistryErrorCode[] = [
			'quill_not_found',
			'version_not_found',
			'load_error',
			'source_unavailable',
		];
		for (const code of codes) {
			const err = new RegistryError(code, `Error: ${code}`);
			expect(err.code).toBe(code);
		}
	});
});
