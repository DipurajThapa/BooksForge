//! LexoRank-style position strings for sibling ordering.
//!
//! BooksForge uses lexicographic strings (not floats) so positions never
//! collide and never need normalisation.  The format is `"<bucket>|<rank>:"`
//! — only the rank portion changes between siblings.  When inserting between
//! two siblings, callers can mint a new string strictly between them.
//!
//! For MVP we only need the *initial* placement of siblings — a project
//! created from an outline gets evenly-spaced ranks.  In-place reordering is
//! a later concern.
//!
//! All functions are pure.

/// The shared bucket prefix for every MVP rank string.  Keeps all positions
/// in the same ordering bucket.
pub const BUCKET: &str = "0|";

/// Rank-portion alphabet — base-36, lexicographic ordering matches numeric
/// ordering for fixed-length strings.
const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// Default fixed rank-portion width.  Six chars yields ~2.1 B distinct
/// values; far more than enough for any plausible book outline.
const RANK_WIDTH: usize = 6;

/// Generate `n` evenly-spaced rank strings in ascending order.
///
/// Returns strings of the form `"0|<rank>:"` where `<rank>` is six base-36
/// chars.  The first rank is `"100000"`, the last is `"yzzzzz"`, and the
/// remaining `n - 2` are spread evenly between them so future inserts have
/// room on both sides.
///
/// `n == 0` returns an empty `Vec`.
/// `n == 1` returns a single mid-bucket rank.
pub fn initial_positions(n: usize) -> Vec<String> {
    if n == 0 {
        return Vec::new();
    }

    // Lower and upper bounds — leave room above and below for inserts.
    let lo: u64 = base36_to_int(b"100000"); // 60_466_176
    let hi: u64 = base36_to_int(b"yzzzzz"); // 2_176_782_335

    if n == 1 {
        let mid = (lo + hi) / 2;
        return vec![format_rank(mid)];
    }

    let span = hi - lo;
    let step = span / (n as u64 - 1);

    (0..n)
        .map(|i| format_rank(lo + step * (i as u64)))
        .collect()
}

fn format_rank(value: u64) -> String {
    let mut s = int_to_base36(value, RANK_WIDTH);
    s.insert_str(0, BUCKET);
    s.push(':');
    s
}

fn base36_to_int(bytes: &[u8]) -> u64 {
    let mut acc: u64 = 0;
    for &b in bytes {
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as u64,
            b'a'..=b'z' => (b - b'a') as u64 + 10,
            _ => 0,
        };
        acc = acc * 36 + digit;
    }
    acc
}

fn int_to_base36(mut value: u64, width: usize) -> String {
    let mut buf = vec![b'0'; width];
    let mut i = width;
    while value > 0 && i > 0 {
        i -= 1;
        buf[i] = ALPHABET[(value % 36) as usize];
        value /= 36;
    }
    String::from_utf8(buf).unwrap_or_else(|_| "0".repeat(width))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_empty() {
        assert!(initial_positions(0).is_empty());
    }

    #[test]
    fn single_returns_one() {
        assert_eq!(initial_positions(1).len(), 1);
    }

    #[test]
    fn positions_are_strictly_increasing() {
        let ps = initial_positions(50);
        assert_eq!(ps.len(), 50);
        for w in ps.windows(2) {
            assert!(w[0] < w[1], "positions must be strictly increasing: {w:?}");
        }
    }

    #[test]
    fn positions_have_room_above_and_below() {
        let ps = initial_positions(3);
        // First > "0|000000:" so we can insert below.
        assert!(ps.first().unwrap().as_str() > "0|000000:");
        // Last < "0|zzzzzz:" so we can insert above.
        assert!(ps.last().unwrap().as_str() < "0|zzzzzz:");
    }

    #[test]
    fn base36_roundtrip() {
        for v in [0_u64, 1, 35, 36, 1_000_000, 2_176_782_335] {
            let s = int_to_base36(v, RANK_WIDTH);
            assert_eq!(
                base36_to_int(s.as_bytes()),
                v,
                "value {v} round-tripped to {s}"
            );
        }
    }
}
