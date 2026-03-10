import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { FileSystemSource } from '../sources/file-system-source.js';
import { RegistryError } from '../errors.js';
import JSZip from 'jszip';

const TEST_DIR = path.join(import.meta.dirname, '../../.test-fixtures/quills');
const OUTPUT_DIR = path.join(import.meta.dirname, '../../.test-fixtures/output');

async function createQuillDir(name: string, version: string, description?: string) {
	const quillDir = path.join(TEST_DIR, name, version);
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
		it('should return a manifest with all quill versions', async () => {
			await createQuillDir('usaf_memo', '1.0.0', 'USAF Memo');
			await createQuillDir('classic_resume', '2.1.0');

			const source = new FileSystemSource(TEST_DIR);
			const manifest = await source.getManifest();

			expect(manifest.quills).toHaveLength(2);
			const names = manifest.quills.map((q) => q.name).sort();
			expect(names).toEqual(['classic_resume', 'usaf_memo']);

			const usaf = manifest.quills.find((q) => q.name === 'usaf_memo')!;
			expect(usaf.version).toBe('1.0.0');
		});

		it('should return multiple versions of the same quill', async () => {
			await createQuillDir('usaf_memo', '0.1.0', 'USAF Memo v0.1');
			await createQuillDir('usaf_memo', '1.0.0', 'USAF Memo v1.0');

			const source = new FileSystemSource(TEST_DIR);
			const manifest = await source.getManifest();

			expect(manifest.quills).toHaveLength(2);
			const versions = manifest.quills.map((q) => q.version).sort();
			expect(versions).toEqual(['0.1.0', '1.0.0']);
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

		it('should throw load_error when Quill.yaml is missing from version directory', async () => {
			// Create a directory structure without Quill.yaml
			const quillDir = path.join(TEST_DIR, 'usaf_memo', '1.0.0');
			await fs.mkdir(quillDir, { recursive: true });
			await fs.writeFile(path.join(quillDir, 'template.typ'), '// template');

			const source = new FileSystemSource(TEST_DIR);
			await expect(source.getManifest()).rejects.toThrow(RegistryError);
			try {
				await source.getManifest();
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('load_error');
			}
		});
	});

	describe('loadQuill()', () => {
		it('should load a quill by name (resolves to latest version)', async () => {
			await createQuillDir('usaf_memo', '0.1.0', 'USAF Memo v0.1');
			await createQuillDir('usaf_memo', '1.0.0', 'USAF Memo v1.0');

			const source = new FileSystemSource(TEST_DIR);
			const bundle = await source.loadQuill('usaf_memo');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('1.0.0');
		});

		it('should load a quill by name and exact version', async () => {
			await createQuillDir('usaf_memo', '0.1.0', 'USAF Memo v0.1');
			await createQuillDir('usaf_memo', '1.0.0', 'USAF Memo v1.0');

			const source = new FileSystemSource(TEST_DIR);
			const bundle = await source.loadQuill('usaf_memo', '0.1.0');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('0.1.0');
		});

		it('should include all quill files in data', async () => {
			await createQuillDir('usaf_memo', '1.0.0', 'USAF Memo');

			const source = new FileSystemSource(TEST_DIR);
			const bundle = await source.loadQuill('usaf_memo', '1.0.0');

			const data = bundle.data as { files: Record<string, unknown> };
			expect(data.files['Quill.yaml']).toBeDefined();
			expect(data.files['template.typ']).toBeDefined();
			const assets = data.files['assets'] as Record<string, unknown>;
			expect(assets['logo.txt']).toBeDefined();
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

	describe('edge cases', () => {
		it('should ignore dot-prefixed directories at the quill name level', async () => {
			await createQuillDir('usaf_memo', '1.0.0');
			// Create a .git directory that should be ignored
			await fs.mkdir(path.join(TEST_DIR, '.git'), { recursive: true });

			const source = new FileSystemSource(TEST_DIR);
			const manifest = await source.getManifest();

			expect(manifest.quills).toHaveLength(1);
			expect(manifest.quills[0].name).toBe('usaf_memo');
		});

		it('should ignore dot-prefixed directories at the version level', async () => {
			await createQuillDir('usaf_memo', '1.0.0');
			// Create a .DS_Store-like directory inside the quill name dir
			await fs.mkdir(path.join(TEST_DIR, 'usaf_memo', '.hidden'), { recursive: true });

			const source = new FileSystemSource(TEST_DIR);
			const manifest = await source.getManifest();

			expect(manifest.quills).toHaveLength(1);
			expect(manifest.quills[0].version).toBe('1.0.0');
		});

		it('should ignore non-semver directories at the version level', async () => {
			await createQuillDir('usaf_memo', '1.0.0');
			// Create a non-semver directory (e.g., "draft") that should be ignored
			const draftDir = path.join(TEST_DIR, 'usaf_memo', 'draft');
			await fs.mkdir(draftDir, { recursive: true });
			await fs.writeFile(
				path.join(draftDir, 'Quill.yaml'),
				'name: usaf_memo\nversion: draft',
			);

			const source = new FileSystemSource(TEST_DIR);
			const manifest = await source.getManifest();

			expect(manifest.quills).toHaveLength(1);
			expect(manifest.quills[0].version).toBe('1.0.0');
		});

		it('should resolve latest ignoring non-semver directories', async () => {
			await createQuillDir('usaf_memo', '0.1.0');
			await createQuillDir('usaf_memo', '1.0.0');
			// Add a non-semver dir that would sort wrong without filtering
			const draftDir = path.join(TEST_DIR, 'usaf_memo', 'draft');
			await fs.mkdir(draftDir, { recursive: true });

			const source = new FileSystemSource(TEST_DIR);
			const bundle = await source.loadQuill('usaf_memo');

			expect(bundle.version).toBe('1.0.0');
		});

		it('should throw load_error from loadQuill when Quill.yaml is missing', async () => {
			// Create a directory structure without Quill.yaml
			const quillDir = path.join(TEST_DIR, 'usaf_memo', '1.0.0');
			await fs.mkdir(quillDir, { recursive: true });
			await fs.writeFile(path.join(quillDir, 'template.typ'), '// template');

			const source = new FileSystemSource(TEST_DIR);
			try {
				await source.loadQuill('usaf_memo', '1.0.0');
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('load_error');
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

		it('should package multiple versions of the same quill', async () => {
			await createQuillDir('usaf_memo', '0.1.0');
			await createQuillDir('usaf_memo', '1.0.0');

			const source = new FileSystemSource(TEST_DIR);
			await source.packageForHttp(OUTPUT_DIR);

			const files = await fs.readdir(OUTPUT_DIR);
			expect(files).toContain('usaf_memo@0.1.0.zip');
			expect(files).toContain('usaf_memo@1.0.0.zip');

			const manifestContent = JSON.parse(
				await fs.readFile(path.join(OUTPUT_DIR, 'manifest.json'), 'utf-8'),
			);
			expect(manifestContent.quills).toHaveLength(2);
		});

		it('should create output directory if it does not exist', async () => {
			await createQuillDir('usaf_memo', '1.0.0');

			const source = new FileSystemSource(TEST_DIR);
			const nestedOutput = path.join(OUTPUT_DIR, 'nested', 'dir');
			await source.packageForHttp(nestedOutput);

			const files = await fs.readdir(nestedOutput);
			expect(files).toContain('manifest.json');
		});

		it('should produce deterministic (byte-identical) zip files across runs', async () => {
			await createQuillDir('usaf_memo', '1.0.0', 'USAF Memo');

			const source = new FileSystemSource(TEST_DIR);

			const outputDir1 = path.join(OUTPUT_DIR, 'run1');
			const outputDir2 = path.join(OUTPUT_DIR, 'run2');

			await source.packageForHttp(outputDir1);
			await source.packageForHttp(outputDir2);

			const zip1 = await fs.readFile(path.join(outputDir1, 'usaf_memo@1.0.0.zip'));
			const zip2 = await fs.readFile(path.join(outputDir2, 'usaf_memo@1.0.0.zip'));

			expect(Buffer.from(zip1).equals(Buffer.from(zip2))).toBe(true);
		});
	});
});
