//! `boundless-sync` — WebSocket message types and sync state machines.
//!
//! The WS open-handshake type carries `client_min_version` + `client_recommended_version`
//! (spec 001 AC7). Message types are compiled from `api/boundless.proto` (the proto
//! contract source of truth); this crate consumes the generated types.
//!
//! Scaffolded by spec 001 task **T01**; proto-derived types are wired at the contract
//! freeze in **T10**.
