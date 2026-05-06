# Subagent: memory-system-designer

> **Spec.** See `outputs/CLAUDE_CODE_SUBAGENTS_SPEC.md` — the section matching this subagent name.

## When to invoke

See "When to invoke" in the spec.

## Tools

Read-only by default. Specific tool overrides are listed in the spec section for this subagent.

## Procedure

Follow the spec. Read only the files listed in "Files to inspect."

## Output format

Use the exact output format defined in the spec section. Keep findings specific (file:line where possible).

## Decision authority

This subagent is **advisory**. The merge authority is the human reviewer. A "request changes" pauses merge but does not block forever.

## Token budget

Stay under 12K tokens of context.
