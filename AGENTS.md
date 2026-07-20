# Flight Tracker Engineering Instructions

These instructions apply to the entire repository.

## Source of truth

Before implementation, read `plans/README.md`, `plans/STATUS.md`, the active milestone ticket file, and relevant decisions and risks. Work on one ticket at a time unless the user explicitly authorizes a different scope.

## Required engineering skill

Use the global `solid-development-engineer` skill for implementation, refactoring, debugging, and code review. Apply SOLID pragmatically: preserve clear responsibilities and dependency direction without creating speculative layers, traits, interfaces, or services.

## Architecture

- Rust owns APIs, ingestion, normalization, replay, alert policy, and operational workflows.
- Keep the Rust backend a modular monolith until measured constraints justify a split.
- Next.js/TypeScript owns the dispatcher interface.
- PostgreSQL/PostGIS is the operational system of record.
- Keep provider payloads, domain facts, API contracts, and UI models separate.
- Deterministic code controls operational eligibility and severity. AI may explain or draft, but may not make authoritative safety decisions.
- Preserve human approval for messages and recommendations.

## Ticket delivery workflow

- Never implement directly on `main`.
- Use one dedicated branch per ticket following `plans/GIT_WORKFLOW.md`.
- Keep commits limited to that ticket and reference its ID.
- Open one PR per ticket unless the user explicitly approves a combined delivery.
- Do not mark a ticket complete until acceptance criteria pass and its branch, commit, and PR evidence are recorded in the ticket file and `plans/STATUS.md`.
- If no Git remote or PR authorization exists, report the exact blocker and leave the ticket incomplete.

## Implementation standard

- Inspect existing code and tests before designing new abstractions.
- Keep domain policy independent of web frameworks, data providers, and persistence details.
- Make time, units, freshness, source attribution, tenant boundaries, and errors explicit.
- Add focused tests with behavior, including relevant failure and boundary cases.
- Verify formatting, linting, type checking, tests, build, runtime behavior, migrations, and diff hygiene in proportion to the change.
- Preserve unrelated user changes and never use destructive Git commands without explicit approval.

## Planning updates

At ticket start, update status and branch fields. At completion, check only verified acceptance boxes, add commands or artifacts as evidence, fill commit and PR fields, update milestone counts, and leave a concise handoff note.
