/**
 * Helper utilities for working with Quillmark WASM API in tests
 *
 * The WASM API uses serde-wasm-bindgen which serializes:
 * - JSON objects as JavaScript Maps
 * - Vec<u8> as JavaScript Arrays (not Uint8Array)
 *
 * These helpers provide a nicer interface for tests.
 */

/**
 * Get a field value from a parsed document
 * @param {Object} parsed - ParsedDocument from Quillmark.parseMarkdown()
 * @param {string} fieldName - Name of the field to get
 * @returns {any} The field value, or undefined if not found
 */
export function getField(parsed, fieldName) {
  if (parsed.fields instanceof Map) {
    return parsed.fields.get(fieldName);
  }
  return parsed.fields[fieldName];
}

/**
 * Get all field names from a parsed document
 * @param {Object} parsed - ParsedDocument from Quillmark.parseMarkdown()
 * @returns {string[]} Array of field names
 */
export function getFieldNames(parsed) {
  if (parsed.fields instanceof Map) {
    return Array.from(parsed.fields.keys()).filter(k => k !== 'body');
  }
  return Object.keys(parsed.fields || {}).filter(k => k !== 'body');
}

/**
 * Convert bytes array to Uint8Array
 * @param {Array|Uint8Array} bytes - Bytes from artifact
 * @returns {Uint8Array} Uint8Array representation
 */
export function toUint8Array(bytes) {
  if (bytes instanceof Uint8Array) {
    return bytes;
  }
  return new Uint8Array(bytes);
}

/**
 * Check if artifact bytes represent a valid PDF
 * @param {Array|Uint8Array} bytes - Bytes to check
 * @returns {boolean} True if bytes start with PDF magic number
 */
export function isPDF(bytes) {
  const arr = toUint8Array(bytes);
  const header = new TextDecoder().decode(arr.slice(0, 4));
  return header === '%PDF';
}

/**
 * Check if artifact bytes represent a valid SVG
 * @param {Array|Uint8Array} bytes - Bytes to check
 * @returns {boolean} True if bytes contain SVG content
 */
export function isSVG(bytes) {
  const arr = toUint8Array(bytes);
  const text = new TextDecoder().decode(arr);
  return text.includes('<svg') || text.includes('<?xml');
}
