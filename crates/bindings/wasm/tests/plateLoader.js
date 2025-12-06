/**
 * Helper utilities for loading Quill directories into WASM-compatible JSON format
 */

import * as fs from 'fs';
import * as path from 'path';
import { Quillmark } from '@quillmark-wasm'



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
export function loadPlate(quillPath) {
  const files = loadDirectory(quillPath);

  return {
    files: files
  };
}