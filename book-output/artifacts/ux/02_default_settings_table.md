# 02 — Default Settings Coverage

**Score:** 4.7/10 · **Status:** FAIL · **Weight:** 1.5

9/19 major workflow categories have defaults that the user can rely on. Big gaps: structure model, editorial strictness, originality level, KDP trim/bleed/interior, marketplace targets, cover brief style — each is either not exposed or has no sensible default.

## Per-category status

- **book_type**: project template → ✓
- **genre**: default 'fantasy' → ✓
- **audience**: default 'adult' → ✓
- **tone**: default 'adventurous' → ✓
- **length**: default 80,000 words → ✓
- **chapter_count**: default 12 → ✓
- **pov**: no default in wizard; user types in chapter-drafter form → ✗
- **structure_model**: no explicit structure-model picker → ✗
- **editorial_strictness**: not exposed → ✗
- **originality_check**: not exposed → ✗
- **keyword_optimization**: not exposed → ✗
- **trim_size**: ExportPanel exposes trim → ✓
- **interior_type**: not exposed in wizard → ✗
- **bleed**: not exposed → ✗
- **ebook_format**: ExportPanel offers epub → ✓
- **marketplace_targets**: no explicit picker → ✗
- **metadata_generation**: agent-driven, not in wizard → ✗
- **cover_brief_style**: not exposed → ✗
- **preview_mode**: ValidatorPanel + previews → ✓

## Recommended defaults to add (ordered by impact)

1. **Trim size** → default `6×9 in` (KDP trade paperback) when "publish to KDP" is selected.
2. **Bleed** → default `0.125 in` for cover, `none` for interior.
3. **Interior paper type** → default `cream uncoated` for fiction, `white uncoated` for non-fiction.
4. **Cover finish** → default `matte`.
5. **Editorial strictness** → default `medium` (3 of 5).
6. **Originality check level** → default `enabled, low-keyword-density`.
7. **Marketplace targets** → default `KDP + Google Play + Apple Books` checkboxes pre-selected.
8. **Cover brief style** → default genre-derived (cozy fantasy → "warm, illustrated, character-forward").
9. **Structure model** → default `three-act` for novels, `chapter-thesis` for non-fiction.
10. **POV** → default `third-limited` for fiction, `first` for memoir, `none` for non-fiction.

Each of these defaults should be **dynamically chosen** based on the brief's
`book_type` field (currently inferred only via the AI toggle).
