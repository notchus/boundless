import Foundation

/// Supplies the Driver's one-time **Recovery Code** for the capture screen (ADR-0016 D3 / AC19).
///
/// The Recovery Code is minted server-side and returned on the successful `/api/auth/bind-device`
/// response (`fresh_recovery_code`); the device captures it **once**, at onboarding, so the Driver can
/// self-serve a device replacement later (phone + Recovery Code → re-bind, old token invalidated,
/// fresh code issued). The real implementation that reads it off the bind response is the deferred
/// imperative shell (the deployable Worker does not exist yet, T07-shell-B); the flow depends only on
/// this protocol so it is testable with a stub.
///
/// **Privacy:** the code is a secret shown on the Driver's own device (like a 2FA backup code). It is
/// rendered on screen by design but **must never be logged** (P2) — this module performs no logging.
/// `nil` until a successful bind populates it; the router then skips the capture rather than render an
/// empty value (it never blocks onboarding).
@MainActor
public protocol RecoveryCodeProviding {
    /// The captured Recovery Code, or `nil` if none is available yet.
    var recoveryCode: String? { get }
}
