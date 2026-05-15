//! Scene Drafter (Fiction) Agent — fiction-shaped sibling of `chapter_drafter`.
//!
//! BACKLOG §A13 / Phase 1 of `PRODUCT_ROADMAP_E2E.md`. Drafts a fiction scene
//! from a scene card (goal / conflict / reveal / target_words / pov), with
//! the character bible + world bible loaded into context, and voice
//! constraints (numeric profile from `booksforge-voice`, when wired in
//! Phase 3) injected as a constraint block.
//!
//! Output is the existing `SceneDraftProposal` (pm_doc + word_count +
//! notes), so the orchestrator's existing `apply_chapter_drafter` flow
//! handles the apply path with one trivial alias.
//!
//! ### Why a separate agent (vs. extending `chapter_drafter`)?
//!
//! - The non-fiction drafter (`chapter-drafter` / `chapter-drafter-nf`) is
//!   shaped around `scene_synopsis + chapter_purpose` — the wrong
//!   abstraction for fiction where scene goal / conflict / reveal are the
//!   load-bearing inputs.
//! - The fiction prompt template carries character-bible + world-bible
//!   slots the non-fiction prompt does not.
//! - Routing in the orchestrator is by `BookKind` (Phase 4) — keeping the
//!   two agents distinct lets the workflow router pick cleanly without
//!   prompt-template branching logic at run time.

use booksforge_domain::SceneDraftProposal;
use booksforge_prompt::PromptTemplateId;

use crate::spec::{
    AgentSpec, ContextBudget, CrossCuttingValidator, DefaultThinking, FailureMode, ModelFamily,
    ModelPreference, ModelSizeHint, UserGate, WhenToRun,
};

const FAILURE_MODES: &[FailureMode] = &[
    FailureMode {
        id: "fact-invention",
        description: "Draft introduces characters or places not in the bibles.",
        recoverable: true,
    },
    FailureMode {
        id: "voice-mismatch",
        description:
            "Draft cadence / sentence-length distribution misses the voice constraint targets.",
        recoverable: true,
    },
    FailureMode {
        id: "wrong-pov",
        description: "Draft switches POV away from the chapter's declared POV character.",
        recoverable: true,
    },
    FailureMode {
        id: "scene-goal-not-advanced",
        description: "Scene ends without advancing the declared scene goal or landing the reveal.",
        recoverable: true,
    },
    FailureMode {
        id: "word-count-undershoot",
        description: "Draft is < 50% of the scene's target_words.",
        recoverable: true,
    },
    FailureMode {
        id: "continuity-violation",
        description: "Draft contradicts a `continuity_constraint` in the world bible.",
        recoverable: true,
    },
];

pub fn spec() -> AgentSpec {
    AgentSpec {
        id:               "scene-drafter-fic",
        name:             "Scene Drafter (Fiction)",
        purpose:          "Draft a fiction scene from a scene card (goal / conflict / reveal / target_words / pov), with character bible + world bible loaded into context and voice constraints applied. Replaces the prior naked-LLM scene-draft prompt for fiction projects.",
        input_schema_id:  "SceneCardInput",
        output_schema_id: "SceneDraftProposal",
        prompt_template:  PromptTemplateId::new("scene-drafter-fic", "v1"),
        model_preference: ModelPreference {
            // Fiction prose at the sentence level needs the larger model.
            // Per RCA §L2.1 — routing fiction drafting to 27B is the single
            // largest pipeline-config quality lift.
            family:   ModelFamily::LongContext,
            min_size: ModelSizeHint::Large,
        },
        pinned_model: None,
        context_budget: ContextBudget {
            // PERMANENT HEADROOM (Run #13 → Run #15 fix). The
            // Run #11 → Run #13 arc revealed every voice-contract
            // tightening tightens the drafter's thinking-mode budget
            // with it. Run #11: drafter returned empty at num_predict
            // 6_000. Run #13: 28.2 min at 16_000 because MANDATORY
            // INTERLEAVING ate thinking cycles.
            //
            // qwen3.6:latest reports `qwen35moe.context_length:
            // 262144` (256k native). On the target Apple Silicon
            // unified-memory profile (51.8 GiB total, q8_0 KV cache,
            // 27 GiB model weights):
            //   -  64k → ~34 GiB total → comfortable
            //   - 128k → ~42 GiB total → comfortable
            //   - 192k → ~51 GiB total → tight, peak spills
            //   - 256k → ~60 GiB total → spills to system RAM,
            //     3-5× slowdown (NOT viable on this hardware)
            //
            // 128k is the safe sweet spot. Today's actual drafter
            // usage is ~10-11k input + ~32k output ≈ 42k. The
            // additional headroom is for future work:
            //   - Adaptive planner (item 4) feeding prior-scene
            //     corpus into context.
            //   - Exemplar memory (item 5) loading top-quality
            //     paragraph examples from earlier runs.
            //   - Multi-chapter consistency loading 2-3 prior
            //     chapters at once.
            //
            // Allocation:
            //   - 64k input — 6× today's actual usage. Fits
            //     long bibles + full prior-chapter context +
            //     exemplar examples + voice contract.
            //   - 32k input — fits long bibles + voice contract +
            //     exemplar block + scene card + prior summary
            //     comfortably (today's actual usage is ~10-12k).
            //   - 32k output — 2× Run #11/#12 budget. Lets
            //     thinking mode consume ~12-16k of reasoning
            //     before emitting JSON while still leaving room
            //     for ~1500-word scenes.
            //   - Total 64k — proven in Run #14 to land in 4.4
            //     min on the user's hardware. The 128k variant
            //     for adaptive-planner / exemplar-memory headroom
            //     hit the orchestrator's 30-min wall-clock cap on
            //     the first drafter call due to model reload +
            //     larger KV-cache init. Reinstate when downstream
            //     consumers actually feed enough context to need it.
            max_context_tokens: 32_000,
            max_output_tokens:  32_000,
        },
        validators: &[
            CrossCuttingValidator::Schema,
            CrossCuttingValidator::Redaction,
            CrossCuttingValidator::Length,
            CrossCuttingValidator::EntitySanity,
            CrossCuttingValidator::Originality,
        ],
        failure_modes: FAILURE_MODES,
        when_to_run:   WhenToRun::OnDemand,
        user_gate:     UserGate::Required,
        // Fiction drafting benefitted from thinking-mode in Run #14 with
        // a lighter directive, but after the Run #15 retighten added
        // MANDATORY INTERLEAVING + the storytelling craft section, the
        // model started spending its entire num_predict budget on
        // <think> tokens and emitting empty pm_doc.content arrays
        // (4 attempts × ~6 min each = 0 words). The directive set is
        // now prescriptive enough that explicit reasoning isn't earning
        // its budget cost. Disabling thinking-mode here gives the prose
        // the full 32k output budget to land in. Re-enable when the
        // adaptive planner exists and routes thinking-budget to where
        // it actually pays off.
        default_thinking: DefaultThinking::Disabled,
    }
}

/// Parse the model's raw output into a `SceneDraftProposal`.
///
/// **Bare-prose recovery** (Run #16 reliability fix). The drafter
/// has historically failed in two opposite ways:
///   - Run #13: model emits prose with no JSON wrapper at all.
///   - Run #15+: model emits empty / truncated output under
///     `format: "json"` decoder constraints.
///
/// Both failures lose 4-28 minutes of generation. The recovery
/// path: try strict JSON parse first; if that fails AND the
/// content is non-empty prose, synthesise a `SceneDraftProposal`
/// with the bare prose as a single paragraph. The drafter call is
/// no longer fragile to whether `format: "json"` succeeded — we
/// take whatever the model produced and salvage it.
pub fn parse_and_validate(raw: &str) -> Result<SceneDraftProposal, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty model output (no JSON, no prose)".to_owned());
    }

    // Strip common markdown code-fence wrappers some models add.
    let stripped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);
    let stripped = stripped
        .strip_suffix("```")
        .map(str::trim)
        .unwrap_or(stripped);

    // Path A — strict JSON. The expected happy path.
    let strict_attempt =
        crate::json_repair::parse_and_repair(stripped).and_then(|(repaired, audit)| {
            if audit.dropped_list_elements > 0 {
                tracing::warn!(
                    agent = "scene-drafter-fic",
                    dropped = audit.dropped_list_elements,
                    "json_repair salvaged malformed list elements before deserialise",
                );
            }
            serde_json::from_value::<SceneDraftProposal>(repaired)
                .map_err(|e| format!("JSON parse error after repair: {e}"))
                .and_then(|p| {
                    let errs = p.validate();
                    if errs.is_empty() {
                        Ok(p)
                    } else {
                        Err(format!("semantic validation failed: {}", errs.join("; ")))
                    }
                })
        });

    if let Ok(proposal) = strict_attempt {
        return Ok(proposal);
    }

    // Path B — bare-prose recovery. Strict JSON failed; if the
    // content looks like prose (has letters, doesn't start with `{`),
    // wrap it as a synthetic single-paragraph SceneDraftProposal so
    // the orchestrator can use the prose the model DID produce.
    if !stripped.starts_with('{') && !stripped.starts_with('[') {
        let word_count = stripped.split_whitespace().count() as u32;
        if word_count >= 30 {
            // Build paragraphs from the bare prose by splitting on \n\n.
            let paragraphs: Vec<serde_json::Value> = stripped
                .split("\n\n")
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(|p| {
                    serde_json::json!({
                        "type": "paragraph",
                        "content": [{ "type": "text", "text": p }],
                    })
                })
                .collect();
            if !paragraphs.is_empty() {
                tracing::warn!(
                    agent = "scene-drafter-fic",
                    word_count,
                    "bare-prose recovery: model emitted prose without JSON wrapper; \
                     synthesising SceneDraftProposal so the {word_count} words are not lost",
                );
                let synth = SceneDraftProposal {
                    pm_doc: serde_json::json!({
                        "type": "doc",
                        "content": paragraphs,
                    }),
                    word_count,
                    notes: "(synthesised from bare-prose response — no JSON wrapper emitted)"
                        .to_owned(),
                };
                return Ok(synth);
            }
        }
    }

    // Path C — truncated-JSON salvage. The model started a
    // SceneDraftProposal JSON but ran out of `num_predict` budget
    // mid-string at line N. Path A failed because the JSON is
    // syntactically invalid; Path B was skipped because the output
    // starts with `{`. Salvage the prose by scanning for every
    // `"text": "..."` pair we can find before the truncation point.
    // Each one becomes a paragraph node in a synthetic pm_doc.
    if stripped.starts_with('{') {
        let texts = extract_text_values(stripped);
        let total_words: u32 = texts
            .iter()
            .map(|t| t.split_whitespace().count() as u32)
            .sum();
        if total_words >= 30 && !texts.is_empty() {
            let paragraphs: Vec<serde_json::Value> = texts
                .iter()
                .filter(|t| !t.trim().is_empty())
                .map(|t| {
                    serde_json::json!({
                        "type": "paragraph",
                        "content": [{ "type": "text", "text": t }],
                    })
                })
                .collect();
            if !paragraphs.is_empty() {
                tracing::warn!(
                    agent = "scene-drafter-fic",
                    word_count = total_words,
                    paragraphs = paragraphs.len(),
                    "truncated-JSON salvage: extracted {} paragraph(s) from a JSON-shaped \
                     but unparseable response (output likely hit num_predict mid-string)",
                    paragraphs.len(),
                );
                let synth = SceneDraftProposal {
                    pm_doc: serde_json::json!({
                        "type": "doc",
                        "content": paragraphs,
                    }),
                    word_count: total_words,
                    notes: "(salvaged from truncated JSON — pm_doc rebuilt from `text` values \
                            scanned before truncation point)"
                        .to_owned(),
                };
                return Ok(synth);
            }
        }
    }

    // Neither path worked — return the strict error for the runner's retry log.
    strict_attempt
}

/// Extract every JSON string value associated with a `"text"` key from
/// the input. Tolerates truncation: stops cleanly at end-of-input or at
/// an unterminated string. Used by the truncated-JSON salvage path.
fn extract_text_values(raw: &str) -> Vec<String> {
    let bytes = raw.as_bytes();
    let mut texts = Vec::new();
    let needle = b"\"text\"";
    let mut i = 0;
    while i + needle.len() < bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            // Skip past the `"text"` token + any whitespace + the `:`
            // + any whitespace + the opening quote of the value.
            let mut j = i + needle.len();
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b':' {
                j += 1;
                while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'"' {
                    j += 1;
                    let mut s = String::new();
                    let mut escape = false;
                    while j < bytes.len() {
                        let c = bytes[j] as char;
                        if escape {
                            // Decode common JSON escapes; otherwise pass through.
                            match c {
                                'n' => s.push('\n'),
                                't' => s.push('\t'),
                                'r' => s.push('\r'),
                                '"' => s.push('"'),
                                '\\' => s.push('\\'),
                                '/' => s.push('/'),
                                _ => s.push(c),
                            }
                            escape = false;
                            j += 1;
                            continue;
                        }
                        if c == '\\' {
                            escape = true;
                            j += 1;
                            continue;
                        }
                        if c == '"' {
                            // Closed string — record it.
                            j += 1;
                            break;
                        }
                        s.push(c);
                        j += 1;
                    }
                    // If the string was truncated (no closing quote
                    // found before end-of-input), still keep it.
                    if !s.is_empty() {
                        texts.push(s);
                    }
                    i = j;
                    continue;
                }
            }
        }
        i += 1;
    }
    texts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let s = spec();
        assert_eq!(s.id, "scene-drafter-fic");
        assert_eq!(s.input_schema_id, "SceneCardInput");
        assert_eq!(s.output_schema_id, "SceneDraftProposal");
        assert!(matches!(s.user_gate, UserGate::Required));
        assert!(matches!(s.default_thinking, DefaultThinking::Disabled));
        // Fiction drafting needs the bigger model + bigger context.
        assert!(matches!(s.model_preference.min_size, ModelSizeHint::Large));
        // Run #13 permanent headroom: 32k input + 32k output = 64k total
        // (qwen3.6 native max is 256k).
        // Permanent headroom — 128k total, safe under the user's
        // 51.8 GiB unified-memory ceiling at q8_0 KV cache. Native
        // model max is 256k but that exceeds VRAM on this hardware.
        // Run #14-validated: 32k+32k = 64k total. The 128k variant
        // is reinstatable once downstream consumers (planner with prior-
        // scene context, exemplar-memory loading) actually fill it.
        assert_eq!(s.context_budget.max_context_tokens, 32_000);
        assert_eq!(s.context_budget.max_output_tokens, 32_000);
        assert_eq!(s.context_budget.total(), 64_000);
        assert_eq!(s.failure_modes.len(), 6);
    }

    #[test]
    fn parse_accepts_well_formed_proposal() {
        let raw = r#"{
          "pm_doc": {
            "type": "doc",
            "content": [
              {"type": "paragraph", "content": [{"type": "text", "text": "Ada walked in. The light was off."}]},
              {"type": "paragraph", "content": [{"type": "text", "text": "She stood for a long moment in the dark."}]}
            ]
          },
          "word_count": 16,
          "notes": "Opening in-medias-res; deliberate short sentences match the comp voice profile."
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_ok(), "expected ok, got {res:?}");
        let p = res.unwrap();
        assert_eq!(p.word_count, 16);
        assert!(!p.notes.is_empty());
    }

    #[test]
    fn parse_rejects_empty_pm_doc() {
        let raw = r#"{
          "pm_doc": {"type": "doc", "content": []},
          "word_count": 0,
          "notes": "x"
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("pm_doc.content is empty"), "got: {err}");
    }

    #[test]
    fn parse_rejects_wrong_doc_type() {
        let raw = r#"{
          "pm_doc": {"type": "wrong", "content": [{"type":"paragraph","content":[{"type":"text","text":"x"}]}]},
          "word_count": 1,
          "notes": "x"
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("type='doc'"), "got: {err}");
    }

    #[test]
    fn parse_repairs_null_in_pm_doc_content() {
        // Realistic null-in-content failure mode — the default json_repair
        // (null-only) salvages it.
        let raw = r#"{
          "pm_doc": {
            "type": "doc",
            "content": [
              {"type":"paragraph","content":[{"type":"text","text":"First."}]},
              null,
              {"type":"paragraph","content":[{"type":"text","text":"Second."}]}
            ]
          },
          "word_count": 2,
          "notes": "x"
        }"#;
        let res = parse_and_validate(raw);
        assert!(res.is_ok(), "json_repair should salvage; got {res:?}");
        let p = res.unwrap();
        assert_eq!(p.pm_doc["content"].as_array().unwrap().len(), 2);
    }

    // ── Run #16 reliability fix: bare-prose recovery ─────────────────────

    #[test]
    fn bare_prose_recovery_synthesises_proposal() {
        // Run #13 failure mode: model emits prose with no JSON wrapper.
        // The recovery path wraps it as a synthetic SceneDraftProposal
        // so the 28+ minutes of generation aren't lost.
        let raw = "The iron key scraped against the tumblers. Elara held her breath. \
                   The lock gave with a heavy click. She pushed the drawer open. \
                   Dust motes drifted in the slanted afternoon light. The smell of \
                   machine oil and old paper filled her nose.";
        let res = parse_and_validate(raw);
        assert!(res.is_ok(), "bare prose should be recovered; got {res:?}");
        let p = res.unwrap();
        assert_eq!(p.pm_doc["type"], "doc");
        assert!(!p.pm_doc["content"].as_array().unwrap().is_empty());
        assert!(p.word_count >= 30);
        assert!(p.notes.contains("bare-prose"));
    }

    #[test]
    fn bare_prose_recovery_handles_multi_paragraph() {
        // Multi-paragraph bare prose splits cleanly on \n\n.
        let raw = "First paragraph with enough words to clear the thirty word threshold for valid recovery and produce something the orchestrator can usefully consume.\n\nSecond paragraph also has enough words to be a real complete unit of prose that lands as its own paragraph node.";
        let res = parse_and_validate(raw);
        assert!(res.is_ok(), "got {res:?}");
        let p = res.unwrap();
        assert_eq!(p.pm_doc["content"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn bare_prose_too_short_is_rejected() {
        // Short bare-prose isn't worth synthesising — it's likely
        // an error message or partial output.
        let raw = "Just a few words.";
        let res = parse_and_validate(raw);
        assert!(res.is_err());
    }

    #[test]
    fn empty_output_is_rejected_cleanly() {
        let res = parse_and_validate("");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("empty"));
    }

    #[test]
    fn markdown_fenced_json_still_parses() {
        // Some models wrap JSON in ```json … ``` fences. The parser
        // strips the fence before strict-JSON parse.
        let raw = "```json\n{\"pm_doc\":{\"type\":\"doc\",\"content\":[{\"type\":\"paragraph\",\"content\":[{\"type\":\"text\",\"text\":\"Hello world from a fenced response.\"}]}]},\"word_count\":6,\"notes\":\"x\"}\n```";
        let res = parse_and_validate(raw);
        assert!(res.is_ok(), "fenced JSON should parse; got {res:?}");
    }
}
