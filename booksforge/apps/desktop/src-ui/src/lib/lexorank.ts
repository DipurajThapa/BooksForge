/**
 * Frontend LexoRank position helpers.
 *
 * Mirrors the Rust side in
 * `crates/booksforge-domain/src/lexorank.rs`: positions are
 * `"0|<rank>:"` strings where `<rank>` is base-36 chars, sorted
 * lexicographically. The Rust side mints initial positions evenly
 * spaced between `"100000"` and `"yzzzzz"`; the frontend only needs
 * to mint *new* positions that sort after existing siblings (for
 * create-at-end). Drag-reorder + insert-between is a follow-up PR.
 *
 * If the existing positions don't match the canonical format (e.g.
 * legacy data or external imports), the helpers fall back to plain
 * lexicographic append (`max + "z"`) which still sorts correctly
 * and is accepted by the Rust storage layer (no format validation
 * on the backend per current `commands/nodes.rs`).
 */

const BUCKET   = "0|";
const RANK_LEN = 6;
const RANK_MIN = "100000"; // base-36; matches Rust constant
const RANK_MAX = "yzzzzz"; // base-36; matches Rust constant

function parseRank(position: string): number | null {
  if (!position.startsWith(BUCKET) || !position.endsWith(":")) return null;
  const rank = position.slice(BUCKET.length, -1);
  if (rank.length !== RANK_LEN) return null;
  const v = parseInt(rank, 36);
  return Number.isFinite(v) ? v : null;
}

function formatRank(value: number): string {
  // pad to RANK_LEN, base-36
  const s = Math.max(0, Math.floor(value)).toString(36).padStart(RANK_LEN, "0");
  return `${BUCKET}${s}:`;
}

/**
 * Compute a position string that sorts strictly after every value
 * in `siblings`. When `siblings` is empty, returns a mid-bucket
 * value so the next "before" insert has room to spread.
 *
 * Strategy:
 *   - If the current max parses cleanly, mint a value half-way
 *     between max and `RANK_MAX`. Halves the available space each
 *     time, but with 2.1B distinct values per bucket the writer
 *     can create ~30 scenes back-to-back before exhausting room.
 *   - If parsing fails, append "z" to the max position string.
 *     Both forms are accepted by the storage layer and still sort
 *     correctly under `localeCompare`.
 */
export function positionAtEnd(siblings: Array<{ position: string }>): string {
  if (siblings.length === 0) {
    const lo = parseInt(RANK_MIN, 36);
    const hi = parseInt(RANK_MAX, 36);
    return formatRank((lo + hi) / 2);
  }
  const max = siblings
    .map((s) => s.position)
    .reduce((a, b) => (a > b ? a : b));
  const parsed = parseRank(max);
  const ceiling = parseInt(RANK_MAX, 36);
  if (parsed === null || parsed >= ceiling) {
    return `${max}z`;
  }
  const next = parsed + Math.max(1, Math.floor((ceiling - parsed) / 2));
  if (next >= ceiling) return `${max}z`;
  return formatRank(next);
}
