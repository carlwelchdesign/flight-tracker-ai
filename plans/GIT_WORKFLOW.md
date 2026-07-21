# Ticket Branch, Commit, and Pull Request Workflow

Every implementation ticket is delivered through its own feature branch, intentional commits, and pull request. Planning-only changes made before repository setup are exempt until FT-001 establishes Git and a remote.

## Prerequisite

FT-001 must establish:

- A Git repository with `main` as the default branch
- A GitHub remote named `origin`
- An initial baseline commit containing the approved planning package
- Required checks or documented merge protections
- PR tooling/authentication that can open and inspect pull requests

## Start a ticket

1. Confirm dependencies and acceptance criteria.
2. Update the ticket status to `In progress` and add the owner.
3. Start from an up-to-date `main` with a clean worktree.
4. Create exactly one ticket branch:
   - Feature: `feat/ft-123-short-description`
   - Fix: `fix/ft-123-short-description`
   - Documentation or planning: `docs/ft-123-short-description`
   - Maintenance: `chore/ft-123-short-description`
5. Record the exact branch in the ticket and `STATUS.md`.

## Commit work

- Make reviewable commits that contain only ticket scope.
- Use Conventional Commit style with the lowercase ticket ID as scope, for example: `feat(ft-123): stream normalized flight events`.
- Commit signing is not required for this repository. Do not pause delivery for GPG, SSH-signing, hardware-key, or pinentry availability.
- Run the relevant verification before the final commit.
- Record the final commit SHA in the ticket.
- Do not mix opportunistic refactors into the ticket; create another ticket when needed.

## Open the PR

- Push the dedicated branch and open one PR targeting `main`.
- Title format: `[FT-123] Concise outcome`.
- PR body must include:
  - Why the change is needed
  - What changed
  - Acceptance-criteria mapping
  - Verification commands and results
  - Screenshots or runtime evidence when user-visible
  - Migration, rollout, rollback, data, security, and observability notes where applicable
  - Known risks or follow-up tickets
- Prefer a draft PR while substantial work remains; mark ready only after local verification.
- Address review comments on the same ticket branch.
- Record the PR URL in the ticket and `STATUS.md`.

## Complete the ticket

A ticket is complete only when:

- [ ] All ticket acceptance criteria are checked with evidence.
- [ ] Dedicated branch is recorded.
- [ ] Intended commits are pushed and final commit SHA is recorded.
- [ ] PR is open, linked, and passes required checks.
- [ ] Review feedback is resolved or explicitly deferred to a linked ticket.
- [ ] Ticket and `STATUS.md` reflect the final state.

Merge approval remains a human-controlled action unless the user explicitly authorizes merging.
