// Relying-Party configuration for admin WebAuthn (ADR-0017). Injected, never ambient:
// production wires the real admin-web origin; tests wire `localhost`. Keeping this out of
// the module body is the web analogue of the Rust core's injected `Clock`/`SecretSource`
// (no ambient environment leaks into the verification logic).

export interface RpConfig {
  /** Human-readable RP name shown by the platform passkey UI. Not a Boundless catalog string. */
  readonly rpName: string;
  /** RP ID — a registrable domain suffix of `origin`'s host (e.g. `boundless.example`). */
  readonly rpID: string;
  /** Expected ceremony origin, scheme + host (+ port), e.g. `https://admin.boundless.example`. */
  readonly origin: string;
}
