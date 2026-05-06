# Phase 14 — Voice + advanced AI

## Goal

Add dictation (Whisper.cpp), TTS playback (XTTS or platform TTS), and long-context AI features (whole-novel critique using long-context cloud models). Voice cloning is opt-in and on-device only.

## Pre-conditions

V1.5 GA shipped.

## Inputs

1. `../_deep/08-ai-integration.md` — section 15 (V2.0 features).

## Deliverables

### 1. Dictation

Whisper.cpp integration for offline speech-to-text. Push-to-talk and continuous modes. Punctuation inference. Per-project vocabulary (entity names) boosts recognition.

### 2. TTS playback

Platform TTS (macOS: AVSpeechSynthesizer; Windows: SAPI; Linux: speech-dispatcher) for free-tier playback. XTTS or similar for higher-quality offline voices (Studio-tier).

### 3. Voice cloning (opt-in, on-device)

User records ≥ 10 minutes of voice; XTTS or coqui-tts trains a voice profile **on-device**. Profile stays on-device; never uploaded.

### 4. Long-context features

Cloud-mode-only feature: whole-novel critique with long-context models. Multi-pass orchestration with explicit token budgeting. Cost estimate prominently shown.

### 5. Tests

- Dictation accuracy benchmark on a fixture audio set.
- TTS playback correctness across OSes.
- Voice cloning training stays on-device (network capture confirms).
- Long-context critique works end-to-end on a 100k-word fixture with mock provider.

## Guard-rails

**[GUARD-P14-1]** Voice cloning never touches the network. CI test with network blocked.

**[GUARD-P14-2]** Long-context features show a cost estimate and require explicit confirmation.

**[GUARD-P14-3]** Audio data is never uploaded to any service except the user-selected TTS/cloud-LLM provider, and only if they're using that feature.

## When you finish

PR title `Phase 14: Voice + advanced AI`.
