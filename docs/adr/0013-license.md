# ADR-0013: License — AGPL-3.0 for the Entire Repository

- **Status:** Accepted
- **Date:** 2026-06-04
- **Author:** Boundless founder
- **Deciders:** Boundless founder (sole copyright holder)

## Context

Boundless is, per constitution **P11** ("Free, open, donation-supported"), a free
and open-source product whose source is public and which will never carry paywalled
features. That principle commits us to a license, but not to a *specific* one — and
the choice has real teeth in this domain.

Two facts shape the decision:

1. **Boundless is a networked app.** The intelligent parts (matching, the privacy
   boundaries, chain construction) run on a server tier (Cloudflare Workers +
   Durable Objects, see ADR-0003). A copyleft license whose obligations trigger only
   on *distribution* of binaries would let someone run a modified, closed backend as
   a hosted service without ever publishing their changes — the classic "SaaS
   loophole." For a privacy-first product whose trustworthiness rests on its source
   being inspectable, that loophole is precisely the thing we cannot leave open.

2. **We also ship native client apps through the Apple App Store** (and Google Play).
   Strong copyleft (GPL/AGPL) is in tension with Apple's standard EULA, which imposes
   a non-transferable, device-limited license on apps — terms GPL/AGPL forbid a
   distributor from adding. This is a well-known conflict that has historically caused
   GPL apps to be pulled from the App Store.

So we need a license that (a) closes the network loophole and (b) has a workable path
onto the App Store. As the sole copyright holder, the author retains the right to
grant additional permissions on top of the chosen license, which makes (b) solvable.

## Decision

License the **entire repository under AGPL-3.0** (GNU Affero General Public License,
version 3).

AGPL is GPL-3.0 plus its **§13** network-use clause: anyone who runs a modified
version of Boundless to provide a service over a network must offer that service's
users the corresponding source. This closes exactly the SaaS/network loophole that
plain GPL leaves open, which matters because a private modified Boundless *backend*
is the most likely place changes would otherwise stay hidden.

To resolve the App Store conflict, the author — as sole copyright holder — will grant
an **App Store additional-permission exception under GPL/AGPL §7**, following Signal's
model: an explicit additional permission allowing distribution through app stores whose
terms would otherwise conflict with the license. This exception will be added as a
`LICENSE-EXCEPTION` file **before the first iOS build** (tracked in `DEFERRED.md` under
"Licensing").

## Considered alternatives

### Option A (chosen) — AGPL-3.0 everywhere + an App Store §7 additional-permission exception

**Pros:**
- One license across the whole repository — server, clients, core, tooling. Simplest
  mental model and the simplest contribution story.
- AGPL §13 closes the network loophole: a hosted, modified Boundless backend must
  publish its source. Directly protects the privacy promise.
- The §7 additional permission (Signal's model) makes App Store distribution workable
  without weakening the copyleft for everyone else.
- Strongly aligned with P11 (free, open) — derivatives stay free and open, including
  networked ones.

**Cons:**
- AGPL deters some commercial adopters and is disallowed by some corporate policies.
  For Boundless (a donation-supported community project, not a vendor courting
  enterprise integrators) this is acceptable, even desirable.
- The §7 exception must be drafted carefully and is a prerequisite for the first iOS
  build (a scheduling dependency, not a blocker).

### Option B — AGPL-3.0 server + Apache-2.0 clients

Split the repo: copyleft on the server tier (where the network loophole lives), permissive
(Apache-2.0) on the native clients (smoothing App Store distribution and client reuse).

**Pros:**
- Apache-2.0 clients sidestep the Apple-EULA conflict without needing a §7 exception.
- Permissive clients are easier for third parties to embed or fork.

**Cons:**
- **Two licenses to manage.** Per-directory license boundaries, per-file headers, and
  a contribution process that has to route changes to the right regime — ongoing
  overhead for a volunteer-run project.
- The client/server boundary is not clean: the shared Rust core (ADR-0001) compiles
  into *both* the clients and the Workers edge. A split license would have to cut
  through the single most-shared module in the codebase.
- Permissive clients permit closed forks of the rider/driver apps — weaker alignment
  with P11 than keeping everything copyleft.

**Rejected for now** — the two-licenses-to-manage cost outweighs the benefit, especially
since the §7 exception in Option A already solves the App Store problem with a single
license.

### Option C — Plain GPL-3.0

Strong copyleft, but without AGPL's §13 network clause.

**Pros:**
- Familiar, widely understood, broadly compatible within the GPL ecosystem.
- Still copyleft on distributed binaries.

**Cons:**
- **Its copyleft triggers only on distribution, leaving a SaaS/network loophole.**
  Boundless is a networked app, so a private *modified backend* could be run as a
  hosted service and stay closed — never distributed, never disclosed.
- AGPL **§13** closes exactly this gap; plain GPL does not.

**Rejected** — the network loophole is disqualifying for a product whose trust model
depends on the running service's source being inspectable.

## Consequences

### Positive

- **The network loophole is closed.** Anyone offering a modified Boundless as a network
  service owes its users the source (AGPL §13). Reinforces P11 and the
  "Unbreachable by Design" / inspectable-source posture.
- **One license across the repo** — the simplest possible licensing surface to reason
  about and to document for contributors.
- **App Store distribution stays open to us** via the planned §7 exception, without
  diluting copyleft for non-app-store use.

### Negative / costs

- **Contributor licensing management.** To preserve the author's ability to manage
  licensing later (e.g. to grant or revise the App Store exception, or relicense if
  ever necessary), outside contributors will need to sign a **DCO or a lightweight CLA**.
  Tracked in `DEFERRED.md`; the trigger is the first external pull request.
- **`LICENSE-EXCEPTION` is a release prerequisite.** The §7 App Store additional
  permission must exist before the first iOS build. Tracked in `DEFERRED.md`.
- **AGPL narrows the adopter pool.** Some organizations forbid AGPL dependencies.
  Accepted as consistent with the project's nature.

### Neutral / follow-ups

- Add the actual `LICENSE` file (AGPL-3.0 text) and, when due, `LICENSE-EXCEPTION`
  (the §7 App Store additional permission) at the repo root.
- Author the DCO/CLA process before the first external PR.
- If a clean client/server license split ever becomes worthwhile (e.g. a strong reason
  to permissively license a client SDK), it would supersede this ADR, not amend it.

## Compliance

- **Constitution change:** None. This decision *implements* P11 ("Free, open,
  donation-supported"); it does not alter the constitution.
- **Stack matrix:** No change. (License choice, not a dependency.)
- **Migration plan:** N/A — greenfield. No existing code is relicensed.
- **Follow-up obligations (tracked in `DEFERRED.md`, "Licensing"):**
  - `LICENSE-EXCEPTION` (AGPLv3 §7 App Store additional permission) — before the first
    iOS build.
  - DCO or lightweight CLA — before the first external pull request.

## References

- [GNU AGPL-3.0](https://www.gnu.org/licenses/agpl-3.0.html) — see §13 (Remote Network Interaction)
- [GNU GPL-3.0](https://www.gnu.org/licenses/gpl-3.0.html) — see §7 (Additional Terms)
- [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0) — the alternative client license weighed in Option B
- Precedent cited by the author: Signal's use of AGPL-3.0 with an App Store additional permission ("Signal's model")
- Constitution **P11** (Free, open, donation-supported)
- `DEFERRED.md` → "Licensing" (LICENSE-EXCEPTION, DCO/CLA, this ADR)
