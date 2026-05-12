/**
 * Frontend LexoRank position helpers.
 *
 * Mirrors the Rust side in
 * `crates/booksforge-domain/src/lexorank.rs`: positions are
 * `"0|<rank>:"` strings where `<rank>` is base-36 chars, sorted
 * lexicographically. The Rust side mints initial positions evenly
 * spaced between `"100000"` and `"yzzzzz"`; the frontend mints
 * new positions for create-at-end (`positionAtEnd`) and for
 * insert-between drag-reorder (`positionBetween`).
 *
 * If the existing positions don't match the canonical format (e.g.
 * legacy data or external imports), the helpers fall back to plain
 * lexicographic append/insert (`max + "z"`, `prev + "m"`) which
 * still sort correctly and are accepted by the Rust storage layer
 * (no format validation on the backend per current `commands/nodes.rs`).
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

/**
 * Mint a position string strictly between `prev` and `next` —
 * used by drag-reorder.
 *
 * `prev = null` means "land at the top" (between `RANK_MIN` and
 * `next`). `next = null` means "land at the bottom" (between `prev`
 * and `RANK_MAX`). Both null returns a mid-bucket value.
 *
 * The drag-reorder UI places `prev` and `next` directly adjacent
 * in the sibling list, so the midpoint is always unambiguous.
 *
 * Fallback when either endpoint doesn't parse cleanly or there's
 * no integer room between them: lexicographic string insertion.
 * `prev + "m"` sorts strictly after `prev` and (because `next`'s
 * 7th+ characters are absent in the canonical 6-char form) sorts
 * before `next` in the practical adjacent case. The Rust storage
 * layer accepts arbitrary strings (no format validation), so the
 * fallback round-trips fine.
 */
export function positionBetween(
  prev: string | null,
  next: string | null,
): string {
  const lo = parseInt(RANK_MIN, 36);
  const hi = parseInt(RANK_MAX, 36);
  const prevValue = prev != null ? parseRank(prev) : lo;
  const nextValue = next != null ? parseRank(next) : hi;

  if (prevValue !== null && nextValue !== null && nextValue > prevValue + 1) {
    const mid = prevValue + Math.floor((nextValue - prevValue) / 2);
    return formatRank(mid);
  }
  if (prev != null) return `${prev}m`;
  if (next != null) {
    // No `prev` — insert at the floor by minting a value strictly
    // below `next`. Use the rank-min integer to be safe.
    return formatRank(lo);
  }
  return formatRank((lo + hi) / 2);
}
