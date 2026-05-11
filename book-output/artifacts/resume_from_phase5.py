#!/usr/bin/env python3
"""
Resume the BF-E2E run from Phase 5. Loads ideation/research/topic/plan from
disk (already produced by the original run) and continues through Phase 16.

Phase 5 is re-run with the defensive filter; the existing 05_/06_ files get
overwritten with the corrected versions.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from run_e2e import (  # type: ignore
    ART, log, record_phase,
    phase5_bibles, phase6_outline, phase7_draft, phase8_editorial_qa,
    phase9_originality, phase10_polish, phase11_rating,
    phase12_metadata, phase13_exports, phase14_marketplace,
    phase15_kindle_preview, phase16_report,
)


def _extract_json_from_md(md: str) -> dict:
    m = re.search(r"```json\n(.*?)\n```", md, re.DOTALL)
    if not m:
        raise RuntimeError("no json block found")
    return json.loads(m.group(1))


def main() -> int:
    ideation = json.loads((ART / "01_ideation.json").read_text())
    topic = json.loads((ART / "03_final_topic.json").read_text())
    plan = json.loads((ART / "04_book_plan.json").read_text())

    # carry the already-completed phase records forward in the in-memory list
    # so phase16's roll-up sees them
    from run_e2e import PHASES
    PHASES.extend([
        {"phase": "0", "status": "PASS",
         "summary": "Sentinel passed. No cloud generation keys active.",
         "artifacts": [str(ART / "audit/local_llm_routing.json")]},
        {"phase": "1", "status": "PASS",
         "summary": f"5 concepts generated, top_two selected",
         "artifacts": [str(ART / "01_ideation.json")]},
        {"phase": "2", "status": "WARN",
         "summary": "Dossier produced; tag coverage=False (some claims un-tagged)",
         "artifacts": [str(ART / "02_research_dossier.md")]},
        {"phase": "3", "status": "PASS",
         "summary": f"Selected: {topic.get('final_title')}",
         "artifacts": [str(ART / "03_final_topic.md")]},
        {"phase": "4", "status": "PASS",
         "summary": "Plan produced (logline, acts, world rules, style guide)",
         "artifacts": [str(ART / "04_book_plan.md")]},
    ])

    log("RESUME from Phase 5")
    chars, world = phase5_bibles(topic, plan)
    outline = phase6_outline(topic, plan, chars)
    draft = phase7_draft(outline, plan, chars, world)
    qa = phase8_editorial_qa(draft, chars, world)
    phase9_originality(qa["fixed_manuscript"], topic)
    polished = phase10_polish(qa["fixed_manuscript"])
    final_ms, _ = phase11_rating(polished)
    metadata, fb, _ = phase12_metadata(topic, plan, final_ms)
    exports = phase13_exports(topic, fb, final_ms, metadata)
    phase14_marketplace(metadata, exports)
    phase15_kindle_preview(topic, final_ms, metadata)
    phase16_report()
    return 0


if __name__ == "__main__":
    sys.exit(main())
