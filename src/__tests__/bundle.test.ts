import { describe, it, expect } from 'vitest';
import { packFiles, unpackFiles } from '../bundle.js';

describe('packFiles / unpackFiles', () => {
	it('should round-trip a single file', async () => {
		const files = { 'hello.txt': new TextEncoder().encode('Hello World') };
		const packed = await packFiles(files);
		const unpacked = await unpackFiles(packed);

		expect(Object.keys(unpacked)).toEqual(['hello.txt']);
		expect(new TextDecoder().decode(unpacked['hello.txt'])).toBe('Hello World');
	});

	it('should round-trip multiple files', async () => {
		const encoder = new TextEncoder();
		const files: Record<string, Uint8Array> = {
			'Quill.yaml': encoder.encode('name: test\nversion: 1.0.0'),
			'template.typ': encoder.encode('// template'),
			'assets/logo.txt': encoder.encode('logo-data'),
		};
		const packed = await packFiles(files);
		const unpacked = await unpackFiles(packed);

		expect(Object.keys(unpacked).sort()).toEqual(
			['Quill.yaml', 'assets/logo.txt', 'template.typ'],
		);
		for (const [path, content] of Object.entries(files)) {
			expect(new TextDecoder().decode(unpacked[path])).toBe(
				new TextDecoder().decode(content),
			);
		}
	});

	it('should handle empty files', async () => {
		const files = { 'empty.txt': new Uint8Array(0) };
		const packed = await packFiles(files);
		const unpacked = await unpackFiles(packed);

		expect(unpacked['empty.txt']).toBeDefined();
		expect(unpacked['empty.txt'].length).toBe(0);
	});

	it('should handle binary content', async () => {
		const binary = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]);
		const files = { 'image.jpg': binary };
		const packed = await packFiles(files);
		const unpacked = await unpackFiles(packed);

		expect(unpacked['image.jpg']).toEqual(binary);
	});

	it('should produce deterministic output regardless of input order', async () => {
		const encoder = new TextEncoder();
		const files1: Record<string, Uint8Array> = {
			'b.txt': encoder.encode('B'),
			'a.txt': encoder.encode('A'),
		};
		const files2: Record<string, Uint8Array> = {
			'a.txt': encoder.encode('A'),
			'b.txt': encoder.encode('B'),
		};

		const packed1 = await packFiles(files1);
		const packed2 = await packFiles(files2);

		expect(packed1).toEqual(packed2);
	});

	it('should handle an empty file map', async () => {
		const packed = await packFiles({});
		const unpacked = await unpackFiles(packed);
		expect(Object.keys(unpacked)).toEqual([]);
	});
});
