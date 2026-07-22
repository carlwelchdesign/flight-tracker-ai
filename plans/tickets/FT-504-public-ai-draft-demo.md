# FT-504 — Expose the bounded AI drafting demonstration

Status: In progress

Branch: `feat/ft-504-public-ai-draft-demo`
Final implementation commit: `063f2df`
Pull request: [#64](https://github.com/carlwelchdesign/flight-tracker-ai/pull/64)
Owner: Backend, AI product, and frontend engineering

Make the existing FT-503 OpenAI integration visible in the public portfolio
without creating a general prompt surface or weakening the human-review and
non-operational boundaries.

Dependencies: FT-502, FT-503

## Acceptance checklist

- [x] One fixed, project-authored synthetic recommendation case is the only
      evidence eligible for the public demonstration.
- [x] Rust performs evidence minimization, OpenAI Responses API generation,
      validation, deterministic fallback, and response shaping.
- [x] The endpoint accepts no prompt, aircraft, provider, route, or tenant input
      and does not persist model or provider data.
- [x] A process-level cache prevents repeated public requests from repeatedly
      spending model tokens for the same fixed demonstration.
- [x] The public UI visibly separates deterministic facts, AI-generated wording,
      model/fallback status, and the mandatory human-review state.
- [x] The UI does not expose approval, send, route-selection, or operational
      action controls.
- [x] Missing, refused, invalid, timed-out, or rate-limited model responses
      degrade to the validated deterministic template.
- [x] Focused Rust and web tests, lint, typecheck, production build, runtime
      smoke, and diff hygiene pass.
- [ ] Branch, final commit, pull request, required checks, and hosted evidence
      are recorded before completion.

## Non-goals

- Arbitrary user prompts or live-aircraft summaries.
- Sending ADSB.lol, NOAA, tenant, or protected operations data to an LLM.
- Letting model output select or modify a route, approve itself, or trigger a
  message or operational action.
- Treating the synthetic offline recommendation as flight-planning guidance.

## Verification evidence

- `apps/api/src/public_ai_draft.rs` evaluates only `held-multi-01` from the
  versioned FT-502 fixture, converts it through the existing FT-503 minimizer,
  and caches one `DraftPackage` in a process-level `OnceCell`. The Axum handler
  has no request body, query, path, authentication, database, provider, tenant,
  approval, send, or operational-action dependency.
- The public React panel makes no request until the visitor explicitly selects
  `Generate AI draft`. It separates deterministic source facts from generated
  wording, identifies OpenAI versus deterministic fallback, keeps the state at
  `awaiting_review`, and exposes no approve or send control.
- All 133 Rust library tests, 13 binary tests, integration tests, strict Clippy,
  and formatting pass. All 152 web tests across 47 files, ESLint, TypeScript,
  and the Next.js production build pass under Node.js 24.18.0.
- A local live Responses API request reached OpenAI and returned the concrete
  `insufficient_quota` code. The application maps it to `quota_exhausted`,
  returned its validated deterministic template, preserved `awaiting_review`,
  and kept `automatic_send_available: false`. No credential or provider body
  appeared in application output.
- A 1440-pixel browser check exercised the model-success presentation with the
  fixed route contract, found the AI and review status visible, zero approve or
  send controls, zero framework overlays, and no horizontal overflow. The
  unrelated live data routes were unavailable in that isolated local browser
  run and the tracker displayed its existing replay/degraded states.
