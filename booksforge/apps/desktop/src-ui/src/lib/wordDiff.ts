/**
 * Lightweight word-level diff via longest-common-subsequence (LCS).
 *
 * Returns a stream of `equal | remove | add` segments suitable for
 * rendering in a reviewer panel.  Pure logic, dependency-free.
 *
 * Token boundary: `\w+` (word chars), and any contiguous run of
 * non-word characters (whitespace, punctuation) is its own token.
 * That keeps spaces / punctuation visually intact across the diff.
 */

export type DiffOp = "equal" | "remove" | "add";

export interface DiffSegment {
  op:   DiffOp;
  text: string;
}

/** Tokenise a string into a flat array of word + non-word runs. */
export function tokenize(s: string): string[] {
  const out: string[] = [];
  let i = 0;
  const isWord = (ch: string) => /[\p{L}\p{N}_]/u.test(ch);
  while (i < s.length) {
    const start = i;
    const word = isWord(s[i]!);
    while (i < s.length && isWord(s[i]!) === word) i++;
    out.push(s.slice(start, i));
  }
  return out;
}

/** Compute a coalesced word-level diff. */
export function wordDiff(before: string, after: string): DiffSegment[] {
  if (before === after) {
    return before.length === 0 ? [] : [{ op: "equal", text: before }];
  }
  const a = tokenize(before);
  const b = tokenize(after);

  // LCS DP table.  Caps at ~10k tokens per side; for typical paragraphs
  // a manuscript polish operates on, this is comfortable.
  const m = a.length, n = b.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = m - 1; i >= 0; i--) {
    for (let j = n - 1; j >= 0; j--) {
      dp[i]![j] = a[i] === b[j]
        ? dp[i + 1]![j + 1]! + 1
        : Math.max(dp[i + 1]![j]!, dp[i]![j + 1]!);
    }
  }

  // Walk back through the table coalescing same-op tokens.
  const segments: DiffSegment[] = [];
  let i = 0, j = 0;
  while (i < m && j < n) {
    if (a[i] === b[j]) {
      pushOrExtend(segments, "equal", a[i]!);
      i++; j++;
    } else if (dp[i + 1]![j]! >= dp[i]![j + 1]!) {
      pushOrExtend(segments, "remove", a[i]!);
      i++;
    } else {
      pushOrExtend(segments, "add", b[j]!);
      j++;
    }
  }
  while (i < m) { pushOrExtend(segments, "remove", a[i]!); i++; }
  while (j < n) { pushOrExtend(segments, "add",    b[j]!); j++; }

  return segments;
}

function pushOrExtend(out: DiffSegment[], op: DiffOp, text: string): void {
  const last = out[out.length - 1];
  if (last && last.op === op) last.text += text;
  else out.push({ op, text });
}
