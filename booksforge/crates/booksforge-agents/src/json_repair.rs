//! Defensive JSON repair before serde deserialise.
//!
//! Local LLMs at 9B–27B occasionally emit a structurally-valid JSON object
//! whose *contents* don't match the declared schema — most often, a list
//! that should hold dicts contains a string-typed placeholder element
//! (e.g. `"characters": [{...}, "characters_2", {...}]`). Hard-failing on
//! this wastes a full retry on what is otherwise a 95%-correct response.
//!
//! `repair_value` walks a `serde_json::Value`, drops obviously-bad list
//! elements (using a caller-supplied predicate), and returns both the
//! cleaned value and a count of edits made for audit logging.
//!
//! Surfaced by BF-E2E-LOCAL-LLM-FIRST-BOOK-001 Phase 5 — the test driver
//! had to add this defensively in Python; this is the production-side
//! version.
//!
//! Usage from a `parse_and_validate`:
//!
//! ```rust,ignore
//! let raw_value: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
//! let (repaired, repairs) = json_repair::repair_value(
//!     raw_value,
//!     &json_repair::DEFAULT_LIST_OF_OBJECTS_KEEP,
//! );
//! if repairs.dropped_list_elements > 0 {
//!     tracing::warn!(
//!         dropped = repairs.dropped_list_elements,
//!         "json_repair: salvaged {} malformed list elements",
//!         repairs.dropped_list_elements,
//!     );
//! }
//! let proposal: T = serde_json::from_value(repaired).map_err(|e| e.to_string())?;
//! ```

use serde_json::Value;

/// Audit counts of edits the repair pass made.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RepairAudit {
    /// Count of list elements that were dropped because they failed `keep`.
    pub dropped_list_elements: usize,
    /// Total number of list nodes the walker examined.
    pub lists_examined: usize,
    /// Field-name corrections made by the schema-aware repair pass —
    /// `(original_key, corrected_key)` for each rename. Empty unless
    /// [`parse_and_repair_with_schema_keys`] or [`repair_field_names`]
    /// was invoked.
    pub field_renames: Vec<(String, String)>,
}

/// A predicate that returns `true` if a list element should be KEPT.
/// The default `DEFAULT_LIST_OF_OBJECTS_KEEP` keeps only `Value::Object`
/// elements; this matches the dominant failure mode (string placeholders
/// like `"characters_2"` slipping into `characters: []`).
pub type KeepPredicate = dyn Fn(&Value, /* parent_key: */ Option<&str>) -> bool + Sync;

/// Default predicate: keep elements that are objects. Drops strings,
/// numbers, bools, nulls, and nested arrays at list positions.
#[allow(non_upper_case_globals)]
pub const DEFAULT_LIST_OF_OBJECTS_KEEP: fn(&Value, Option<&str>) -> bool = |v, _| v.is_object();

/// Permissive variant: keep anything except `Value::Null`. Useful for
/// list-of-strings fields like `keywords` where the LLM may emit a stray
/// null we still want to drop.
#[allow(non_upper_case_globals)]
pub const KEEP_NON_NULL: fn(&Value, Option<&str>) -> bool = |v, _| !v.is_null();

/// Walk `value` recursively, dropping list elements that fail `keep`.
///
/// Returns the cleaned value and an audit struct. The walker is depth-first
/// and visits every nested array and object once.
pub fn repair_value<F>(mut value: Value, keep: &F) -> (Value, RepairAudit)
where
    F: Fn(&Value, Option<&str>) -> bool + ?Sized,
{
    let mut audit = RepairAudit::default();
    repair_inner(&mut value, None, keep, &mut audit);
    (value, audit)
}

fn repair_inner<F>(value: &mut Value, parent_key: Option<&str>, keep: &F, audit: &mut RepairAudit)
where
    F: Fn(&Value, Option<&str>) -> bool + ?Sized,
{
    match value {
        Value::Array(items) => {
            audit.lists_examined += 1;
            let original_len = items.len();
            items.retain(|item| keep(item, parent_key));
            audit.dropped_list_elements += original_len - items.len();
            for item in items.iter_mut() {
                repair_inner(item, parent_key, keep, audit);
            }
        }
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                let k_clone = k.clone();
                repair_inner(v, Some(k_clone.as_str()), keep, audit);
            }
        }
        _ => {}
    }
}

/// Convenience wrapper: parse a raw JSON string and drop only `null` list
/// elements. Safe for ANY schema — the most permissive repair the LLM
/// might benefit from without inventing edits the schema didn't ask for.
///
/// For the stricter "drop non-object elements from lists" repair (the
/// BF-E2E Phase 5 case where a string placeholder slipped into a
/// list-of-objects), use [`parse_and_repair_strict_objects`] in the
/// agent's parse path where the schema is known.
pub fn parse_and_repair(raw: &str) -> Result<(Value, RepairAudit), String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;
    Ok(repair_value(v, &KEEP_NON_NULL))
}

/// Stricter variant: drop list elements that fail the "is an object"
/// predicate at the parent keys named in `object_list_keys`. Other lists
/// (e.g. `voice_traits: Vec<String>`) only have nulls dropped.
///
/// Use this in agent parse paths where the schema declares some lists as
/// `Vec<Struct>` and others as `Vec<String>`.
pub fn parse_and_repair_strict_objects(
    raw: &str,
    object_list_keys: &[&str],
) -> Result<(Value, RepairAudit), String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;
    let owned_keys: Vec<String> = object_list_keys.iter().map(|s| (*s).to_owned()).collect();
    let pred = move |item: &Value, parent_key: Option<&str>| -> bool {
        if let Some(k) = parent_key {
            if owned_keys.iter().any(|ok| ok == k) {
                return item.is_object();
            }
        }
        !item.is_null()
    };
    Ok(repair_value(v, &pred))
}

// ── Schema-aware field-name self-healing ───────────────────────────────────
//
// Run #11 card #2 was rejected because the model emitted `"external_object"`
// instead of the schema's `"external_objective"` (one missing suffix, edit
// distance 4). The whole bible card was discarded; the rest of the
// chunked-bible run had to rely on the lenient retry policy to keep going.
//
// `repair_field_names` walks the value, and for every object key not in the
// declared schema it finds the unique nearest allowed key under a
// [`RepairPolicy`] and rewrites it. Renames are recorded so the audit
// ledger surfaces what was healed and what was left alone.
//
// `RepairPolicy` combines two caps because absolute Levenshtein distance
// alone is the wrong gate. `external_object` → `external_objective` is
// distance 4 — too far at a hard `max_distance = 2` (the v1 default,
// which therefore did NOT actually fix its own headline test case),
// and not informative at `max_distance = 6` either, because at distance
// 6 a 4-char field name like `name` could spuriously rename to anything.
// The fix from `book-output/FEATURE_HARDENING_PLAN.md §2.3` is to use
// `distance / max(len_a, len_b) ≤ 0.25` as the primary gate, with an
// absolute ceiling as a sanity backstop.
//
// Conservative on purpose:
//   - Does nothing if the key is already in `allowed` (case-insensitive).
//   - Does nothing if two allowed keys tie at the same minimum distance
//     (ambiguous — better to fail loudly than corrupt the wrong field).
//   - Refuses to clobber an existing key (better to leave the typo'd
//     key in place than overwrite a legitimately-named field).

/// Damerau-Levenshtein edit distance between two strings (case-insensitive).
///
/// Uses the *optimal string alignment* variant — handles single
/// adjacent transpositions in addition to the four Levenshtein edits
/// (insert, delete, substitute, match). Adjacent transposition is the
/// dominant typo class for keyboard-typed identifiers; treating it as
/// 1 op (not 2) gives substantially better matches for typos like
/// `"vioce" → "voice"`, `"ohter" → "other"`, `"nmae" → "name"`.
///
/// FEATURE_HARDENING_PLAN.md §2.1 — upgraded from plain Levenshtein
/// because a higher distance cap (item 2.3 normalized-distance fix)
/// admits a wider candidate pool, where DL's transposition handling
/// removes a meaningful class of false-positive renames.
///
/// Three-row DP (current, previous, pre-previous) so the transposition
/// check `(i-2, j-2)` is reachable; still O(m*n) time, O(min(m,n)) space.
fn damerau_levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().flat_map(char::to_lowercase).collect();
    let b_chars: Vec<char> = b.chars().flat_map(char::to_lowercase).collect();
    let (m, n) = (a_chars.len(), b_chars.len());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut pre_prev: Vec<usize> = vec![0; n + 1];
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut cur: Vec<usize> = vec![0; n + 1];
    for i in 1..=m {
        cur[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            let mut v = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            // Transposition: a[i-1] == b[j-2] && a[i-2] == b[j-1].
            if i > 1
                && j > 1
                && a_chars[i - 1] == b_chars[j - 2]
                && a_chars[i - 2] == b_chars[j - 1]
            {
                v = v.min(pre_prev[j - 2] + 1);
            }
            cur[j] = v;
        }
        // Rotate: pre_prev <- prev, prev <- cur, cur (old pre_prev) is reused.
        std::mem::swap(&mut pre_prev, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[n]
}

/// Plain-Levenshtein alias — kept for tests that assert algorithmic
/// equivalence on non-transposition inputs. New code should call
/// [`damerau_levenshtein`] directly.
#[cfg(test)]
fn levenshtein(a: &str, b: &str) -> usize {
    damerau_levenshtein(a, b)
}

/// Field-name repair policy — combines an absolute distance ceiling
/// (sanity backstop) with a normalized-distance gate (the actual signal).
///
/// Default values are calibrated against the Run #11 failure modes:
///   - `max_absolute_distance = 6` — beyond 6 character edits the
///     strings are simply not the same identifier, regardless of length.
///   - `max_normalized_distance = 0.25` — admits 1-char typos in
///     4-char names (1/4 = 0.25), 4-char typos in 16-char names
///     (4/16 = 0.25), 4-char typos in 18-char names (4/18 = 0.22).
///     Hard rejects 5-char distance in 8-char names (5/8 = 0.63).
///
/// Both gates must pass for a candidate to qualify as a match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepairPolicy {
    pub max_absolute_distance: usize,
    pub max_normalized_distance: f32,
}

impl Default for RepairPolicy {
    fn default() -> Self {
        Self {
            max_absolute_distance: 6,
            max_normalized_distance: 0.25,
        }
    }
}

impl RepairPolicy {
    /// True iff `(actual, candidate)` qualify as a typo-pair under this
    /// policy. Both gates must pass.
    fn admits(&self, actual: &str, candidate: &str, distance: usize) -> bool {
        if distance > self.max_absolute_distance {
            return false;
        }
        let max_len = actual.chars().count().max(candidate.chars().count()).max(1);
        let normalized = distance as f32 / max_len as f32;
        normalized <= self.max_normalized_distance
    }
}

/// Find the unique nearest key in `allowed` to `actual` under `policy`,
/// or `None` if `actual` is already allowed (case-insensitive), no
/// candidate qualifies under the policy, or two candidates tie at the
/// same minimum distance (ambiguous).
fn nearest_key(actual: &str, allowed: &[&str], policy: &RepairPolicy) -> Option<String> {
    if allowed.iter().any(|a| a.eq_ignore_ascii_case(actual)) {
        return None;
    }
    let mut best: Option<(usize, &str)> = None;
    let mut tied = false;
    for cand in allowed {
        let d = damerau_levenshtein(actual, cand);
        if !policy.admits(actual, cand, d) {
            continue;
        }
        match best {
            None => {
                best = Some((d, cand));
                tied = false;
            }
            Some((bd, _)) => {
                if d < bd {
                    best = Some((d, cand));
                    tied = false;
                } else if d == bd {
                    tied = true;
                }
            }
        }
    }
    if tied {
        return None;
    }
    best.map(|(_, k)| (*k).to_owned())
}

/// Walk `value` recursively. For every object key not in `allowed`,
/// find the unique nearest allowed key under `policy` and rename it.
/// Renames are recorded in `audit.field_renames`.
///
/// Conservative: refuses to clobber an existing key, refuses to rename
/// when two allowed keys tie at the same distance.
pub fn repair_field_names(
    value: &mut Value,
    allowed: &[&str],
    policy: &RepairPolicy,
    audit: &mut RepairAudit,
) {
    match value {
        Value::Object(map) => {
            // Snapshot keys so we can mutate the map without invalidating an iterator.
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                let Some(corrected) = nearest_key(&k, allowed, policy) else {
                    continue;
                };
                if map.contains_key(&corrected) {
                    // Don't overwrite a legitimately-named field.
                    continue;
                }
                if let Some(v) = map.remove(&k) {
                    map.insert(corrected.clone(), v);
                    audit.field_renames.push((k, corrected));
                }
            }
            for v in map.values_mut() {
                repair_field_names(v, allowed, policy, audit);
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                repair_field_names(item, allowed, policy, audit);
            }
        }
        _ => {}
    }
}

/// Parse a JSON string, drop null list elements (the safe default),
/// and apply field-name self-healing using `allowed_keys` under the
/// default [`RepairPolicy`].
///
/// This is the recommended entry point for any agent whose schema has
/// distinguishable field names (most do — the bibles, scene cards, etc.).
/// Pass the union of every nested field name across the schema. False
/// positives are blocked by the conservative tie-breaking in
/// [`nearest_key`] and the dual-gate policy in [`RepairPolicy`].
///
/// For an agent that needs a stricter or laxer policy, use the lower-level
/// [`parse_and_repair_with_policy`].
pub fn parse_and_repair_with_schema_keys(
    raw: &str,
    allowed_keys: &[&str],
) -> Result<(Value, RepairAudit), String> {
    parse_and_repair_with_policy(raw, allowed_keys, &RepairPolicy::default())
}

/// Like [`parse_and_repair_with_schema_keys`] but with an explicit
/// policy. Useful for agents whose schemas have unusually short field
/// names or particularly noisy model outputs.
pub fn parse_and_repair_with_policy(
    raw: &str,
    allowed_keys: &[&str],
    policy: &RepairPolicy,
) -> Result<(Value, RepairAudit), String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;
    let (mut v2, mut audit) = repair_value(v, &KEEP_NON_NULL);
    repair_field_names(&mut v2, allowed_keys, policy, &mut audit);
    Ok((v2, audit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keeps_well_formed_list_of_objects() {
        let raw = json!({
            "characters": [
                {"name": "Ada"},
                {"name": "Bryn"},
            ],
        });
        let (v, audit) = repair_value(raw.clone(), &DEFAULT_LIST_OF_OBJECTS_KEEP);
        assert_eq!(v, raw);
        assert_eq!(audit.dropped_list_elements, 0);
        assert_eq!(audit.lists_examined, 1);
    }

    #[test]
    fn drops_string_placeholder_in_object_list() {
        // The exact failure mode from BF-E2E-LOCAL-LLM-FIRST-BOOK-001 Phase 5.
        let raw = json!({
            "characters": [
                {"name": "Ada"},
                "characters_2",
                {"name": "Bryn"},
            ],
        });
        let (v, audit) = repair_value(raw, &DEFAULT_LIST_OF_OBJECTS_KEEP);
        assert_eq!(audit.dropped_list_elements, 1);
        assert_eq!(v["characters"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn walks_nested_lists() {
        let raw = json!({
            "outer": {
                "inner": [
                    {"keep": 1, "scenes": [{"goal": "x"}, "scene_2_placeholder", {"goal": "y"}]},
                    "drop_me",
                ],
            },
        });
        let (v, audit) = repair_value(raw, &DEFAULT_LIST_OF_OBJECTS_KEEP);
        assert_eq!(audit.dropped_list_elements, 2);
        assert_eq!(v["outer"]["inner"].as_array().unwrap().len(), 1);
        assert_eq!(
            v["outer"]["inner"][0]["scenes"].as_array().unwrap().len(),
            2,
        );
    }

    #[test]
    fn keep_non_null_predicate_drops_only_nulls() {
        let raw = json!({
            "keywords": ["fantasy", null, "courage", null],
        });
        let (v, audit) = repair_value(raw, &KEEP_NON_NULL);
        assert_eq!(audit.dropped_list_elements, 2);
        assert_eq!(
            v["keywords"].as_array().unwrap(),
            &vec![json!("fantasy"), json!("courage")],
        );
    }

    #[test]
    fn parse_and_repair_handles_invalid_json() {
        let res = parse_and_repair("not json");
        assert!(res.is_err());
    }

    #[test]
    fn parse_and_repair_drops_only_nulls_by_default() {
        // Permissive default — strings are kept. Use parse_and_repair_strict_objects
        // when the schema actually requires object-only lists at known keys.
        let raw = r#"{"characters":[{"n":1},"oops",{"n":2},null]}"#;
        let (v, audit) = parse_and_repair(raw).unwrap();
        assert_eq!(audit.dropped_list_elements, 1, "should drop the null only");
        assert_eq!(v["characters"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn parse_and_repair_strict_drops_strings_at_named_keys() {
        // The BF-E2E Phase 5 case: characters is declared list-of-objects,
        // and a string placeholder slipped in.
        let raw = r#"{"characters":[{"n":1},"characters_2",{"n":2}],"keywords":["a","b",null]}"#;
        let (v, audit) = parse_and_repair_strict_objects(raw, &["characters"]).unwrap();
        // 1 string dropped from characters + 1 null dropped from keywords.
        assert_eq!(audit.dropped_list_elements, 2);
        assert_eq!(v["characters"].as_array().unwrap().len(), 2);
        assert_eq!(
            v["keywords"].as_array().unwrap(),
            &vec![json!("a"), json!("b")],
            "strings in keywords (not in object_list_keys) must be kept",
        );
    }

    // ── Schema-aware field-name self-healing ────────────────────────────

    /// Strict policy for the older "edit distance ≤ 2 absolute" tests
    /// — kept around because some assertions specifically test the
    /// rejection-of-too-far-typo behaviour.
    fn strict_2() -> RepairPolicy {
        RepairPolicy {
            max_absolute_distance: 2,
            max_normalized_distance: 1.0, // disable the normalized gate
        }
    }

    #[test]
    fn levenshtein_basic_correctness() {
        // Damerau-Levenshtein agrees with plain Levenshtein on
        // non-transposition inputs.
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("kitten", "sitting"), 3); // textbook example
        assert_eq!(levenshtein("Foo", "foo"), 0); // case-insensitive
    }

    #[test]
    fn damerau_levenshtein_handles_transpositions_as_one_edit() {
        // Plain Levenshtein scores each of these as 2 edits (sub + sub).
        // Damerau-Levenshtein scores them as 1 (single transposition).
        assert_eq!(damerau_levenshtein("vioce", "voice"), 1);
        assert_eq!(damerau_levenshtein("ohter", "other"), 1);
        assert_eq!(damerau_levenshtein("nmae", "name"), 1);
        // Multi-transposition prose still costs more than 1.
        assert_eq!(damerau_levenshtein("vioec", "voice"), 2);
        // Match base cases.
        assert_eq!(damerau_levenshtein("", ""), 0);
        assert_eq!(damerau_levenshtein("name", "name"), 0);
    }

    #[test]
    fn damerau_levenshtein_transposition_lets_short_field_typo_match() {
        // Pre-DL: "nmae" → "name" was distance 2, normalized 2/4 = 0.50,
        // rejected by default policy. With DL, distance is 1 (0.25),
        // which exactly meets the cap and is admitted.
        let allowed = ["name", "external_objective"];
        let pol = RepairPolicy::default();
        assert_eq!(nearest_key("nmae", &allowed, &pol), Some("name".to_owned()),);
    }

    #[test]
    fn repair_policy_default_admits_run11_typo() {
        // The headline FEATURE_HARDENING_PLAN.md §2.3 claim: the default
        // policy must admit `external_object` → `external_objective`
        // (distance 4, normalized 4/18 = 0.222) without anyone having
        // to override the policy.
        let pol = RepairPolicy::default();
        assert!(pol.admits("external_object", "external_objective", 4));
        // ...but must NOT admit a 5-char typo in an 8-char name (5/8 = 0.625).
        assert!(!pol.admits("foo_obje", "external", 5));
        // ...nor a 1-char typo in a 1-char string (1/1 = 1.0).
        assert!(!pol.admits("a", "b", 1));
    }

    #[test]
    fn repair_policy_absolute_ceiling_blocks_long_typos_in_long_names() {
        // A 7-char distance in 28-char names normalizes to 0.25 (passes
        // the normalized gate) but absolute 7 exceeds the default
        // ceiling of 6. Both gates must pass — so this is rejected.
        let pol = RepairPolicy::default();
        assert!(!pol.admits(
            "the_long_field_namee_typo_x", // 27 chars
            "the_long_field_name_correct", // 27 chars
            7,
        ));
    }

    #[test]
    fn nearest_key_returns_none_for_already_allowed() {
        let allowed = ["external_objective", "internal_wound"];
        let pol = RepairPolicy::default();
        assert_eq!(nearest_key("external_objective", &allowed, &pol), None);
        // Case-insensitive equality counts as already-allowed.
        assert_eq!(nearest_key("EXTERNAL_OBJECTIVE", &allowed, &pol), None);
    }

    #[test]
    fn nearest_key_corrects_run11_typo_at_default_policy() {
        // The Run #11 card #2 failure — distance 4, normalized 4/18 ≈ 0.22.
        // MUST work at default policy. v1 (max_distance=2) failed this.
        let allowed = ["external_objective", "internal_wound", "voice_traits"];
        assert_eq!(
            nearest_key("external_object", &allowed, &RepairPolicy::default()),
            Some("external_objective".to_owned()),
        );
        // The strict-2 policy still rejects (preserved for back-compat tests).
        assert_eq!(nearest_key("external_object", &allowed, &strict_2()), None,);
    }

    #[test]
    fn nearest_key_corrects_single_letter_typo() {
        let allowed = ["voice_traits", "external_objective"];
        let pol = RepairPolicy::default();
        assert_eq!(
            nearest_key("voice_trait", &allowed, &pol),
            Some("voice_traits".to_owned()),
        );
        assert_eq!(
            nearest_key("voce_traits", &allowed, &pol),
            Some("voice_traits".to_owned()),
        );
    }

    #[test]
    fn nearest_key_refuses_ambiguous_ties() {
        // "stop" is distance 1 from both "step" and "shop" — ambiguous.
        let allowed = ["step", "shop"];
        let pol = RepairPolicy::default();
        assert_eq!(nearest_key("stop", &allowed, &pol), None);
    }

    #[test]
    fn nearest_key_refuses_distant_match() {
        let allowed = ["external_objective"];
        let pol = RepairPolicy::default();
        // "external_target" shares prefix but distance is high enough
        // that neither gate admits it.
        assert_eq!(nearest_key("external_target", &allowed, &pol), None);
    }

    #[test]
    fn nearest_key_normalized_gate_protects_short_names() {
        // 1-char typo in `name` (4 chars) is normalized 0.25 — at the cap.
        // This MUST pass.
        let allowed = ["name"];
        let pol = RepairPolicy::default();
        assert_eq!(nearest_key("nme", &allowed, &pol), Some("name".to_owned()),);
        // 2-char typo in `name` is normalized 0.50 — over the cap.
        // Must reject (otherwise short fields become a magnet for any
        // similarly-shaped key).
        assert_eq!(nearest_key("xy", &allowed, &pol), None);
    }

    #[test]
    fn repair_field_names_renames_top_level_typo() {
        let mut v = json!({
            "name": "Elara",
            "voce_traits": ["short", "punchy"],
        });
        let mut audit = RepairAudit::default();
        repair_field_names(
            &mut v,
            &["name", "voice_traits"],
            &RepairPolicy::default(),
            &mut audit,
        );
        assert!(v.get("voice_traits").is_some());
        assert!(v.get("voce_traits").is_none());
        assert_eq!(
            audit.field_renames,
            vec![("voce_traits".to_owned(), "voice_traits".to_owned())]
        );
    }

    #[test]
    fn repair_field_names_walks_nested_objects() {
        let mut v = json!({
            "character": {
                "name": "Elara",
                "voce_traits": ["short"],          // distance 1 → voice_traits
            },
            "world": {
                "geograhpy": "fjords",             // distance 2 → geography
                "regions": ["north", "south"],
            },
        });
        let mut audit = RepairAudit::default();
        repair_field_names(
            &mut v,
            &[
                "character",
                "name",
                "voice_traits",
                "world",
                "geography",
                "regions",
            ],
            &RepairPolicy::default(),
            &mut audit,
        );
        assert!(
            v["character"]["voice_traits"].is_array(),
            "nested rename should reach 2nd level"
        );
        assert!(
            v["world"]["geography"].is_string(),
            "nested rename should reach 2nd level"
        );
        assert_eq!(audit.field_renames.len(), 2);
    }

    #[test]
    fn repair_field_names_refuses_to_clobber() {
        // The model emitted both the typo'd key AND the real key.
        // We must NOT silently drop a field by overwriting.
        let mut v = json!({
            "voice_traits": ["real"],
            "voce_traits": ["typo"],
        });
        let mut audit = RepairAudit::default();
        repair_field_names(
            &mut v,
            &["voice_traits"],
            &RepairPolicy::default(),
            &mut audit,
        );
        assert_eq!(v["voice_traits"].as_array().unwrap().len(), 1);
        assert_eq!(v["voice_traits"][0], json!("real"));
        // Typo'd key stays, audit records no rename.
        assert!(v.get("voce_traits").is_some());
        assert!(audit.field_renames.is_empty());
    }

    #[test]
    fn parse_and_repair_with_schema_keys_end_to_end_now_fixes_external_object() {
        // Reconstructs the Run #11 card #2 failure end-to-end. After
        // FEATURE_HARDENING_PLAN.md §2.3, the default policy MUST fix
        // the long-suffix typo too — not just the 1-char `voce_traits`.
        let raw = r#"{
          "name": "Elara",
          "external_object": "Find Arthur's letter.",
          "voce_traits": ["dry", null, "interior"],
          "internal_wound": "Old grief, never spoken."
        }"#;
        let allowed = [
            "name",
            "external_objective",
            "voice_traits",
            "internal_wound",
            "relationships",
        ];
        let (v, audit) = parse_and_repair_with_schema_keys(raw, &allowed).unwrap();
        // BOTH typos are now fixed at default policy.
        assert!(
            v.get("voice_traits").is_some(),
            "voce_traits → voice_traits"
        );
        assert!(
            v.get("external_objective").is_some(),
            "external_object → external_objective (Run #11 headline fix)",
        );
        assert!(v.get("external_object").is_none(), "old typo'd key removed");
        assert_eq!(
            audit.dropped_list_elements, 1,
            "null in voice_traits dropped",
        );
        let rename_keys: std::collections::HashSet<&str> = audit
            .field_renames
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert!(rename_keys.contains("voce_traits"));
        assert!(rename_keys.contains("external_object"));
    }

    #[test]
    fn parse_and_repair_with_policy_allows_strict_override() {
        // An agent that explicitly wants the v1 absolute-only behaviour
        // can still get it via the policy.
        let raw = r#"{"external_object": "x", "name": "Elara"}"#;
        let (v, _audit) =
            parse_and_repair_with_policy(raw, &["external_objective", "name"], &strict_2())
                .unwrap();
        // Strict policy rejects the 4-char typo.
        assert!(v.get("external_object").is_some());
        assert!(v.get("external_objective").is_none());
    }
}
