//! The admin response seam `admin_response_body` accepts only an `AuditedResponse` — a `PiiDisclosure`
//! (which an audit minted) or an allowlisted PII-free type. A hand-rolled PII body, even though it is
//! `Serialize`, is not `AuditedResponse`, so the future Worker cannot send it through the seam without
//! an audited disclosure. This is the I5 gate at the response boundary.
use boundless_server_core::admin_response_body;

#[derive(serde::Serialize)]
struct ForgedMemberBody {
    name: String,
    phone: String,
    address: String,
}

fn main() {
    let body = ForgedMemberBody {
        name: "Maria".to_string(),
        phone: "+15551230001".to_string(),
        address: "12 Olive St".to_string(),
    };
    // ERROR: `ForgedMemberBody: AuditedResponse` is not satisfied.
    let _ = admin_response_body(&body);
}
