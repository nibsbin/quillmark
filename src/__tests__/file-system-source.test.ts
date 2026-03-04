import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { FileSystemSource } from '../sources/file-system-source.js';
import { RegistryError } from '../errors.js';
import JSZip from 'jszip';

const TEST_DIR = path.join(import.meta.dirname, '../../.test-fixtures/quills');
const OUTPUT_DIR = path.join(import.meta.dirname, '../../.test-fixtures/output');

async function createQuillDir(name: string, version: string, description?: string) {
	const quillDir = path.join(TEST_DIR, name);
	await fs.mkdir(quillDir, { recursive: true });

	const yaml = [
		`name: ${name}`,
		`version: ${version}`,
		...(description ? [`description: ${description}`] : []),
	].join('\n');

	await fs.writeFile(path.join(quillDir, 'Quill.yaml'), yaml);
	await fs.writeFile(path.join(quillDir, 'template.typ'), `// Template for ${name}`);

	// Create a subdirectory with an asset
	const assetsDir = path.join(quillDir, 'assets');
	await fs.mkdir(assetsDir, { recursive: true });
	await fs.writeFile(path.join(assetsDir, 'logo.txt'), 'logo-placeholder');
}

describe('FileSystemSource', () => {
	beforeEach(async () => {
		await fs.rm(TEST_DIR, { recursive: true, force: true });
		await fs.rm(OUTPUT_DIR, { recursive: true, force: true });
		await fs.mkdir(TEST_DIR, { recursive: true });
	});

	afterEach(async () => {
		await fs.rm(path.join(import.meta.dirname, '../../.test-fixtures'), {
			recursive: true,
			force: true,
		});
	});

	describe('getManifest()', () => {
		it('should return a manifest with all quills', async () => {
			await createQuillDir('usaf_memo', '1.0.0', 'USAF Memo');
			await createQuillDir('classic_resume', '2.1.0');

			const source = new FileSystemSource(TEST_DIR);
			const manifest = await source.getManifest();

			expect(manifest.quills).toHaveLength(2);
			const names = manifest.quills.map((q) => q.name).sort();
			expect(names).toEqual(['classic_resume', 'usaf_memo']);

			const usaf = manifest.quills.find((q) => q.name === 'usaf_memo')!;
			expect(usaf.version).toBe('1.0.0');
			expect(usaf.description).toBe('USAF Memo');
		});

		it('should return empty manifest for empty directory', async () => {
			const source = new FileSystemSource(TEST_DIR);
			const manifest = await source.getManifest();
			expect(manifest.quills).toEqual([]);
		});

		it('should throw source_unavailable for non-existent directory', async () => {
			const source = new FileSystemSource('/nonexistent/path');
			await expect(source.getManifest()).rejects.toThrow(RegistryError);
			try {
				await source.getManifest();
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('source_unavailable');
			}
		});
	});

	describe('loadQuill()', () => {
		it('should load a quill by name', async () => {
			await createQuillDir('usaf_memo', '1.0.0', 'USAF Memo');

			const source = new FileSystemSource(TEST_DIR);
			const bundle = await source.loadQuill('usaf_memo');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('1.0.0');
			expect(bundle.metadata.name).toBe('usaf_memo');
			expect(bundle.metadata.version).toBe('1.0.0');
			expect(bundle.metadata.description).toBe('USAF Memo');

			// Data should contain the quill files
			const data = bundle.data as Record<string, Uint8Array>;
			expect(data['Quill.yaml']).toBeDefined();
			expect(data['template.typ']).toBeDefined();
			expect(data['assets/logo.txt']).toBeDefined();
		});

		it('should load a quill by name and version', async () => {
			await createQuillDir('usaf_memo', '1.0.0');

			const source = new FileSystemSource(TEST_DIR);
			const bundle = await source.loadQuill('usaf_memo', '1.0.0');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('1.0.0');
		});

		it('should throw quill_not_found for unknown quill', async () => {
			await createQuillDir('usaf_memo', '1.0.0');

			const source = new FileSystemSource(TEST_DIR);
			try {
				await source.loadQuill('nonexistent');
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('quill_not_found');
				expect((err as RegistryError).quillName).toBe('nonexistent');
			}
		});

		it('should throw version_not_found when quill exists but version does not', async () => {
			await createQuillDir('usaf_memo', '1.0.0');

			const source = new FileSystemSource(TEST_DIR);
			try {
				await source.loadQuill('usaf_memo', '2.0.0');
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('version_not_found');
				expect((err as RegistryError).quillName).toBe('usaf_memo');
				expect((err as RegistryError).version).toBe('2.0.0');
			}
		});
	});

	describe('packageForHttp()', () => {
		it('should write zips and manifest.json to output directory', async () => {
			await createQuillDir('usaf_memo', '1.0.0', 'USAF Memo');
			await createQuillDir('classic_resume', '2.1.0');

			const source = new FileSystemSource(TEST_DIR);
			await source.packageForHttp(OUTPUT_DIR);

			// Verify manifest.json was written
			const manifestPath = path.join(OUTPUT_DIR, 'manifest.json');
			const manifestContent = JSON.parse(await fs.readFile(manifestPath, 'utf-8'));
			expect(manifestContent.quills).toHaveLength(2);

			// Verify zip files were written
			const files = await fs.readdir(OUTPUT_DIR);
			expect(files).toContain('usaf_memo@1.0.0.zip');
			expect(files).toContain('classic_resume@2.1.0.zip');
			expect(files).toContain('manifest.json');

			// Verify zip contents
			const zipData = await fs.readFile(path.join(OUTPUT_DIR, 'usaf_memo@1.0.0.zip'));
			const zip = await JSZip.loadAsync(zipData);
			expect(zip.file('Quill.yaml')).not.toBeNull();
			expect(zip.file('template.typ')).not.toBeNull();
			expect(zip.file('assets/logo.txt')).not.toBeNull();
		});

		it('should create output directory if it does not exist', async () => {
			await createQuillDir('usaf_memo', '1.0.0');

			const source = new FileSystemSource(TEST_DIR);
			const nestedOutput = path.join(OUTPUT_DIR, 'nested', 'dir');
			await source.packageForHttp(nestedOutput);

			const files = await fs.readdir(nestedOutput);
			expect(files).toContain('manifest.json');
		});
	});
});
