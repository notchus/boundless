package app.boundless.driver.onboarding

/**
 * Supplies the Driver's one-time **Recovery Code** for the AC19 capture screen (ADR-0016 D3). The code
 * is minted on the `/api/auth/bind-device` response (`fresh_recovery_code`); captured once at
 * onboarding, it lets the Driver self-serve a future device replacement (phone + Recovery Code →
 * re-bind). Riders have no equivalent — they recover via an Admin.
 *
 * The real implementation (reading the bind response) is the deferred app shell (T14-shell); the flow
 * depends only on this interface so it stays deterministic in tests. The Android twin of
 * `DriverShared.RecoveryCodeProviding`.
 *
 * Privacy: the code is a secret shown on the Driver's own device (like a 2FA backup code). It is
 * rendered by design but **must never be logged** (P2) — this module performs no logging.
 */
interface RecoveryCodeProviding {
    /** The captured Recovery Code, or `null` if none is available yet (the router then skips the
     *  capture — never an empty-code screen, never a block). */
    val recoveryCode: String?
}
