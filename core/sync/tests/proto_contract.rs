//! AC7 (proto leg) — the contract-freeze gate (spec 001 T10).
//!
//! Asserts that the frozen `api/boundless.proto` declares the WebSocket open-handshake
//! acknowledgement carrying `client_min_version` (+ `client_recommended_version`) — the WS
//! mirror of the HTTP `/api/auth/*` version handshake (AC7 / O4 / O5). `core/sync` is the
//! documented consumer of the proto-derived types, so the contract check lives here.
//!
//! Dependency-free + std-only: it `include_str!`s the contract at compile time and does a small
//! structural scan (strip comments → brace-match the message block → check the field decls),
//! mirroring the `server/tests/migrations.rs` static-convention-test idiom (T06). It does NOT
//! pull a proto parser — the assertion is narrow and the file is ours.

/// The frozen proto contract, embedded at compile time (path is relative to this crate's root:
/// `core/sync/../../api` → repo-root `api/`).
const PROTO: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../api/boundless.proto"
));

/// Strip `// …` line comments and `/* … */` block comments in a SINGLE pass, so neither kind can
/// be misread inside the other — e.g. a `/*` that appears inside a `// …` line comment (such as
/// the `/api/auth/*` in this file's own header) must NOT open a block comment. Operates on byte
/// indices but only ever pushes `&str` slices (UTF-8 safe — `/`, `*`, `\n` are single-byte ASCII
/// and never appear as UTF-8 continuation bytes). Newlines are preserved so line structure holds.
fn strip_comments(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let mut run_start = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            out.push_str(&src[run_start..i]);
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            run_start = i; // resume at (and keep) the newline
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            out.push_str(&src[run_start..i]);
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(b.len()); // consume the closing */ (or run off the end harmlessly)
            run_start = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&src[run_start..]);
    out
}

/// Return the brace-matched body of `message <name> { … }`, or `None` if absent. Guards against a
/// prefix match (`message FooBar` must not satisfy a search for `message Foo`).
///
/// Limitation (acceptable for this freeze gate over a file we own): the brace matcher is not
/// string-literal-aware, so a `{` or `}` inside a quoted proto option/default value would mis-count.
/// Our frozen handshake proto has no such literals; if the contract later grows brace-bearing
/// string options, this must become string-aware (or the assertion narrowed).
fn message_block<'a>(src: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("message {name}");
    let mut from = 0;
    while let Some(rel) = src[from..].find(&needle) {
        let start = from + rel;
        let after = &src[start + needle.len()..];
        // The next char must end the identifier (whitespace or the opening brace) — not extend it.
        if after.starts_with(|c: char| c.is_whitespace() || c == '{') {
            if let Some(open) = after.find('{') {
                let bytes = after.as_bytes();
                let mut depth = 0i32;
                let mut i = open;
                while i < bytes.len() {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                return Some(&after[open + 1..i]);
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                return None; // unbalanced braces
            }
        }
        from = start + needle.len();
    }
    None
}

/// The portion of a message body at brace-depth 0 — i.e. with any NESTED `{ … }` blocks (a child
/// `message`/`enum`/`oneof`) removed. So a field declared inside a nested message is not mistaken
/// for a direct field of the enclosing one.
fn top_level_only(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut depth = 0i32;
    for c in body.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// Whether a message body declares a field named `name` with a numeric tag **as a direct field** —
/// i.e. a depth-0 statement of the proto3 form `[repeated] <type> <name> = <tag>;`. Treats `=` and
/// whitespace as delimiters (so `name = 1` and `name=1` both parse), and ignores fields nested in a
/// child message (so a `client_min_version` inside some other message can't satisfy the assertion).
fn declares_field(body: &str, name: &str) -> bool {
    top_level_only(body).split(';').any(|stmt| {
        let toks: Vec<&str> = stmt
            .split(|c: char| c.is_whitespace() || c == '=')
            .filter(|t| !t.is_empty())
            .collect();
        toks.iter().position(|t| *t == name).is_some_and(|i| {
            // a preceding type token, and a following all-digit field tag
            i >= 1
                && toks
                    .get(i + 1)
                    .is_some_and(|tag| tag.chars().all(|c| c.is_ascii_digit()))
        })
    })
}

#[test]
fn ac7_ws_handshake_has_client_min_version() {
    let proto = strip_comments(PROTO);
    let body = message_block(&proto, "ServerOpenHandshake")
        .expect("api/boundless.proto must declare `message ServerOpenHandshake` (the WS open-handshake ack)");

    assert!(
        declares_field(body, "client_min_version"),
        "ServerOpenHandshake must declare `client_min_version` (AC7 / O4)"
    );
    assert!(
        declares_field(body, "client_recommended_version"),
        "ServerOpenHandshake must declare `client_recommended_version` (AC7 / O5)"
    );
}

/// The handshake is a pair — the client→server frame must carry the reported version so the
/// server can compute the requirement. Keeps the frozen contract coherent.
#[test]
fn proto_declares_client_open_handshake_with_reported_version() {
    let proto = strip_comments(PROTO);
    assert!(
        message_block(&proto, "ClientOpenHandshake").is_some(),
        "api/boundless.proto must declare `message ClientOpenHandshake`"
    );
    let cv = message_block(&proto, "ClientVersion")
        .expect("api/boundless.proto must declare `message ClientVersion`");
    assert!(declares_field(cv, "platform") && declares_field(cv, "app_version"));
}

/// Guard the scanner itself: a missing message yields `None`, and a prefix is not a match — so
/// the AC7 assertion above can't pass on a coincidental substring.
#[test]
fn message_block_scanner_is_exact() {
    let src =
        "message ServerOpenHandshakeExtra { string nope = 1; }\nmessage Other { int32 x = 1; }";
    assert!(message_block(src, "ServerOpenHandshake").is_none());
    assert!(message_block(src, "Other").is_some());
}

/// Guard the comment stripper: a `/*` appearing INSIDE a `//` line comment (as in this file's own
/// `/api/auth/*` header line) must not open a block comment and swallow the following message.
#[test]
fn strip_comments_handles_block_marker_inside_line_comment() {
    let src = "// see /api/auth/* here\nmessage M { string f = 1; }\n";
    let stripped = strip_comments(src);
    assert!(stripped.contains("message M"), "stripped = {stripped:?}");
    let body = message_block(&stripped, "M").expect("message M survives stripping");
    assert!(declares_field(body, "f"));
    // A real block comment is still removed.
    assert!(!strip_comments("a /* x */ b").contains('x'));
}

/// Guard `declares_field` against the nested-message false-PASS: a field with the target name
/// declared inside a CHILD message must NOT satisfy a search for a direct field of the parent —
/// otherwise the AC7 assertion could be fooled once matching/realtime messages grow the proto.
#[test]
fn declares_field_ignores_nested_message_fields() {
    let body = " string real = 1;\n message Child { string client_min_version = 1; }\n";
    assert!(declares_field(body, "real"));
    assert!(
        !declares_field(body, "client_min_version"),
        "a field nested in a child message must not count as a direct field"
    );
}
