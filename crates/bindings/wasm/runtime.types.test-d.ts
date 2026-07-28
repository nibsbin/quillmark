// Type-level DRIFT GUARD for the canonical render types.
//
// `runtime/runtime.d.ts` defines the render-side types (`RenderResult`,
// `RenderOptions`, `Artifact`, `OutputFormat`, `PageSize`, `PaintOptions`,
// `PaintResult`) as the backend-NEUTRAL canonical contract, rather than
// re-exporting them from the private Typst backend build. This file asserts that
// those canonical declarations and the Typst backend's GENERATED declarations
// (`pkg/backends/typst/wasm.d.ts`, produced from the `typescript_custom_section`
// blocks in `crates/bindings/wasm/src/engine.rs`) stay mutually assignable — if
// either side drifts, one of the assignments below stops compiling.
//
// Run via `npm run typecheck` (tsc --noEmit). This file emits no runtime code.

import type {
	RenderResult as CanonicalRenderResult,
	RenderOptions as CanonicalRenderOptions,
	Artifact as CanonicalArtifact,
	OutputFormat as CanonicalOutputFormat,
	PageSize as CanonicalPageSize,
	PaintOptions as CanonicalPaintOptions,
	PaintResult as CanonicalPaintResult,
	FieldRegion as CanonicalFieldRegion,
	ChangeSet as CanonicalChangeSet,
	ContentHit as CanonicalContentHit
  // The BUILT copy (synced from `runtime/runtime.d.ts` by build-wasm.sh / the
  // cp step), because only there does the d.ts's own `../core/wasm.js` import
  // resolve to the generated `pkg/core` build. The two copies are byte-identical.
} from '../../../pkg/runtime/runtime.d.ts';

import type {
	RenderResult as TypstRenderResult,
	RenderOptions as TypstRenderOptions,
	Artifact as TypstArtifact,
	OutputFormat as TypstOutputFormat,
	PageSize as TypstPageSize,
	PaintOptions as TypstPaintOptions,
	PaintResult as TypstPaintResult,
	FieldRegion as TypstFieldRegion,
	ChangeSet as TypstChangeSet,
	ContentHit as TypstContentHit
} from '../../../pkg/backends/typst/wasm';

// One mutual-assignability pair per hoisted type: typst → canonical and
// canonical → typst. `void` the bindings so "declared but never read" is not an
// error under noUnusedLocals.
//
// Mutual assignability alone cannot catch a missing OPTIONAL member: for an
// all-optional interface pair (RenderOptions, PaintOptions) both assignments
// compile even when one side lacks a member entirely. The `KeysEqual`
// assertions close that hole — `true` only when both sides declare exactly
// the same property names.

type KeysEqual<A, B> = [Exclude<keyof A, keyof B>, Exclude<keyof B, keyof A>] extends [
	never,
	never
]
	? true
	: false;

const renderResultA: CanonicalRenderResult = {} as TypstRenderResult;
const renderResultB: TypstRenderResult = {} as CanonicalRenderResult;
void renderResultA;
void renderResultB;

const renderOptionsA: CanonicalRenderOptions = {} as TypstRenderOptions;
const renderOptionsB: TypstRenderOptions = {} as CanonicalRenderOptions;
void renderOptionsA;
void renderOptionsB;

const artifactA: CanonicalArtifact = {} as TypstArtifact;
const artifactB: TypstArtifact = {} as CanonicalArtifact;
void artifactA;
void artifactB;

const outputFormatA: CanonicalOutputFormat = {} as TypstOutputFormat;
const outputFormatB: TypstOutputFormat = {} as CanonicalOutputFormat;
void outputFormatA;
void outputFormatB;

const pageSizeA: CanonicalPageSize = {} as TypstPageSize;
const pageSizeB: TypstPageSize = {} as CanonicalPageSize;
void pageSizeA;
void pageSizeB;

const paintOptionsA: CanonicalPaintOptions = {} as TypstPaintOptions;
const paintOptionsB: TypstPaintOptions = {} as CanonicalPaintOptions;
void paintOptionsA;
void paintOptionsB;

const paintResultA: CanonicalPaintResult = {} as TypstPaintResult;
const paintResultB: TypstPaintResult = {} as CanonicalPaintResult;
void paintResultA;
void paintResultB;

const fieldRegionA: CanonicalFieldRegion = {} as TypstFieldRegion;
const fieldRegionB: TypstFieldRegion = {} as CanonicalFieldRegion;
void fieldRegionA;
void fieldRegionB;

const changeSetA: CanonicalChangeSet = {} as TypstChangeSet;
const changeSetB: TypstChangeSet = {} as CanonicalChangeSet;
void changeSetA;
void changeSetB;

const contentHitA: CanonicalContentHit = {} as TypstContentHit;
const contentHitB: TypstContentHit = {} as CanonicalContentHit;
void contentHitA;
void contentHitB;

const renderResultKeys: KeysEqual<CanonicalRenderResult, TypstRenderResult> = true;
const renderOptionsKeys: KeysEqual<CanonicalRenderOptions, TypstRenderOptions> = true;
const artifactKeys: KeysEqual<CanonicalArtifact, TypstArtifact> = true;
const pageSizeKeys: KeysEqual<CanonicalPageSize, TypstPageSize> = true;
const paintOptionsKeys: KeysEqual<CanonicalPaintOptions, TypstPaintOptions> = true;
const paintResultKeys: KeysEqual<CanonicalPaintResult, TypstPaintResult> = true;
const fieldRegionKeys: KeysEqual<CanonicalFieldRegion, TypstFieldRegion> = true;
const changeSetKeys: KeysEqual<CanonicalChangeSet, TypstChangeSet> = true;
const contentHitKeys: KeysEqual<CanonicalContentHit, TypstContentHit> = true;
void renderResultKeys;
void renderOptionsKeys;
void artifactKeys;
void pageSizeKeys;
void paintOptionsKeys;
void paintResultKeys;
void fieldRegionKeys;
void changeSetKeys;
void contentHitKeys;

// ── Re-export presence guard (#948) ─────────────────────────────────────────
// The content edit vocabulary is DECLARED in the core build but consumed through
// the single runtime entry point. Importing every name from the runtime root
// here asserts the re-export in `runtime/runtime.d.ts` stays present: drop any
// one and this import stops resolving, failing `npm run typecheck`. Type-only —
// no runtime code, no assignability claim, pure existence.
import type {
	Content,
	ContentLine,
	ContentLineKind,
	ContentContainer,
	ContentMark,
	ContentIsland,
	TableProps,
	ImageProps,
	TableCell,
	CardInput,
	PathStep,
	Addr,
	CardAddr,
	Delta,
	Assoc,
	LineOp,
	MarkOp,
	ChangeBundle
} from '../../../pkg/runtime/runtime.d.ts';

// Referencing each name in an exported tuple keeps the import "used" without a
// runtime statement; an exported alias is never an unused-local error.
export type ContentExportsPresent = [
	Content,
	ContentLine,
	ContentLineKind,
	ContentContainer,
	ContentMark,
	ContentIsland,
	TableProps,
	ImageProps,
	TableCell,
	CardInput,
	PathStep,
	Addr,
	CardAddr,
	Delta,
	Assoc,
	LineOp,
	MarkOp,
	ChangeBundle
];

// ── MAIN_CARD_ADDR is a CardAddr (#969) ─────────────────────────────────────
// The named main-card address must type as a `CardAddr` so it flows into every
// card-scoped verb's address slot. `typeof import(...)` keeps this purely
// type-level — no value import, no runtime code — and the assignment fails
// `npm run typecheck` if the constant's declared type ever drifts off `CardAddr`.
type MainCardAddrType = typeof import('../../../pkg/runtime/runtime.d.ts').MAIN_CARD_ADDR;
const mainCardAddrIsCardAddr: CardAddr = {} as MainCardAddrType;
void mainCardAddrIsCardAddr;

// ── Open-set discriminant guards (#1042) ────────────────────────────────────
// The guards must NARROW the open `type` unions — the whole point, since a bare
// `x.type === 'table'` check cannot (the residual `{ type: string; … }` arm
// stays live). Each `if` body reads a payload reachable only after narrowing, so
// a guard that stops narrowing fails `npm run typecheck`. `ContentIsland`,
// `TableProps`, `ImageProps`, `ContentMark`, `ContentLine`, and
// `ContentContainer` are the types imported above. (#1054 opened the block
// vocabulary — `kind` and `container` — on the same terms.)
import {
	isTableIsland,
	isImageIsland,
	isLinkMark,
	isAnchorMark,
	isHeadingLine,
	isCodeLine,
	isListItemContainer
} from '../../../pkg/runtime/runtime.js';

declare const guardIsland: ContentIsland;
if (isTableIsland(guardIsland)) {
	const tableProps: TableProps = guardIsland.props;
	void tableProps;
}
if (isImageIsland(guardIsland)) {
	const imageProps: ImageProps = guardIsland.props;
	void imageProps;
}

declare const guardMark: ContentMark;
if (isLinkMark(guardMark)) {
	const url: string = guardMark.url;
	void url;
}
if (isAnchorMark(guardMark)) {
	const id: string = guardMark.id;
	void id;
}

declare const guardLine: ContentLine;
if (isHeadingLine(guardLine)) {
	const level: number = guardLine.level;
	void level;
}
if (isCodeLine(guardLine)) {
	const lang: string | undefined = guardLine.lang;
	void lang;
}

declare const guardContainer: ContentContainer;
if (isListItemContainer(guardContainer)) {
	const ordinal: number = guardContainer.ordinal;
	void ordinal;
}

// ── Open-set membership guards (#1085) ──────────────────────────────────────
// The negative predicates the pinned-arm guards above cannot express. Each `if`
// body reads the open arm's opaque payload, reachable only after narrowing, so a
// guard that stops narrowing fails `npm run typecheck`.
import {
	isUnknownLine,
	isUnknownContainer,
	isUnknownMark,
	isUnknownIsland
} from '../../../pkg/runtime/runtime.js';

if (isUnknownLine(guardLine)) {
	const attrs: unknown = guardLine.attrs;
	void attrs;
}
if (isUnknownContainer(guardContainer)) {
	const attrs: unknown = guardContainer.attrs;
	void attrs;
}
if (isUnknownMark(guardMark)) {
	const attrs: unknown = guardMark.attrs;
	void attrs;
}
if (isUnknownIsland(guardIsland)) {
	const props: unknown = guardIsland.props;
	void props;
}

// ── ContentLineKind is nameable (#1086) ─────────────────────────────────────
// `ContentLineKind` is exactly `setKind`'s payload, so building the op is a
// whole-lift: drop a line's envelope, spread the rest. That spelling survives
// every arm added upstream — including the open one, whose shape an arm-by-arm
// switch would have to guess at. It only type-checks if the type is nameable
// from the package entry point, which is the point of the re-export.
function kindPart(line: ContentLine): ContentLineKind {
	const { containers, continues, ...kind } = line;
	void containers;
	void continues;
	return kind;
}
declare const liftLine: ContentLine;
const liftedOp: LineOp = { op: 'setKind', line: 0, ...kindPart(liftLine) };
void liftedOp;
