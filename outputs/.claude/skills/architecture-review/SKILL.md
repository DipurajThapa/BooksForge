# Skill: architecture-review

> **Spec.** See `outputs/CLAUDE_CODE_SKILLS_SPEC.md` — the section matching this skill id.

## When to invoke

See the spec's "When Claude Code should use it" section for this skill.

## Procedure

Follow the procedure in the spec. Do not invent steps.

## Inputs

Read only the files listed in "Files it may read" in the spec. Avoid loading large deep specs unless they are explicitly listed.

## Output

Produce the format specified in the spec. Keep reports under 300 words unless detail is required.

## Failure mode

If the procedure cannot complete (missing files, ambiguous case), surface the issue and pause for human review. Do not guess.

## Token budget

Stay under 8K tokens of context. If you need more, you are reading too much — stop and consult `CLAUDE_CODE_CONTEXT_HARNESS.md`.
