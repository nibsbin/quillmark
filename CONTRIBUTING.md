# Contributing to @quillmark/registry

Thanks for your interest in contributing! This guide covers the basics to get you up and running.

## Prerequisites

- **Node.js 24+**
- **npm** (ships with Node)

## Getting Started

```bash
git clone https://github.com/nibsbin/quillmark-registry.git
cd quillmark-registry
npm install
```

## Development Workflow

| Command | Purpose |
|---|---|
| `npm run build` | Compile TypeScript (`tsc`) |
| `npm test` | Run the test suite once (Vitest) |
| `npm run test:watch` | Run tests in watch mode |
| `npm run lint` | Type-check without emitting (`tsc --noEmit`) |

Make sure **both lint and tests pass** before opening a pull request:

```bash
npm run lint && npm test
```

## Project Structure

```
src/
├── index.ts              # Public API exports
├── registry.ts           # QuillRegistry orchestrator
├── bundle.ts             # QuillBundle helpers
├── errors.ts             # RegistryError types
├── format.ts             # Format utilities
├── types.ts              # Shared type definitions
├── validate.ts           # Validation logic
├── sources/
│   ├── http-source.ts    # Browser/Node HTTP source
│   └── file-system-source.ts  # Node filesystem source
└── __tests__/            # Vitest test files
```

## Writing Tests

Tests live in `src/__tests__/` and use [Vitest](https://vitest.dev/) with globals enabled. Name test files `<module>.test.ts` to match existing conventions.

## Submitting Changes

1. Fork the repo and create a feature branch from `main`.
2. Make your changes — keep commits focused and well-described.
3. Ensure `npm run lint && npm test` passes.
4. Open a pull request with a clear description of what changed and why.

## Reporting Bugs

Open an issue at [github.com/nibsbin/quillmark-registry/issues](https://github.com/nibsbin/quillmark-registry/issues) with:

- Steps to reproduce
- Expected vs. actual behavior
- Node.js version and environment (browser / Node)

## License

By contributing, you agree that your contributions will be licensed under the [Apache-2.0 License](./LICENSE).
