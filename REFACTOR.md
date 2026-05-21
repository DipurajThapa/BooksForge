# BookForge Architectural Refactor: 4-Workflow System Design

**Version:** 1.0  
**Status:** Ready for Claude Code Execution  
**Complexity Level:** HIGH (Multi-Phase, Cross-Module Dependencies)  
**Estimated Duration:** 8–12 Claude Code sessions

---

## EXECUTIVE SUMMARY

Refactor BookForge from monolithic write-publish pipeline into **4 independent, sequenced workflows** (Planning → Writing → Editing → Formatting) connected via **formal handoff process**. Each workflow operates at different human/AI ratios with integrated error handling, rollback capability, and real-time completion tracking.

**Key Constraint:** No breaking changes to existing users mid-workflow; implement feature-flag rollout for new architecture.

---

## PHASE 0: PRE-REFACTOR SETUP (Claude Code Session 1)

### Objective
Establish scaffolding, feature flags, audit logging, and rollback harness.

### Tasks

#### 0.1 — Feature Flag & Rollback Infrastructure
- [ ] Create `src/lib/feature-flags.ts` with `BookForgeV2` flag (default: disabled)
- [ ] Implement dual-mode router: legacy monolithic vs. new workflow system
- [ ] Add `ROLLBACK_ENABLED` flag with version checkpoints
- [ ] Create `src/lib/audit-log.ts`: immutable event log (action, actor, timestamp, state_delta, rollback_hash)
- [ ] Database: Add `audit_logs` table + `workflow_snapshots` table (stores full state at phase boundaries)

#### 0.2 — Workflow State Management
- [ ] Create `src/lib/workflow-state.ts` (immutable state machine)
  - States: `PLANNING`, `PLANNING_COMPLETE`, `WRITING`, `WRITING_COMPLETE`, `EDITING`, `EDITING_COMPLETE`, `FORMATTING`, `PUBLISHED`
  - Transition rules: Only forward in sequence (enforce via middleware)
  - Rollback only to previous *complete* state
- [ ] Database schema additions:
  ```sql
  ALTER TABLE books ADD COLUMN workflow_state VARCHAR(20) DEFAULT 'PLANNING';
  ALTER TABLE books ADD COLUMN workflow_version INT DEFAULT 1;
  ALTER TABLE books ADD COLUMN last_rollback_point TEXT;
  
  CREATE TABLE workflow_handoffs (
    id UUID PRIMARY KEY,
    book_id UUID NOT NULL,
    from_workflow VARCHAR(20) NOT NULL,
    to_workflow VARCHAR(20) NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR(20) DEFAULT 'PENDING',
    created_at TIMESTAMP,
    completed_at TIMESTAMP,
    error_log TEXT
  );
  ```

#### 0.3 — Error Handling & Backlog System
- [ ] Create `src/lib/error-tracker.ts`: Centralized error registry
  - Severity levels: `CRITICAL` (halt), `HIGH` (retry), `MEDIUM` (flag for backlog), `LOW` (log only)
  - Implement auto-retry logic (exponential backoff) for `HIGH` errors
  - Dead-letter queue for `CRITICAL` + `MEDIUM` errors → Backlog table
- [ ] Database:
  ```sql
  CREATE TABLE backlog_items (
    id UUID PRIMARY KEY,
    book_id UUID,
    workflow_stage VARCHAR(20),
    error_type VARCHAR(255),
    error_message TEXT,
    context JSONB,
    severity VARCHAR(20),
    created_at TIMESTAMP,
    resolved BOOLEAN DEFAULT FALSE,
    resolution_notes TEXT,
    assigned_to UUID
  );
  ```
- [ ] Add Backlog Dashboard endpoint: `GET /api/backlog?book_id=...` with filters (severity, workflow, status)

#### 0.4 — Testing Scaffold
- [ ] Create `tests/workflows/setup.test.ts` — sanity test for feature flags + state transitions
- [ ] Create `tests/fixtures/sample-books.ts` — 5 sample books at different workflow states
- [ ] Add `npm run test:workflows` script

### Expected Output
- ✅ Feature flags functional (test via dashboard toggle)
- ✅ State machine enforces sequence (try invalid transition, verify rejection)
- ✅ Audit log captures all state changes
- ✅ Backlog system ready to receive errors

### Error Handling in Phase 0
- **If migration fails:** Rollback schema changes, disable flag
- **If state machine conflicts:** Log and use legacy router (no users blocked)
- **Capture in backlog:** Any schema migration warnings

---

## PHASE 1: PLANNING WORKFLOW (Claude Code Sessions 2–3)

### Objective
Build independent Planning module (60% human input + 40% AI) with handoff to Writing.

### Architecture
```
User Input (topic, genre, audience)
    ↓
Planning Workflow Orchestrator
    ├→ Ideation Engine (Claude API: claude-sonnet-4)
    ├→ Market Research Module (Web search + analysis)
    ├→ Predictability Scorer (Custom algorithm)
    ├→ Originality Check (Plagiarism + novelty)
    └→ Handoff Generator
         ↓
         JSON Payload → workflow_handoffs table
         ↓
         Writing Workflow (Phase 2)
```

### Tasks

#### 1.1 — Ideation Engine
- [ ] Endpoint: `POST /api/workflows/planning/ideate`
  - Input: `{ topic: string, genre: string, targetAudience: string, userNotes?: string }`
  - Process:
    ```
    1. Call Claude API with prompt:
       "Based on topic '{topic}', genre '{genre}', audience '{targetAudience}', 
        generate 5 book concept variations with:
        - Hook (1-line elevator pitch)
        - Core premise
        - Unique angle vs. existing books
        - 3-sentence market rationale
        
        Return JSON: { concepts: [{ hook, premise, angle, rationale }] }"
    2. Store results in `books.ideation_output` (JSONB)
    3. Return to UI with "approval required" flag
    ```
  - Error handling:
    - If Claude API fails: Backlog item (severity: HIGH, retry with exponential backoff)
    - If response is malformed JSON: Log + ask user to retry

#### 1.2 — Market Research Module
- [ ] Endpoint: `POST /api/workflows/planning/market-research`
  - Input: `{ genre: string, targetAudience: string, ideationOutput?: object }`
  - Process:
    ```
    1. Web search (use existing web search tool):
       - Query: "bestselling {genre} books 2025 {targetAudience}"
       - Extract: Top 10 books (title, author, publish date, review count, avg rating)
    2. Analysis (Claude API):
       "Analyze these 10 bestsellers in {genre} for {targetAudience}:
        {book list}
        
        Provide:
        - Common themes/tropes (%)
        - Target reader profile (age, interests, pain points)
        - Pricing range (paperback, hardcover, ebook)
        - Typical chapter count & length
        - Key differentiators of top 3 vs. rest
        
        Return JSON: { commonThemes, readerProfile, pricing, structure, differentiators }"
    3. Store in `books.market_research` (JSONB)
    4. Return analysis + recommendation for positioning
    ```
  - Error handling:
    - Web search timeout: Use cached data from last 7 days (backlog medium)
    - Claude API failure: Backlog (HIGH, retry)

#### 1.3 — Predictability Scorer (CRITICAL ALGORITHM)
- [ ] Endpoint: `POST /api/workflows/planning/predictability-score`
  - Input: Full planning data (topic, genre, audience, market research, ideation, user notes)
  - **Algorithm:** Weighted scoring model
    ```
    Score Components (max 100):
    
    1. Market Demand (25 pts)
       - Genre popularity in last 12 months: +0–10 pts
       - Target audience size: +0–8 pts
       - Pricing tier competitiveness: +0–7 pts
    
    2. Concept Uniqueness (20 pts)
       - Overlap with top 50 bestsellers: −5 to +20 pts
       - New angle/subgenre: +0–10 pts
       - Author angle (if stated): +0–5 pts
    
    3. Execution Feasibility (15 pts)
       - Outline clarity: +0–8 pts
       - Target chapter count achievability: +0–7 pts
    
    4. Audience Match (20 pts)
       - Reader profile alignment: +0–12 pts
       - Emotional resonance score (Claude assess): +0–8 pts
    
    5. Marketing Potential (20 pts)
       - Hook strength: +0–10 pts
       - Serialization potential (series, spinoffs): +0–5 pts
       - Social media virality score: +0–5 pts
    
    **Final Score:** Sum of all components
    **Threshold:** ≥65 → "Go" | <65 → "Refine" (flag for re-ideation or market pivot)
    ```
  - Output:
    ```json
    {
      "score": 72,
      "status": "Go",
      "breakdown": { marketDemand: 18, uniqueness: 16, feasibility: 12, audienceMatch: 18, marketingPotential: 8 },
      "strengths": ["Strong market demand", "Unique angle"],
      "risks": ["New author in this genre", "Niche audience"],
      "recommendation": "Proceed with confidence; focus marketing on [niche platform]"
    }
    ```
  - Error handling:
    - Missing data: Return partial score with warnings (user can re-submit)
    - Algorithm exception: Fallback to manual review queue

#### 1.4 — Originality & Plagiarism Check
- [ ] Endpoint: `POST /api/workflows/planning/originality-check`
  - Input: Concept summary (hook + premise), intended chapters outline (if available)
  - Process:
    ```
    1. Web search for similar books (title + topic keywords)
    2. Check against plagiarism API (e.g., Turnitin or Copyscape) if integrated
    3. Claude assessment:
       "Given this book concept: {concept}, and these similar books: {list},
        provide:
        - Plagiarism risk (0–100%, 0=safe, 100=duplicate)
        - Unique differentiators vs. top 3 similar books
        - Originality assessment
        
        Return JSON: { plagiarismRisk, differentiators, originalityScore }"
    ```
  - Output:
    ```json
    {
      "plagiarismRisk": 12,
      "status": "Original",
      "similarBooks": [{ title, author, similarity% }],
      "differentiators": ["Unique POV", "New setting"],
      "clearanceToProceeed": true
    }
    ```

#### 1.5 — Planning Finalization & Handoff
- [ ] Endpoint: `POST /api/workflows/planning/finalize`
  - Input: Approval flag + any user refinements
  - Process:
    ```
    1. Validate score ≥65, originality check passed
    2. If not: Return "Refinement Required" with specifics (re-do ideation, market research, or concept pivot)
    3. If passed:
       - Compile Planning Summary:
         {
           "topic": "...",
           "genre": "...",
           "targetAudience": "...",
           "planningNotes": "...",
           "ideation": { ... },
           "marketResearch": { ... },
           "predictabilityScore": 72,
           "originalityStatus": "Original",
           "marketPositioning": "...",
           "recommendedChapterCount": 25,
           "recommendedWordCount": 75000,
           "tone": "...",
           "pov": "...",
           "keyThemes": ["...", "..."],
           "timestamp": "2025-05-21T..."
         }
       - Create workflow_handoffs record:
         {
           from: "PLANNING",
           to: "WRITING",
           payload: Planning Summary,
           status: "READY"
         }
       - Update book.workflow_state → "PLANNING_COMPLETE"
       - Create workflow_snapshots record (full state backup)
    ```
  - Return: "Handoff ready. Writing workflow can now begin."

#### 1.6 — Planning Dashboard
- [ ] UI: Show Planning progress for all books
  - Columns: Book Title | Ideation | Market Research | Score | Originality | Status | Handoff Date
  - Actions: Edit ideation, re-run market research, view full report, finalize
  - Filtering: Status (in-progress, ready for handoff, refinement needed), score (>65, <65)

### Testing for Phase 1
```bash
npm run test:workflows -- --phase planning

# Specific tests:
tests/workflows/planning/ideation.test.ts
tests/workflows/planning/market-research.test.ts
tests/workflows/planning/predictability-scorer.test.ts
tests/workflows/planning/originality-check.test.ts
tests/workflows/planning/handoff.test.ts

# Test cases:
✓ Ideation generates 5 valid concepts
✓ Market research fetches top 10 books
✓ Predictability scorer returns 0–100 range
✓ Score ≥65 triggers "Go" status
✓ Score <65 triggers "Refine" workflow
✓ Originality check returns valid JSON
✓ Handoff payload validates schema
✓ Workflow state transitions to PLANNING_COMPLETE
✓ Error: API failure → backlog item created
✓ Error: Missing data → partial score with warnings
```

### Backlog Capture for Phase 1
- Runs of predictability scorer with score <65 (refinement needed)
- Market research queries that timeout (fallback to cache)
- Any API failures (Claude, web search)

### Expected Output
- ✅ Planning module fully functional (test with 5 sample books)
- ✅ All 4 sub-modules return correct JSON
- ✅ Handoff payload created and validated
- ✅ Dashboard shows planning progress
- ✅ Error handling directs failures to backlog

---

## PHASE 2: WRITING WORKFLOW (Claude Code Sessions 4–6)

### Objective
Build independent Writing module (10% human + 90% AI) consuming Planning handoff OR direct user uploads.

### Architecture
```
Input Source A: Planning Handoff    OR    Input Source B: Direct User Upload
    ↓                                           ↓
    └─────────────────────┬─────────────────────┘
                          ↓
            Writing Workflow Orchestrator
                ├→ Character Setup Module
                ├→ Comprehensive Outline Engine
                ├→ Outline Review & Refinement
                ├→ Chapter-by-Chapter Writer
                └→ Handoff Generator
                     ↓
                     JSON Payload (completed manuscript)
                     ↓
                     Editing Workflow (Phase 3)
```

#### 2.1 — Character & Metadata Setup
- [ ] Endpoint: `POST /api/workflows/writing/setup`
  - Input: `{ bookId, planningPayload || userUploadedOutline, characters?: array, worldBuilding?: object }`
  - Process:
    ```
    1. Extract/generate character list:
       - If provided: Validate schema
       - If not: Claude AI generates 5–10 main characters based on genre + audience
         Prompt: "For a {genre} book targeting {audience} with premise '{premise}',
                   generate main characters: name, age, role, motivation, arc.
                   Return JSON: { characters: [{ name, age, role, motivation, arc }] }"
    2. Store in `books.characters` (JSONB)
    3. Extract world-building (setting, time period, rules, etc.) from planning
    4. Return to UI: "Character setup complete. Review and proceed to outline."
    ```

#### 2.2 — Comprehensive Outline Engine
- [ ] Endpoint: `POST /api/workflows/writing/generate-outline`
  - Input: `{ bookId, planningPayload, characters, userOutlineNotes?: string }`
  - Process:
    ```
    1. Call Claude API with system prompt + instructions:
       "You are a master book outline builder. Given:
        - Genre: {genre}
        - Target: {audience}
        - Premise: {premise}
        - Recommended length: {chapterCount} chapters, {wordCount} words
        - Characters: {characters}
        - Key themes: {themes}
        
        Generate a **comprehensive outline** with:
        - Chapter-by-chapter breakdown (title, word count, summary)
        - Acts/sections (if applicable to genre)
        - Character appearances per chapter (name: %)
        - Plot points/turning points marked
        - Pacing analysis (action %, dialogue %, introspection %)
        
        Return JSON: {
          chapters: [
            {
              chapterNum: 1,
              title: '...',
              summary: '...',
              wordCount: 3500,
              keyPlotPoints: ['...'],
              characters: [{ name, appearances: 45 }],
              pacing: { action: 30, dialogue: 50, introspection: 20 }
            }
          ],
          acts: [...],
          totalEstimatedWords: 75000,
          pacing_analysis: { ... }
        }"
    2. Store in `books.outline` (JSONB)
    3. Return outline with "ready for review" flag
    ```
  - Error handling:
    - Claude API timeout: Backlog (HIGH, retry)
    - Outline validation fails (schema mismatch): Request regeneration

#### 2.3 — Outline Review & Fine-Tuning
- [ ] Endpoint: `PUT /api/workflows/writing/refine-outline`
  - Input: `{ bookId, outlineId, refinements: [{ chapterNum, field, newValue }] }`
  - Process:
    ```
    1. Apply user edits to outline
    2. Run validation:
       - Total word count still matches target (±5%)
       - All chapters have plot points
       - Character appearances sum to reasonable (%s)
    3. If validation passes: Update books.outline, return "Outline finalized"
    4. If validation fails: Return specific errors + suggestions
    ```
  - Error handling:
    - Invalid refinements: Return validation errors with auto-suggestions
    - User manually breaks structure: Offer AI "repair" option (Claude regenerates affected chapters)

#### 2.4 — Chapter-by-Chapter Writer
- [ ] Endpoint: `POST /api/workflows/writing/write-chapter`
  - Input: `{ bookId, chapterNum, outline, previousChapterContext? }`
  - Process:
    ```
    1. Build system context:
       - Full outline + this chapter summary
       - Character guide (name, voice, motivation)
       - Genre conventions + tone guidelines
       - Previous chapter(s) ending (last 500 words) for continuity
    
    2. Build user prompt:
       "Write Chapter {chapterNum}: {title}
        
        Summary: {summary}
        Target word count: {wordCount}
        Characters featured: {characters with %}
        Plot points to hit: {bulletList}
        
        Tone: {tone}
        POV: {pov}
        Pacing target: {actionPct}% action, {dialoguePct}% dialogue
        
        Previous chapter context:
        {last 500 words}
        
        Write this chapter maintaining:
        - Consistent character voices
        - Pacing alignment
        - Plot momentum
        - Genre conventions
        - Thematic coherence
        
        Return: Full chapter text (approximately {wordCount} words)"
    
    3. Call Claude API (model: claude-opus-4-6 for best quality) with streaming:
       - Stream output to UI (real-time progress)
       - Store chapter draft in `books.chapters` table
    
    4. Post-write validation:
       - Word count within 10% of target
       - Character names consistent
       - No plot point misses (list any gaps)
       - Tone consistency check (flag tone shifts)
    ```
  - Error handling:
    - Word count significantly off: Offer "expand" or "condense" regeneration
    - Character name inconsistencies: Auto-correct + log
    - Claude API failure: Save partial draft to backlog (HIGH, allow user to retry)

#### 2.5 — Batch Writing Mode
- [ ] Endpoint: `POST /api/workflows/writing/write-all-chapters`
  - Input: `{ bookId }`
  - Process:
    ```
    1. Iterate through all chapters sequentially
    2. For each chapter:
       - Load previous chapter context (last 500 words)
       - Call write-chapter endpoint
       - Wait for completion (or set configurable timeout)
       - Capture any errors → backlog items
       - Update UI progress bar
    3. On completion or timeout:
       - Mark workflow as "WRITING_COMPLETE" OR "WRITING_PARTIAL" (if errors)
       - Summarize writing run: X/Y chapters completed, Z errors to resolve
    ```
  - Error handling:
    - Mid-batch failure: Pause, create backlog items, allow resume from failed chapter
    - User cancels: Partial manuscript saved, can resume later

#### 2.6 — Writing Handoff
- [ ] Endpoint: `POST /api/workflows/writing/finalize`
  - Input: `{ bookId }`
  - Process:
    ```
    1. Validate all chapters written:
       - Query chapters table for bookId
       - If any chapters missing: Return error + list of missing chapters
    2. Compile Manuscript Summary:
       {
         "bookId": "...",
         "title": "...",
         "totalWords": 75432,
         "chapterCount": 25,
         "completionDate": "...",
         "chapters": [
           {
             "num": 1,
             "title": "...",
             "wordCount": 3200,
             "text": "...",
             "validationNotes": "..."
           }
         ],
         "manuscriptQualityScore": 0–100 (auto-generated via Claude review),
         "issues": [{ chapterNum, issue, severity }]
       }
    3. Create workflow_handoffs record:
       {
         from: "WRITING",
         to: "EDITING",
         payload: Manuscript Summary,
         status: "READY"
       }
    4. Update book.workflow_state → "WRITING_COMPLETE"
    5. Create workflow_snapshots record
    ```
  - Return: "Manuscript complete. Editing workflow ready to begin."

#### 2.7 — Writing Dashboard
- [ ] UI: Writing progress tracking
  - Columns: Book Title | Chapters Written / Total | Word Count | Last Edited | Issues | Status
  - Actions: Write chapter, batch write, view draft, finalize
  - Real-time progress bar for batch writing

### Testing for Phase 2
```bash
npm run test:workflows -- --phase writing

tests/workflows/writing/character-setup.test.ts
tests/workflows/writing/outline-generator.test.ts
tests/workflows/writing/outline-refinement.test.ts
tests/workflows/writing/chapter-writer.test.ts
tests/workflows/writing/batch-writing.test.ts
tests/workflows/writing/handoff.test.ts

# Test cases:
✓ Character setup generates valid schema
✓ Outline engine produces all chapters
✓ Chapter word counts match target (±10%)
✓ Character names consistent across chapters
✓ Batch write completes all chapters sequentially
✓ Handoff payload includes full manuscript
✓ Error: Claude timeout → backlog item + partial draft saved
✓ Error: Missing chapters → finalize blocked with list
✓ Workflow state transitions to WRITING_COMPLETE
✓ Previous chapter context flows to next chapter
```

### Backlog Capture for Phase 2
- Chapters with word count >10% off target
- Tone consistency failures
- Character name inconsistencies (auto-corrected + logged)
- Claude API timeouts
- Batch write interruptions

### Expected Output
- ✅ Writing module fully functional
- ✅ Can consume Planning handoff OR direct user upload
- ✅ All chapters written with validation
- ✅ Handoff payload created with full manuscript
- ✅ Dashboard shows writing progress real-time

---

## PHASE 3: EDITING WORKFLOW (Claude Code Sessions 7–8)

### Objective
Build independent Editing module (25% human + 75% AI) consuming Writing handoff OR direct user uploads.

### Architecture
```
Input Source A: Writing Handoff    OR    Input Source B: Direct User Upload
    ↓                                           ↓
    └─────────────────────┬─────────────────────┘
                          ↓
        Editing Workflow Orchestrator + AI Tools
            ├→ Ready-to-Edit Dashboard
            ├→ Editing Mode Router (AI vs. Manual)
            ├→ AI Editing Engine
            │   ├→ Grammar/Style Enhancer
            │   ├→ Flow & Consistency Checker
            │   ├→ Plagiarism / Humanizer / Anti-AI Detector
            │   ├→ Content Enrichment (quotes, images, elements)
            │   └→ Chapter Continuity Validator
            ├→ Manual Review Interface
            ├→ Kindle Preview
            └→ Handoff Generator
                 ↓
                 Final Edited Manuscript
                 ↓
                 Formatting Workflow (Phase 4)
```

#### 3.1 — Ready-to-Edit Dashboard
- [ ] Endpoint: `GET /api/workflows/editing/dashboard`
  - Columns: Book Title | From Workflow | Chapter/Status | Edit Type | Last Edited | Actions
  - Filters: Source (Writing handoff vs. direct upload), edit mode (AI, manual, hybrid), status (ready, in-progress, complete)
  - Actions: Start editing, batch edit, view preview, finalize

#### 3.2 — Edit Mode Router
- [ ] Endpoint: `POST /api/workflows/editing/start-edit`
  - Input: `{ bookId, chapterId, editMode: "ai" | "manual" | "hybrid", editScope?: "section" }`
  - Process:
    ```
    1. Load chapter content
    2. Branch on editMode:
       - "ai": Jump to 3.3 (AI Editing Engine)
       - "manual": Jump to 3.5 (Manual Review Interface)
       - "hybrid": Load both interfaces side-by-side (user chooses per section)
    3. Initialize editing session → store in books.editing_sessions table
    ```

#### 3.3 — AI Editing Engine
- [ ] Sub-module: Grammar & Style Enhancement
  - Endpoint: `POST /api/workflows/editing/enhance-grammar`
  - Input: `{ chapterId, scope: "full" | "paragraph", paragraphId? }`
  - Process:
    ```
    1. Extract target text (chapter or paragraph)
    2. Call Claude API:
       "Edit this {genre} text for grammar, clarity, and style consistency:
        
        {text}
        
        Maintain:
        - Original tone and voice
        - Character personalities
        - Pacing
        - Thematic content
        
        Return JSON: {
          editedText: '...',
          changes: [
            {
              originalPhrase: '...',
              editedPhrase: '...',
              reason: 'grammar' | 'clarity' | 'style' | 'flow',
              severity: 'minor' | 'major'
            }
          ],
          summaryNotes: 'X grammar fixes, Y clarity improvements, Z style enhancements'
        }"
    3. Return diff view (highlight changes) to UI
    4. User accepts/rejects per change or bulk-accept
    5. Store final text in books.chapters (text_version field)
    ```
  - Error handling:
    - Large chapter timeout: Split into sections, process sequentially

- [ ] Sub-module: Flow & Consistency Checker
  - Endpoint: `POST /api/workflows/editing/check-consistency`
  - Input: `{ bookId, chapterId }`
  - Process:
    ```
    1. Load current chapter + previous chapter (last 1000 words) + outline
    2. Call Claude API:
       "Review this chapter for flow and consistency:
        
        Previous Chapter Ending:
        {last 1000 words of prev chapter}
        
        Current Chapter:
        {full chapter}
        
        Outline Context:
        {outline entry for this chapter}
        
        Check for:
        - Abrupt tone/POV shifts
        - Character voice consistency
        - Plot logic continuity
        - Unresolved threads from previous chapter
        - Pacing alignment with outline
        - Thematic consistency
        
        Return JSON: {
          flowScore: 0–100,
          consistencyIssues: [
            {
              type: 'tone' | 'character' | 'plot' | 'theme' | 'pacing',
              location: 'paragraph X',
              issue: '...',
              suggestion: '...',
              severity: 'critical' | 'high' | 'medium' | 'low'
            }
          ],
          summaryNotes: '...'
        }"
    3. Return issues ranked by severity
    4. UI allows user to apply suggestions or manually edit
    ```

- [ ] Sub-module: Plagiarism / Humanizer / Anti-AI Detector
  - Endpoint: `POST /api/workflows/editing/humanize-detect`
  - Input: `{ chapterId, tool: "plagiarism" | "humanizer" | "antiAI" }`
  - Process (varies by tool):
    
    **Plagiarism Check:**
    ```
    1. Web search for plagiarism API integration (e.g., Turnitin, Copyscape)
    2. Submit chapter text
    3. Return: Plagiarism % + flagged passages + sources
    4. User can: Rewrite flagged sections, accept if <5%, or escalate
    ```
    
    **Humanizer:**
    ```
    1. Call Claude API:
       "Make this AI-generated text sound more human and authentic:
        
        {chapter text}
        
        Enhance:
        - Natural language patterns
        - Realistic dialogue
        - Human-like imperfections (occasional typos, stutters, hesitations where appropriate)
        - Emotional authenticity
        - Varied sentence structure
        
        Return JSON: {
          humanizedText: '...',
          changesApplied: [
            {
              originalPhrase: '...',
              humanizedPhrase: '...',
              reason: '...'
            }
          ]
        }"
    ```
    
    **Anti-AI Detector:**
    ```
    1. Call AI-detection service (e.g., OpenAI's classifier, Turnitin AI)
    2. Return: AI-detection score (0–100%, 100 = human-written)
    3. Flag sections >50% likely AI-written
    4. Suggest humanizer or rewrite
    ```

- [ ] Sub-module: Content Enrichment
  - Endpoint: `POST /api/workflows/editing/enrich-content`
  - Input: `{ chapterId, enrichmentType: "quotes" | "images" | "annotations" }`
  - Process (varies by type):
    
    **Relevant Quotes:**
    ```
    1. Extract chapter theme/topic
    2. Web search for relevant quotes from famous authors/figures
    3. Return: List of quotes (author, text, relevance score)
    4. User selects quotes to insert with placement suggestions
    ```
    
    **Chapter Images:**
    ```
    1. Generate image description from chapter key scene
    2. Call image generation API (if available) or suggest stock photo search terms
    3. Return: Generated image OR search keywords
    4. User inserts image with proper credits
    ```
    
    **Annotations/Footnotes:**
    ```
    1. Identify terms requiring explanation (genre-specific jargon, historical refs, etc.)
    2. Suggest annotation placements
    3. User adds annotations maintaining chapter flow
    ```

#### 3.4 — Chapter Continuity Validator (Cross-Chapter)
- [ ] Endpoint: `POST /api/workflows/editing/validate-continuity`
  - Input: `{ bookId, fromChapter, toChapter }`
  - Process:
    ```
    1. Load chapters in range
    2. Call Claude API:
       "Review this book section for narrative continuity:
        
        Chapters {fromChapter}–{toChapter}:
        {full text}
        
        Check for:
        - Character consistency across chapters
        - Plot logic and causality
        - Timeline coherence
        - Dialogue consistency
        - Setting details
        - Resolved vs. unresolved threads
        - Pacing across section
        
        Return JSON: {
          continuityScore: 0–100,
          issues: [
            {
              affectedChapters: [X, Y],
              issue: '...',
              suggestion: '...',
              severity: 'critical' | 'high' | 'medium'
            }
          ]
        }"
    3. Return issues + allow user to batch-apply suggestions
    ```

#### 3.5 — Manual Review Interface
- [ ] UI: Code-editor-style interface (like VSCode)
  - Left panel: Chapter outline + metadata
  - Center: Editable chapter text with line numbers
  - Right panel: AI suggestions sidebar (grammar, flow, consistency)
  - Bottom: Kindle preview (optional)
  - Versioning: Save drafts, view edit history

#### 3.6 — Kindle Preview
- [ ] Endpoint: `GET /api/workflows/editing/kindle-preview?chapterId=...`
  - Process:
    ```
    1. Load chapter + formatting templates
    2. Render as Kindle would (font, line spacing, page breaks)
    3. Return HTML + embedded CSS
    4. UI: Render in sidebar or modal
    ```
  - Features: Adjust font size, page width, theme (light/dark)

#### 3.7 — Batch Editing Mode
- [ ] Endpoint: `POST /api/workflows/editing/batch-edit`
  - Input: `{ bookId, chapters: [1–25], tool: "humanizer" | "consistency" | "grammar" }`
  - Process:
    ```
    1. Iterate chapters sequentially
    2. Apply selected tool to each chapter
    3. Collect all changes + present for bulk review
    4. Allow user to accept/reject per chapter or globally
    5. On completion: Mark chapters as "reviewed"
    ```

#### 3.8 — Editing Finalization & Handoff
- [ ] Endpoint: `POST /api/workflows/editing/finalize`
  - Input: `{ bookId }`
  - Process:
    ```
    1. Validate all chapters edited (or marked "skip editing"):
       - Query chapters for review_status
       - If any "pending review": Return error + list
    2. Compile Edited Manuscript:
       {
         "bookId": "...",
         "chapters": [
           {
             "num": 1,
             "title": "...",
             "text": "... (edited)",
             "editingNotes": "...",
             "toolsApplied": ["grammar", "humanizer"],
             "plagarismScore": 2,
             "humanScore": 98,
             "continuityScore": 95
           }
         ],
         "overallEditingScore": 95,
         "editingSummary": "All chapters reviewed. Plagiarism <5%, AI detection <2%, continuity >90%.",
         "timestamp": "..."
       }
    3. Create workflow_handoffs record
    4. Update book.workflow_state → "EDITING_COMPLETE"
    5. Create workflow_snapshots record
    ```
  - Return: "Edited manuscript ready for formatting."

### Testing for Phase 3
```bash
npm run test:workflows -- --phase editing

tests/workflows/editing/dashboard.test.ts
tests/workflows/editing/grammar-enhancement.test.ts
tests/workflows/editing/flow-consistency.test.ts
tests/workflows/editing/humanizer.test.ts
tests/workflows/editing/plagiarism-check.test.ts
tests/workflows/editing/continuity-validator.test.ts
tests/workflows/editing/kindle-preview.test.ts
tests/workflows/editing/batch-editing.test.ts
tests/workflows/editing/handoff.test.ts

# Test cases:
✓ Grammar enhancer returns change list
✓ Flow checker identifies tone shifts
✓ Humanizer increases human-detection score
✓ Plagiarism checker returns <5% for original text
✓ Continuity validator flags missing threads
✓ Kindle preview renders correctly
✓ Batch edit applies tool to all chapters
✓ Manual edits saved to chapters table
✓ Handoff payload includes all editing scores
✓ Workflow state transitions to EDITING_COMPLETE
```

### Backlog Capture for Phase 3
- Plagiarism flags (>5%)
- AI detection flags (>5%)
- Critical continuity issues
- Grammar/flow issues marked "high severity"
- Manual edits that break character voice (flagged by consistency checker)

### Expected Output
- ✅ Editing module fully functional
- ✅ Can consume Writing handoff OR direct user upload
- ✅ All AI tools (grammar, humanizer, plagiarism, continuity) working
- ✅ Manual review interface functional
- ✅ Handoff payload with editing scores
- ✅ Backlog items for unresolved issues

---

## PHASE 4: FORMATTING WORKFLOW (Claude Code Session 9)

### Objective
Build independent Formatting module (10% human + 90% AI) consuming Editing handoff OR direct user uploads.

### Architecture
```
Input Source A: Editing Handoff    OR    Input Source B: Direct User Upload
    ↓                                           ↓
    └─────────────────────┬─────────────────────┘
                          ↓
        Formatting Workflow Orchestrator
            ├→ Template Selection / Builder
            ├→ Genre-Based Format Auto-Generator
            ├→ Custom Template Creation
            ├→ Format Application Engine
            ├→ Multi-Format Export (PDF, EPUB, DOCX, MOBI)
            └→ Publishing Ready Handoff
```

#### 4.1 — Template Library
- [ ] Database: `templates` table
  ```sql
  CREATE TABLE templates (
    id UUID PRIMARY KEY,
    name VARCHAR(255),
    genre VARCHAR(100),
    subGenre VARCHAR(100),
    targetAudience VARCHAR(255),
    templateType VARCHAR(20), -- 'predefined' | 'custom' | 'community'
    cssStyles TEXT, -- Full CSS for Kindle/ePub
    designTokens JSONB, -- Font, colors, spacing
    chapterTemplate TEXT, -- HTML structure for chapters
    frontMatterTemplate TEXT, -- TOC, dedication, etc.
    backMatterTemplate TEXT, -- Author bio, etc.
    createdBy UUID,
    createdAt TIMESTAMP,
    usageCount INT DEFAULT 0
  );
  ```
- [ ] Seed initial templates (50+ pre-built):
  - Romance: Contemporary, Historical, Paranormal, etc.
  - Thriller: Mystery, Spy, Psychological, etc.
  - Fantasy: Epic, Urban, Paranormal, etc.
  - Sci-Fi: Space Opera, Cyberpunk, Post-Apocalyptic, etc.
  - Non-Fiction: Business, Self-Help, Memoir, etc.
  - Children's: Picture books, Middle Grade, YA, etc.

#### 4.2 — Template Selection Endpoint
- [ ] Endpoint: `GET /api/workflows/formatting/templates?genre=...&subGenre=...&audience=...`
  - Returns: List of 10–20 matching templates (ranked by relevance + popularity)
  - Fields per template: Name, preview image, description, characteristics, usageCount
  - User selects or searches for specific template

#### 4.3 — Genre-Based Format Auto-Generator
- [ ] Endpoint: `POST /api/workflows/formatting/generate-format`
  - Input: `{ bookId, genre, subGenre, targetAudience }`
  - Process:
    ```
    1. Call Claude API:
       "Generate optimal formatting for this {genre} {subGenre} book:
        
        Provide:
        - Font recommendations (serif/sans-serif, size)
        - Color scheme (based on genre conventions)
        - Chapter header style
        - Page margins + line spacing (for readability)
        - Image placement guidelines
        - Special formatting (quote blocks, footnotes, etc.)
        
        Return JSON: { font, fontSize, fontColor, backgroundColor, ... }"
    
    2. Create temporary template from generated spec
    3. Return to UI: "Auto-generated format. Customize or apply?"
    ```

#### 4.4 — Custom Template Builder
- [ ] Endpoint: `POST /api/workflows/formatting/create-template`
  - Input: `{ name, genre, description, designTokens, chapterTemplate, ... }`
  - Process:
    ```
    1. Validate CSS + HTML templates (no script injection)
    2. Store in templates table
    3. Return: "Template created. Available for future books."
    ```
  - UI: WYSIWYG template editor with live preview

#### 4.5 — Format Application Engine
- [ ] Endpoint: `POST /api/workflows/formatting/apply-format`
  - Input: `{ bookId, templateId }`
  - Process:
    ```
    1. Load edited manuscript chapters
    2. Load template (CSS + HTML)
    3. Iterate chapters:
       - Wrap chapter text in template HTML
       - Insert images (if applicable)
       - Apply CSS styles
       - Ensure proper pagination (Kindle + ePub constraints)
    4. Generate front matter:
       - Title page
       - Table of Contents (auto-generated)
       - Dedication (if provided)
       - Author bio (if provided)
    5. Generate back matter:
       - About the author
       - Related works
       - Acknowledgments
    6. Store formatted manuscript in books.formatted_manuscript (JSONB)
    ```
  - Error handling:
    - CSS parsing error: Fall back to default template
    - Image loading failed: Skip image, warn user

#### 4.6 — Multi-Format Export
- [ ] Endpoint: `POST /api/workflows/formatting/export`
  - Input: `{ bookId, formats: ["pdf" | "epub" | "docx" | "mobi"], qualityLevel: "draft" | "final" }`
  - Process (varies by format):
    
    **PDF Export:**
    ```
    Use Playwright:
    1. Render formatted HTML to PDF
    2. Optimize images
    3. Add PDF metadata (title, author, keywords)
    4. Return: Download link
    ```
    
    **EPUB Export:**
    ```
    Use epub-gen-memory (existing):
    1. Generate EPUB structure (OPF, NCX files)
    2. Embed images
    3. Validate against EPUB standard
    4. Return: Download link
    ```
    
    **DOCX Export:**
    ```
    Use docx library (existing):
    1. Create Word document
    2. Apply formatting via Word styles
    3. Insert images
    4. Add TOC (auto-generated)
    5. Return: Download link
    ```
    
    **MOBI Export:**
    ```
    Use kindlegen or conversion service:
    1. Convert EPUB to MOBI
    2. Optimize for Kindle devices
    3. Return: Download link
    ```

#### 4.7 — Preview & Quality Check
- [ ] Endpoint: `GET /api/workflows/formatting/preview?format=pdf|epub|docx`
  - Returns: Rendered preview of first 5 chapters in chosen format
  - User can review before final export

#### 4.8 — Publishing Ready Handoff
- [ ] Endpoint: `POST /api/workflows/formatting/finalize`
  - Input: `{ bookId, formats: [...], publishingChecklist: { ... } }`
  - Process:
    ```
    1. Generate Publishing Checklist Validation:
       [
         { item: "Title page", completed: true },
         { item: "Table of contents", completed: true },
         { item: "Author bio", completed: true },
         { item: "No formatting errors", completed: true },
         { item: "All images embedded", completed: true },
         { item: "PDF passes KDP validation", completed: true }
       ]
    
    2. Export all formats
    3. Create Publishing Package:
       {
         "bookId": "...",
         "title": "...",
         "author": "...",
         "formats": {
           "pdf": { url, size, generatedAt },
           "epub": { url, size, generatedAt },
           "docx": { url, size, generatedAt }
         },
         "publicationDate": "...",
         "readyForPublishing": true,
         "publishingGuide": "Steps to publish on KDP, Apple Books, etc."
       }
    4. Create workflow_handoffs record (final)
    5. Update book.workflow_state → "PUBLISHED" (or "READY_FOR_PUBLICATION")
    6. Create workflow_snapshots record (final backup)
    ```
  - Return: "Book ready for publication. Download formats or publish directly."

#### 4.9 — Publishing Integration (Future Phase)
- [ ] Placeholder endpoints for:
  - `POST /api/workflows/formatting/publish-kdp` (Amazon KDP)
  - `POST /api/workflows/formatting/publish-apple-books`
  - `POST /api/workflows/formatting/publish-google-play`

### Testing for Phase 4
```bash
npm run test:workflows -- --phase formatting

tests/workflows/formatting/template-selection.test.ts
tests/workflows/formatting/format-generator.test.ts
tests/workflows/formatting/format-application.test.ts
tests/workflows/formatting/pdf-export.test.ts
tests/workflows/formatting/epub-export.test.ts
tests/workflows/formatting/docx-export.test.ts
tests/workflows/formatting/publishing-checklist.test.ts
tests/workflows/formatting/handoff.test.ts

# Test cases:
✓ Template selection returns matching templates
✓ Format auto-generator creates valid CSS
✓ Custom template builder validates CSS
✓ Format application wraps chapters correctly
✓ PDF export generates valid PDF
✓ EPUB export passes EPUB validation
✓ DOCX export opens in Word
✓ All formats export without errors
✓ Publishing checklist validates completeness
✓ Workflow state transitions to PUBLISHED/READY_FOR_PUBLICATION
```

### Backlog Capture for Phase 4
- CSS parsing errors (fallback applied)
- Image loading failures
- Format validation failures (flagged for manual review)

### Expected Output
- ✅ Formatting module fully functional
- ✅ All export formats working (PDF, EPUB, DOCX)
- ✅ Template library with 50+ pre-built templates
- ✅ Publishing checklist passing
- ✅ Final handoff ready

---

## PHASE 5: INTEGRATION & END-TO-END TESTING (Claude Code Session 10)

### Objective
Validate all 4 workflows together with proper sequencing, error recovery, and rollback.

### Tasks

#### 5.1 — End-to-End Workflow Test
- [ ] Create test suite: `tests/workflows/e2e.test.ts`
  - Input: Create test book
  - Run through full Planning → Writing → Editing → Formatting pipeline
  - Validate:
    - Handoffs succeed between workflows
    - State transitions correct
    - Final output files (PDF, EPUB, DOCX) valid
    - Audit log captures all events
    - No data loss

#### 5.2 — Error Recovery Test
- [ ] Simulate failures at each workflow stage:
  - Claude API timeout in Planning → verify backlog item + user can retry
  - Missing chapter in Writing → finalize blocked + list missing
  - High plagiarism in Editing → flag + user can rewrite + retry check
  - Template error in Formatting → fallback to default
  - Verify error handling doesn't break subsequent workflows

#### 5.3 — Rollback Test
- [ ] Test rollback at each phase boundary:
  - User approves Planning, starts Writing, changes mind
  - Rollback to PLANNING_COMPLETE
  - User re-does planning
  - Verify no data loss + audit log captures rollback

#### 5.4 — Feature Flag Rollout
- [ ] Test legacy vs. new workflow parallel operation:
  - Feature flag OFF: Old users continue with monolithic flow
  - Feature flag ON: New users use 4-workflow system
  - Verify no interference between flows

#### 5.5 — Performance & Scalability
- [ ] Baseline tests:
  - Planning for 10 books concurrently: <30s per book
  - Writing 25-chapter book: <15 min (batch mode)
  - Editing full manuscript: <10 min (all tools)
  - Formatting + export: <5 min (all formats)
  - Identify bottlenecks + optimize

### Testing for Phase 5
```bash
npm run test:workflows -- --phase integration

tests/workflows/e2e/full-pipeline.test.ts
tests/workflows/e2e/error-recovery.test.ts
tests/workflows/e2e/rollback.test.ts
tests/workflows/e2e/feature-flag.test.ts
tests/workflows/e2e/performance.test.ts

# Coverage target: 90%+ of workflow code paths
```

### Expected Output
- ✅ Full end-to-end pipeline validated
- ✅ Error recovery tested at all failure points
- ✅ Rollback functionality verified
- ✅ Performance baselines established
- ✅ Feature flag operational (gradual rollout ready)

---

## PHASE 6: DOCUMENTATION & DEPLOYMENT (Claude Code Session 11)

### Tasks

#### 6.1 — User Documentation
- [ ] Create guides:
  - Planning workflow: Step-by-step with screenshots
  - Writing workflow: Batch vs. chapter-by-chapter options
  - Editing workflow: AI tools comparison + best practices
  - Formatting workflow: Template selection + export guide
  - Error handling: How to resolve backlog items
  - Rollback: When and how to rollback

#### 6.2 — Developer Documentation
- [ ] API reference: All endpoints + schemas
- [ ] Architecture diagram: 4 workflows + handoff process
- [ ] Error codes + troubleshooting
- [ ] Database schema overview
- [ ] Deployment checklist

#### 6.3 — Deployment Plan
- [ ] Feature flag ON for 5% users (beta testing)
- [ ] Monitor errors + performance
- [ ] Ramp to 50%, then 100% (2-week rollout)
- [ ] Disable legacy monolithic flow once >95% on new system

### Expected Output
- ✅ Complete user + developer documentation
- ✅ Deployment runbook
- ✅ Rollout strategy defined

---

## MASTER BACKLOG TRACKING

### Backlog Dashboard
- [ ] Endpoint: `GET /api/backlog`
  - Filters: Book ID, workflow stage, severity, status (open/resolved)
  - Columns: Issue | Book | Workflow | Severity | Created | Status | Owner
  - Actions: View details, assign, resolve, re-run

### Backlog Item Resolution
- [ ] Each item includes:
  - Error context (what failed, where)
  - Suggested action (retry, rewrite, manual fix)
  - One-click "Retry" button
  - Manual resolution field

### Backlog Report (Weekly)
- [ ] Auto-generated report:
  - Total items created
  - Items resolved
  - Unresolved by severity/workflow
  - Trends (which workflows have most issues)

---

## RISK MITIGATION & GUARDRAILS

### Critical Guardrails

#### Sequence Enforcement
- No workflow can proceed without predecessor complete
- State machine enforces forward-only movement (no skipping)
- Rollback only to *previous complete* state

#### Data Integrity
- Audit log captures all changes (immutable)
- workflow_snapshots created at each phase boundary (full state backup)
- Database transactions ensure atomicity

#### Error Handling
- **CRITICAL errors:** Halt workflow, create backlog item, alert user
- **HIGH errors:** Retry 3x with exponential backoff; if still fails → backlog + HIGH severity
- **MEDIUM errors:** Log + flag for backlog, allow user to continue
- **LOW errors:** Log only, user notified in UI

#### User Blocking
- No workflow-blocking errors by default
- Users can mark backlog items as "accept risk + continue" (logged in audit)
- Dashboard shows real-time error count

---

## IMPLEMENTATION CHECKLIST FOR CLAUDE CODE

### Pre-Execution
- [ ] Read this entire instruction
- [ ] Ask for clarification if any section is unclear
- [ ] Confirm estimated 11-session breakdown aligns with actual scope

### Per-Session
- [ ] Start session with clear phase objective
- [ ] Create test file + expected outputs document
- [ ] Implement feature step-by-step (don't jump ahead)
- [ ] Run tests after each sub-module
- [ ] Document any deviations + ask user for approval
- [ ] End session with summary: What was built, what's next

### On Errors
- [ ] Show user the specific error (don't hide)
- [ ] Suggest 2–3 solutions
- [ ] Ask user which direction to pursue
- [ ] Update this instruction if approach changes

### On Completion
- [ ] Confirm all tests pass (90%+ coverage)
- [ ] Verify no breaking changes to existing users
- [ ] Create summary document with metrics:
  - Code added (lines)
  - Tests written (count)
  - Database changes (count)
  - New endpoints (count)
  - Performance baselines established

---

## SUCCESS CRITERIA

✅ **Phase 0:** Feature flags + state machine + error tracking operational  
✅ **Phase 1:** Planning workflow 100% complete, handoff valid  
✅ **Phase 2:** Writing workflow 100% complete, consumes Planning handoff  
✅ **Phase 3:** Editing workflow 100% complete, all tools functional  
✅ **Phase 4:** Formatting workflow 100% complete, all exports valid  
✅ **Phase 5:** End-to-end pipeline validates, error recovery tested  
✅ **Phase 6:** Documentation complete, rollout plan ready  
✅ **Backlog:** Dashboard functional, all issues tracked  
✅ **Tests:** 90%+ coverage across all workflows  
✅ **No Breaking Changes:** Existing users unaffected until feature flag flip  

---

## APPENDIX: SAMPLE WORKFLOW HANDOFF PAYLOADS

### Planning → Writing Handoff
```json
{
  "from": "PLANNING",
  "to": "WRITING",
  "bookId": "book-uuid-123",
  "payload": {
    "topic": "A haunted mansion in Victorian England",
    "genre": "Gothic Romance",
    "targetAudience": "25–45, female, loves historical fiction",
    "planningNotes": "Combine gothic atmosphere with slow-burn romance",
    "ideation": { "concepts": [...] },
    "marketResearch": { "bestsellers": [...], "pricing": "..." },
    "predictabilityScore": 78,
    "originalityStatus": "Original",
    "recommendedChapterCount": 28,
    "recommendedWordCount": 85000,
    "tone": "Atmospheric, mysterious, romantic",
    "pov": "Third-person limited (female lead)",
    "keyThemes": ["Secrets", "Redemption", "Love against odds"],
    "timestamp": "2025-05-21T10:00:00Z"
  },
  "status": "READY",
  "createdAt": "2025-05-21T10:00:00Z"
}
```

### Writing → Editing Handoff
```json
{
  "from": "WRITING",
  "to": "EDITING",
  "bookId": "book-uuid-123",
  "payload": {
    "title": "Shadows of Thornfield Manor",
    "totalWords": 84230,
    "chapterCount": 28,
    "completionDate": "2025-05-28T14:30:00Z",
    "chapters": [
      {
        "num": 1,
        "title": "The Arrival",
        "wordCount": 3200,
        "text": "...(full chapter text)...",
        "validationNotes": "Word count within range, character introductions clear"
      }
      // ... chapters 2–28
    ],
    "manuscriptQualityScore": 82,
    "issues": []
  },
  "status": "READY"
}
```

### Editing → Formatting Handoff
```json
{
  "from": "EDITING",
  "to": "FORMATTING",
  "bookId": "book-uuid-123",
  "payload": {
    "chapters": [
      {
        "num": 1,
        "title": "The Arrival",
        "text": "...(edited chapter text)...",
        "editingNotes": "Grammar enhanced, flow improved",
        "toolsApplied": ["grammar", "consistency", "humanizer"],
        "plagiarismScore": 1,
        "humanScore": 97,
        "continuityScore": 94
      }
      // ... chapters 2–28
    ],
    "overallEditingScore": 95,
    "editingSummary": "All chapters reviewed. Plagiarism <2%, AI detection <3%, continuity >90%."
  },
  "status": "READY"
}
```

### Formatting → Publishing Handoff
```json
{
  "from": "FORMATTING",
  "to": "PUBLISHING",
  "bookId": "book-uuid-123",
  "payload": {
    "title": "Shadows of Thornfield Manor",
    "author": "User Name",
    "formats": {
      "pdf": {
        "url": "s3://bookforge-exports/book-uuid-123/export.pdf",
        "size": "2.3 MB",
        "generatedAt": "2025-05-30T16:45:00Z"
      },
      "epub": {
        "url": "s3://bookforge-exports/book-uuid-123/export.epub",
        "size": "1.8 MB",
        "generatedAt": "2025-05-30T16:45:00Z"
      },
      "docx": {
        "url": "s3://bookforge-exports/book-uuid-123/export.docx",
        "size": "3.1 MB",
        "generatedAt": "2025-05-30T16:45:00Z"
      }
    },
    "publicationDate": "2025-06-15",
    "readyForPublishing": true,
    "publishingGuide": "Next steps: 1) Login to KDP, 2) Click 'Create new title', 3) Upload EPUB file..."
  },
  "status": "READY"
}
```

---

## QUESTIONS FOR CLARIFICATION (Before Claude Code Starts)

1. **Predictability Scorer Algorithm:** Is the weighted scoring model sufficient, or do you want a more sophisticated ML-based model?
2. **Writing Tool:** Should Claude Code use `claude-opus-4-6` for chapter writing (slower, higher quality) or `claude-sonnet-4` (faster)?
3. **Plagiarism/AI Detection:** Do you have existing integrations (Turnitin, OpenAI classifier) or should I build mockups?
4. **Template Library:** Should I seed 50 pre-built templates or create a sample library of 5–10 and document how to add more?
5. **Publishing Integration:** Should Phase 4 include actual KDP/Apple Books API integration or just a placeholder for Phase 2?
6. **Concurrent Workflow Requests:** Can users run multiple books through different workflows simultaneously (e.g., Book A in Writing, Book B in Formatting)?
7. **Rollback Scope:** If a user rolls back from Writing to Planning, should all Writing chapters be deleted or archived?

---

**Ready for Claude Code execution. Confirm start with Phase 0 or ask for clarifications above.**
