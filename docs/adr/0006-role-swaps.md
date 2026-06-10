# ADR-0006: Role swaps allowed — a Member's role is a set, not a single value

- **Status:** Accepted
- **Date:** 2026-06-10
- **Author:** notch
- **Deciders:** notch
- **Relates to:** P4 (Rust core source of truth); the glossary (`Role`, "Driver may also be a Rider in another context"); spec 001 (auth/onboarding); spec 008 (admin member-management / issuance)
- **Was:** an index stub ("ADR-0006 — Role swaps allowed") in `docs/adr/README.md`, formalized here because spec 008 is the first surface that *sets* a member's roles.

## Context

Boundless deliberately blurs the Rider/Driver line: the glossary states a **Driver may
also be a Rider in another context (role swaps supported)**, and Sarah's persona lists
"performs role swaps" as a core admin activity (e.g. "Margaret can drive this Sunday, she
has a guest car"). The personas Margaret (mostly Rider, occasionally Driver) and Tobias
(Driver) are concrete instances.

The data model already anticipates this: `server/migrations/0002_members.up.sql` defines
`members.roles` as a **`member_role[]` array**, not a scalar `member_role`. So a member
*can* hold `{Rider}`, `{Driver}`, or `{Rider, Driver}`. But there was **no decision
record** explaining why, and the glossary referenced a non-existent `ADR-006-role-swaps`
(3-digit, off-convention). Spec 008 is where a member's role set is first *established*
(at issuance), so the policy must be pinned now, or the issuance form and the matching
eligibility logic would each guess independently.

This ADR records **the policy** (role swaps are allowed; `roles` is a set). It does **not**
build the role-swap *workflow* (an Admin re-granting roles per Gathering, or a member
flipping context) — that is a sibling spec. Spec 008 only sets the initial `roles[]` and
allows editing it.

## Decision

**A Member's role is a set (`roles member_role[]`), and role swaps are allowed.** A single
person (one `(Group, Member)` identity) may hold any non-empty subset of `{Rider, Driver}`
— and `Admin` orthogonally, but Admin is Developer-issued (I11) and is not granted on this
path.

- **Issuance (spec 008)** sets the initial `roles[]` (≥1 role) and may edit it.
- **The role-swap *workflow*** — per-Gathering role activation ("Margaret drives this
  Sunday"), or any rules about *when* a multi-role member acts as which role — is
  **deferred to a sibling spec**. Matching (spec 004+) decides eligibility from the role
  set; this ADR only guarantees the set exists and is multi-valued.
- **Identity is not duplicated per role.** A person who is both a Rider and a Driver is
  **one** member row with `roles = {Rider, Driver}`, **one** `MemberId`, **one** auth
  identity/device binding (spec 001) — not two accounts. This is what makes a "swap" a
  view-level concern, not a re-onboarding.

## Considered alternatives

### Option B — a single scalar role per member (`role member_role`)

**Rejected.** It contradicts the glossary ("Driver may also be a Rider in another
context") and Sarah's persona ("performs role swaps"), and it would force a person who
both drives and rides to hold **two** accounts — two onboardings, two device bindings, two
identities to keep in sync — which fractures the auth model (spec 001) and the audit trail.
The `member_role[]` column already in the schema would have to be narrowed, losing the
half-built capability for no benefit.

### Option C — separate Rider and Driver *entities* linked by a person id

**Rejected.** A "person" super-entity with linked role-specific records is more machinery
than a closed group of dozens–hundreds of members needs, and it complicates RLS (every
query would join person→role-record) and the I12 forget path (forgetting a person must
sweep multiple tables). A single member row with a role *set* is the minimal model that
satisfies the requirement.

## Consequences

### Positive

- One identity per person — auth, device binding, audit, and deletion all key on a single
  `MemberId` regardless of how many roles the person holds.
- The issuance form is a multi-select, not a single choice; no schema change needed
  (`roles[]` exists).
- Matching can read eligibility straight off the set.

### Negative / costs

- **Queries and RLS must treat `roles` as an array** (`'Driver' = ANY(roles)`, not
  `role = 'Driver'`). Slightly more care in every role-filtered query; the matching spec
  inherits this.
- **"Acts as which role when"** is left unspecified by this ADR — a multi-role member's
  per-Gathering behavior is a real design question the sibling role-swap spec must answer.
  Recorded honestly rather than hand-waved.

### Neutral / follow-ups

- `docs/domain-glossary.md`: the dangling `ADR-006-role-swaps` reference is corrected to
  `ADR-0006`.
- `docs/adr/README.md`: ADR-0006 moves from the "suggested stubs" list to the active list.
- The **role-swap workflow** is tracked as a future spec (not in 008's task list); 008
  delivers only the role *set* at issuance.

## References

- `docs/domain-glossary.md` (`Role`, `Driver`, `Rider`) · `docs/personas.md` (Sarah,
  Margaret, Tobias)
- `server/migrations/0002_members.up.sql` (`roles member_role[]`, the multi-role column)
- spec 008 (`specs/008-admin-member-management/spec.md`, AC13) · spec 001 (single auth
  identity per member)
