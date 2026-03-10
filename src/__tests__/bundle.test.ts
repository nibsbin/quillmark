import { describe, it, expect } from 'vitest';
import { packFiles, unpackFiles } from '../bundle.js';

describe('packFiles / unpackFiles', () => {
	it('should round-trip a single file', () => {
		const files = { 'hello.txt': new TextEncoder().encode('Hello World') };
		const packed = packFiles(files);
		const unpacked = unpackFiles(packed);

		expect(Object.keys(unpacked)).toEqual(['hello.txt']);
		expect(new TextDecoder().decode(unpacked['hello.txt'])).toBe('Hello World');
	});

	it('should round-trip multiple files', () => {
		const encoder = new TextEncoder();
		const files: Record<string, Uint8Array> = {
			'Quill.yaml': encoder.encode('name: test\nversion: 1.0.0'),
			'template.typ': encoder.encode('// template'),
			'assets/logo.txt': encoder.encode('logo-data'),
		};
		const packed = packFiles(files);
		const unpacked = unpackFiles(packed);

		expect(Object.keys(unpacked).sort()).toEqual(
			['Quill.yaml', 'assets/logo.txt', 'template.typ'],
		);
		for (const [path, content] of Object.entries(files)) {
			expect(new TextDecoder().decode(unpacked[path])).toBe(
				new TextDecoder().decode(content),
			);
		}
	});

	it('should handle empty files', () => {
		const files = { 'empty.txt': new Uint8Array(0) };
		const packed = packFiles(files);
		const unpacked = unpackFiles(packed);

		expect(unpacked['empty.txt']).toBeDefined();
		expect(unpacked['empty.txt'].length).toBe(0);
	});

	it('should handle binary content', () => {
		const binary = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]);
		const files = { 'image.jpg': binary };
		const packed = packFiles(files);
		const unpacked = unpackFiles(packed);

		expect(unpacked['image.jpg']).toEqual(binary);
	});

	it('should produce deterministic output regardless of input order', () => {
		const encoder = new TextEncoder();
		const files1: Record<string, Uint8Array> = {
			'b.txt': encoder.encode('B'),
			'a.txt': encoder.encode('A'),
		};
		const files2: Record<string, Uint8Array> = {
			'a.txt': encoder.encode('A'),
			'b.txt': encoder.encode('B'),
		};

		const packed1 = packFiles(files1);
		const packed2 = packFiles(files2);

		expect(packed1).toEqual(packed2);
	});

	it('should handle an empty file map', () => {
		const packed = packFiles({});
		const unpacked = unpackFiles(packed);
		expect(Object.keys(unpacked)).toEqual([]);
	});

	it('should produce a valid ustar tar archive', () => {
		const files = { 'test.txt': new TextEncoder().encode('content') };
		const packed = packFiles(files);

		// Check ustar magic at offset 257 in the first header
		const magic = String.fromCharCode(...packed.subarray(257, 263));
		expect(magic).toBe('ustar\0');

		// Check type flag is '0' (regular file)
		expect(packed[156]).toBe(0x30);
	});

	it('should throw for paths exceeding 100 bytes', () => {
		const longPath = 'a'.repeat(101);
		const files = { [longPath]: new Uint8Array(0) };
		expect(() => packFiles(files)).toThrow('exceeds 100 bytes');
	});

	it('should throw for corrupted archive with invalid size', () => {
		// Create a valid archive, then corrupt the size field
		const packed = packFiles({ 'test.txt': new TextEncoder().encode('hi') });
		const corrupted = new Uint8Array(packed);
		// Overwrite size field (offset 124–135) with garbage
		for (let i = 124; i < 136; i++) corrupted[i] = 0x58; // 'X'
		expect(() => unpackFiles(corrupted)).toThrow('invalid size field');
	});

	it('should throw for truncated archive', () => {
		const packed = packFiles({ 'test.txt': new TextEncoder().encode('hello world') });
		// Truncate: keep header but cut the data short
		const truncated = packed.slice(0, 512 + 2);
		expect(() => unpackFiles(truncated)).toThrow('extends beyond data');
	});
});
