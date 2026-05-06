# `.claude/` — BooksForge

This folder is the Claude Code home for BooksForge.

- `skills/<id>/SKILL.md` — invokable skill definitions. See `outputs/CLAUDE_CODE_SKILLS_SPEC.md` for the full spec of each.
- `hooks/HOOKS.md` — declarative hook list Claude Code respects at edit time. See `outputs/CLAUDE_CODE_HOOKS_SPEC.md`.
- `agents/<id>.md` — review-only subagent stubs. See `outputs/CLAUDE_CODE_SUBAGENTS_SPEC.md`.

The stubs are intentionally short. Each one references the authoritative spec file by section. Procedures, inputs, outputs, and acceptance criteria live in the spec — the stub is the runnable form.

When you add or change a skill, hook, or subagent, update both the spec and the stub in the same PR.
