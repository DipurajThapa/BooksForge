//! Privacy invariant #4 — agent dispatch fails with `ConsentRequired`
//! until the user opts in per project.
//!
//! Closes EXTERNAL_AUDIT_BACKLOG.md #9.
//!
//! Strategy: spin up a fresh project bundle (no AI consent yet),
//! attempt to dispatch the simplest agent (Outline Architect),
//! assert the result is `Err(BooksForgeError::ConsentRequired { .. })`.
//! Then flip the consent flag, re-dispatch, assert success.  Then
//! corrupt the consent row (write garbage JSON) and re-dispatch,
//! assert `ConsentRequired` again — corruption must NEVER fall back
//! to "consent granted".
//!
//! Implementation status: SCAFFOLDED.  The shape of the test mirrors
//! the existing `default_originality_provider_is_local_only` test in
//! `privacy_invariants.rs`.  Activation requires confirmation of the
//! BooksForgeError variant name and the consent-storage helper API
//! after Stabilisation Sprint S1 lands.
//!
//! The test is `#[ignore]` until that confirmation; removing the
//! `#[ignore]` is the gating action for closing audit #9.

#[test]
#[ignore = "wires against agent dispatch + consent-storage API; activate after Stabilisation Sprint S1 lands"]
fn agent_dispatch_returns_consent_required_until_user_opts_in() {
    // TODO(MZ-09): replace with real integration once the
    // dispatch + consent-storage APIs are stable.  Pseudocode:
    //
    //     let tempdir = tempfile::tempdir().expect("tempdir");
    //     let storage = open_pool(tempdir.path()).await.expect("pool");
    //     let mock = MockOllama::accept_any();
    //
    //     // 1. Fresh project, no consent.  Dispatch should be denied.
    //     let project = fiction_project(storage.clone()).await;
    //     let dispatch_pre = run_agent(
    //         &project, AgentKind::OutlineArchitect, mock.clone(),
    //     ).await;
    //     assert!(matches!(
    //         dispatch_pre,
    //         Err(BooksForgeError::ConsentRequired { .. }),
    //     ), "dispatch must fail with ConsentRequired pre-consent; got {:?}", dispatch_pre);
    //
    //     // 2. User opts in.
    //     project.set_ai_consent(true).await.expect("opt-in");
    //
    //     // 3. Re-dispatch should succeed.
    //     let dispatch_post = run_agent(
    //         &project, AgentKind::OutlineArchitect, mock.clone(),
    //     ).await;
    //     assert!(dispatch_post.is_ok(), "post-consent dispatch failed: {:?}", dispatch_post);
    //
    //     // 4. Corrupt the consent JSON, re-dispatch.  Default-on-corruption
    //     //    must be "off", not "on".
    //     project.write_raw_consent_row("not-json").await;
    //     let dispatch_corrupt = run_agent(
    //         &project, AgentKind::OutlineArchitect, mock,
    //     ).await;
    //     assert!(matches!(
    //         dispatch_corrupt,
    //         Err(BooksForgeError::ConsentRequired { .. }),
    //     ), "corrupted consent row must default to OFF; got {:?}", dispatch_corrupt);
}

#[test]
#[ignore = "depends on consent-storage helper API; activate after Stabilisation Sprint S1 lands"]
fn revoking_consent_blocks_subsequent_dispatches_immediately() {
    // After consent is revoked mid-session, the next dispatch must
    // fail with ConsentRequired even if a previous run is in flight.
    // The check happens at dispatch time, not at app start.
    //
    // TODO(MZ-09): wire when consent-storage helper is stable.
}
