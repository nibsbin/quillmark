/**
 * Helper utilities for loading Quill directories into WASM-compatible JSON format
 */

import * as fs from 'fs';
import * as path from 'path';

/**
 * Recursively load a directory structure into the Quill JSON format
 * @param {string} dirPath - Path to directory to load
 * @returns {object} - Directory structure as nested objects with {contents: ...}
 */
function loadDirectory(dirPath) {
  const result = {};
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);

    if (entry.isDirectory()) {
      // Recursively load subdirectories
      result[entry.name] = loadDirectory(fullPath);
    } else if (entry.isFile()) {
      // Check if file is binary based on extension
      const isBinary = /\.(png|jpg|jpeg|gif|pdf|woff|woff2|ttf|otf)$/i.test(entry.name);

      if (isBinary) {
        // Load as byte array
        const buffer = fs.readFileSync(fullPath);
        result[entry.name] = {
          contents: Array.from(buffer)
        };
      } else {
        // Load as UTF-8 string
        const text = fs.readFileSync(fullPath, 'utf8');
        result[entry.name] = {
          contents: text
        };
      }
    }
  }

  return result;
}

/**
 * Load a Quill directory into WASM-compatible JSON format
 * @param {string} quillPath - Path to Quill directory
 * @returns {object} - Quill JSON with {files: {...}}
 */
export function loadQuill(quillPath) {
  const files = loadDirectory(quillPath);

  return {
    files: files
  };
}

/**
 * Load the markdown example from a Quill directory
 * @param {string} quillPath - Path to Quill directory
 * @param {string} markdownFile - Name of markdown file (default: from Quill.toml)
 * @returns {string} - Markdown content
 */
export function loadQuillMarkdown(quillPath, markdownFile = null) {
  if (!markdownFile) {
    // Try to parse Quill.toml to find example field
    const tomlPath = path.join(quillPath, 'Quill.toml');
    const tomlContent = fs.readFileSync(tomlPath, 'utf8');
    
    // Simple regex to extract example = "filename"
    const match = tomlContent.match(/example\s*=\s*"([^"]+)"/);
    if (match) {
      markdownFile = match[1];
    } else {
      throw new Error('No markdown file specified and no example field found in Quill.toml');
    }
  }

  const mdPath = path.join(quillPath, markdownFile);
  return fs.readFileSync(mdPath, 'utf8');
}
