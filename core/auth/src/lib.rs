//! `boundless-auth` — device-side authentication & onboarding logic (ADR-0016, spec 001).
//!
//! Home of the `OnboardingState` machine, Onboarding/Recovery-code request shaping and
//! result interpretation, client-version comparison (O4/O1), indefinite sessions with
//! silent refresh-token rotation + replay/lineage detection (ADR-0016 D2), and the
//! device-token binding tuple `(member_id, platform, app_version)` (I4).
//!
//! Pure logic with an **injected `Clock`/RNG** — never `SystemTime::now` or ambient
//! randomness (`docs/forbidden-patterns.md`). Generated to Swift/Kotlin so every
//! platform transitions identically.
//!
//! Scaffolded by spec 001 task **T01**; the state machine + code logic land in **T04**,
//! sessions/refresh/device-token binding in **T05**.
