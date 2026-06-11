// AC7 (OpenAPI leg) — the contract-freeze gate (spec 001 T10).
//
// Parses the frozen api/openapi.yaml and asserts that `client_min_version` AND
// `client_recommended_version` are REQUIRED fields on EVERY /api/auth/* response (asserts O4 +
// O5's straggler signal). The contract puts both in a shared `VersionHandshake` component that
// each auth response `allOf`s, so they are required in every routing variant.
//
// Lives in web/ because the web client is the openapi-typescript consumer; the proto leg
// (ac7_ws_handshake_has_client_min_version) is a dependency-free Rust test in core/sync.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

// Three levels up from web/tests/contract/ → the repo-root api/openapi.yaml.
const OPENAPI_PATH = fileURLToPath(new URL('../../../api/openapi.yaml', import.meta.url));

type Json = Record<string, unknown>;

function isObject(v: unknown): v is Json {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

const doc = parse(readFileSync(OPENAPI_PATH, 'utf8')) as Json;

function schemas(): Json {
  const components = doc['components'];
  if (!isObject(components)) throw new Error('openapi.yaml: missing components');
  const s = components['schemas'];
  if (!isObject(s)) throw new Error('openapi.yaml: missing components.schemas');
  return s;
}

// Resolve a local `$ref` (#/components/schemas/Name) one level.
function deref(node: unknown): unknown {
  if (isObject(node) && typeof node['$ref'] === 'string') {
    return schemas()[node['$ref'].replace('#/components/schemas/', '')];
  }
  return node;
}

// The union of `required` field names reachable through `$ref` + `allOf` chains. It deliberately
// does NOT descend `oneOf`/`anyOf`: a field is "always present" only if it is shared across every
// alternative — which is exactly how the contract is built (the version handshake lives in a
// shared `allOf` member; the routing variants live under a sibling `oneOf`). `seen` is keyed on the
// resolved schema *identity* and a re-encountered schema contributes nothing further; that is
// correct here because the only `required`-bearing schema reached via `allOf` is `VersionHandshake`
// itself, and no schema is double-counted through two distinct `allOf` branches.
function requiredUnion(node: unknown, seen = new Set<unknown>()): Set<string> {
  const out = new Set<string>();
  const s = deref(node);
  if (!isObject(s) || seen.has(s)) return out;
  seen.add(s);
  const req = s['required'];
  if (Array.isArray(req)) for (const r of req) if (typeof r === 'string') out.add(r);
  const allOf = s['allOf'];
  if (Array.isArray(allOf)) for (const m of allOf) for (const r of requiredUnion(m, seen)) out.add(r);
  return out;
}

const AUTH_PREFIX = '/api/auth/';
const REQUIRED_VERSION_FIELDS = ['client_min_version', 'client_recommended_version'] as const;
const HTTP_METHODS = ['get', 'put', 'post', 'delete', 'patch', 'options', 'head', 'trace'];

interface AuthResponseCase {
  readonly id: string;
  readonly path: string;
  readonly schema: unknown;
}

function pathsObject(): Json {
  const paths = doc['paths'];
  if (!isObject(paths)) throw new Error('openapi.yaml: missing paths');
  return paths;
}

// Every declared `/api/auth/*` path key — the set the AC7 coverage assertion pins to (so a
// renamed/typo'd path can't silently drop coverage while a hardcoded count still passes).
function declaredAuthPaths(): string[] {
  return Object.keys(pathsObject()).filter((p) => p.startsWith(AUTH_PREFIX));
}

function authResponseCases(): AuthResponseCase[] {
  const cases: AuthResponseCase[] = [];
  for (const [path, item] of Object.entries(pathsObject())) {
    if (!path.startsWith(AUTH_PREFIX) || !isObject(item)) continue;
    for (const method of HTTP_METHODS) {
      const op = item[method];
      if (!isObject(op)) continue;
      const responses = op['responses'];
      if (!isObject(responses)) continue;
      for (const [status, resp] of Object.entries(responses)) {
        if (!isObject(resp)) continue;
        const content = resp['content'];
        const json = isObject(content) ? content['application/json'] : undefined;
        const schema = isObject(json) ? json['schema'] : undefined;
        cases.push({ id: `${method.toUpperCase()} ${path} -> ${status}`, path, schema });
      }
    }
  }
  return cases;
}

describe('AC7 — API contract freeze (OpenAPI leg, spec 001 T10)', () => {
  it('VersionHandshake itself requires both version fields (the shared anchor)', () => {
    const req = requiredUnion(schemas()['VersionHandshake']);
    for (const f of REQUIRED_VERSION_FIELDS) {
      expect(req.has(f), `VersionHandshake must require ${f}`).toBe(true);
    }
  });

  it('ac7_auth_responses_require_min_and_recommended_version', () => {
    const authPaths = declaredAuthPaths();
    const cases = authResponseCases();

    // Non-vacuity, pinned to the contract's OWN path set (not a magic count): the four frozen auth
    // endpoints must exist, and EVERY declared /api/auth/* path must contribute ≥1 checked response
    // — so a renamed/typo'd path can't silently drop coverage.
    expect(authPaths.length).toBeGreaterThanOrEqual(4);
    const coveredPaths = new Set(cases.map((c) => c.path));
    for (const p of authPaths) {
      expect(coveredPaths.has(p), `${p}: no application/json response case found`).toBe(true);
    }

    for (const c of cases) {
      expect(c.schema, `${c.id}: declares an application/json response schema`).toBeTruthy();
      const req = requiredUnion(c.schema);
      for (const f of REQUIRED_VERSION_FIELDS) {
        expect(req.has(f), `${c.id}: must require ${f} (AC7 / O4 / O5)`).toBe(true);
      }
    }
  });
});

// The three auth REQUEST schemas, each a flat object (no allOf/oneOf) — so its own `required` +
// `properties` are read directly.
const AUTH_REQUEST_SCHEMAS = ['SignInRequest', 'BindDeviceRequest', 'RecoveryRebindRequest'] as const;

function ownSchema(name: string): { required: Set<string>; properties: Set<string> } {
  const s = schemas()[name];
  if (!isObject(s)) throw new Error(`openapi.yaml: missing schema ${name}`);
  const required = new Set<string>();
  const req = s['required'];
  if (Array.isArray(req)) for (const r of req) if (typeof r === 'string') required.add(r);
  const properties = new Set<string>();
  const props = s['properties'];
  if (isObject(props)) for (const k of Object.keys(props)) properties.add(k);
  return { required, properties };
}

describe('ADR-0023 — auth requests carry the plaintext phone, not a client-computed hash', () => {
  // I3's `phone_lookup_hash` is HMAC-SHA256 keyed by a per-instance SERVER secret (core/crypto), so
  // no client can compute it; the server hashes the received E.164 `phone` and drops the plaintext
  // (never logged — P2; only the keyed hash is persisted — I3). The request contract must therefore
  // carry `phone` and never `phone_lookup_hash` — a regression to the latter would generate a client
  // that sends a base64 hash the server can't use.
  it('all three auth requests require `phone` and expose no `phone_lookup_hash`', () => {
    for (const name of AUTH_REQUEST_SCHEMAS) {
      const { required, properties } = ownSchema(name);
      expect(required.has('phone'), `${name}.required must include phone`).toBe(true);
      expect(properties.has('phone'), `${name}.properties must include phone`).toBe(true);
      expect(
        properties.has('phone_lookup_hash'),
        `${name}: must not expose a client-computed phone_lookup_hash (ADR-0023)`,
      ).toBe(false);
      expect(
        required.has('phone_lookup_hash'),
        `${name}: must not require a client-computed phone_lookup_hash (ADR-0023)`,
      ).toBe(false);
    }
  });
});

// ── Spec 008 — admin member-management surface (additive freeze) ──────────────────────────
// The /api/auth/* freeze above is unaffected (these are new /api/admin/members + /api/admin/audit-log
// paths). Four gates: PII handlers are marked x-requires-audit (I5/AC7 coverage leg), MemberSummary
// carries no tainted field (AC8), the issuance error codes are registered (P12), and the member
// surface offers no Admin-creation path (I11/AC10).

const ERROR_CODES_PATH = fileURLToPath(new URL('../../../docs/error-codes.md', import.meta.url));
const RAW_OPENAPI = readFileSync(OPENAPI_PATH, 'utf8');

// The seven stable spec-008 issuance codes (docs/error-codes.md "Admin member-management" section).
// Append-only + stable, so listing them here pins registry completeness, not just the contract literals.
const EXPECTED_ADMIN_ISSUANCE_CODES = [
  'ADMIN_MEMBER_PHONE_INVALID',
  'ADMIN_MEMBER_ADDRESS_INVALID',
  'ADMIN_MEMBER_ROLES_REQUIRED',
  'ADMIN_MEMBER_DUPLICATE_PHONE',
  'ADMIN_MEMBER_EDIT_STALE',
  'ADMIN_MEMBER_ROLE_FORBIDDEN',
  'ADMIN_GROUP_KEY_MISSING',
] as const;

// Every `#/components/schemas/<Name>` reachable from `node` through $ref / allOf·oneOf·anyOf / items /
// properties (component-deref'd, with a seen set so cycles terminate). Used to ask "does this response
// graph reach the PII-bearing MemberDetail?".
function collectSchemaRefs(node: unknown, acc = new Set<string>(), seen = new Set<unknown>()): Set<string> {
  if (Array.isArray(node)) {
    for (const x of node) collectSchemaRefs(x, acc, seen);
    return acc;
  }
  if (!isObject(node) || seen.has(node)) return acc;
  seen.add(node);
  const ref = node['$ref'];
  if (typeof ref === 'string' && ref.startsWith('#/components/schemas/')) {
    const name = ref.replace('#/components/schemas/', '');
    if (!acc.has(name)) {
      acc.add(name);
      collectSchemaRefs(schemas()[name], acc, seen);
    }
  }
  for (const key of ['allOf', 'oneOf', 'anyOf', 'items']) if (key in node) collectSchemaRefs(node[key], acc, seen);
  const props = node['properties'];
  if (isObject(props)) for (const v of Object.values(props)) collectSchemaRefs(v, acc, seen);
  return acc;
}

interface OpCase {
  readonly id: string;
  readonly op: Json;
}

// Every (method, path) operation in the whole document.
function allOps(): OpCase[] {
  const out: OpCase[] = [];
  for (const [path, item] of Object.entries(pathsObject())) {
    if (!isObject(item)) continue;
    for (const method of HTTP_METHODS) {
      const op = item[method];
      if (isObject(op)) out.push({ id: `${method.toUpperCase()} ${path}`, op });
    }
  }
  return out;
}

// The application/json response schemas declared by an operation (across all status codes).
function responseSchemas(op: Json): unknown[] {
  const out: unknown[] = [];
  const responses = op['responses'];
  if (!isObject(responses)) return out;
  for (const resp of Object.values(responses)) {
    if (!isObject(resp)) continue;
    const content = resp['content'];
    const json = isObject(content) ? content['application/json'] : undefined;
    const schema = isObject(json) ? json['schema'] : undefined;
    if (schema !== undefined) out.push(schema);
  }
  return out;
}

// The schemas whose presence in a response means the handler DISCLOSED member PII to the admin and so
// MUST be audited (I5). `MemberDetail` = the full detail read (phone+address). `DuplicatePhoneLink` =
// the issuance surface-and-link of an EXISTING member's name (a targeted, phone-keyed disclosure — the
// core writes its audit row in `IssueMemberOutcome::DuplicatePhone`). Deliberately NOT here:
// `MemberSummary` on the list path (a bulk name list is not an audited read — spec.md / member.rs).
// Hand-curated, like T06's sealed `AuditedResponse` allowlist — negative schema bounds don't exist.
const AUDITED_DISCLOSURE_SCHEMAS = ['MemberDetail', 'DuplicatePhoneLink'] as const;

function isPiiDisclosureHandler(op: Json): boolean {
  const refs = new Set<string>();
  for (const s of responseSchemas(op)) collectSchemaRefs(s, refs);
  return AUDITED_DISCLOSURE_SCHEMAS.some((name) => refs.has(name));
}

describe('spec 008 — admin member-management surface (additive)', () => {
  it('openapi_pii_handlers_all_require_audit', () => {
    // I5/AC7 coverage leg (the named second layer behind T06's compile gate): ANY handler whose
    // response graph reaches an audited-disclosure schema (MemberDetail OR DuplicatePhoneLink) MUST
    // declare `x-requires-audit: true`.
    const piiHandlers = allOps().filter(({ op }) => isPiiDisclosureHandler(op));
    // Non-vacuity + scope-proof: the detail READ (GET) + EDIT (PATCH) reach MemberDetail; the issuance
    // POST reaches DuplicatePhoneLink. The gate must cover all THREE — the duplicate-phone disclosure is
    // the one place a name-only MemberSummary IS audited, so without DuplicatePhoneLink the POST escaped.
    const ids = new Set(piiHandlers.map(({ id }) => id));
    expect(ids.has('POST /api/admin/members'), 'the audit gate must cover POST /api/admin/members (duplicate-phone disclosure)').toBe(true);
    expect([...ids].some((id) => id.startsWith('GET /api/admin/members/')), 'the gate must cover GET /{id}').toBe(true);
    expect([...ids].some((id) => id.startsWith('PATCH /api/admin/members/')), 'the gate must cover PATCH /{id}').toBe(true);
    expect(piiHandlers.length, 'expected ≥3 audited-disclosure handlers').toBeGreaterThanOrEqual(3);
    for (const { id, op } of piiHandlers) {
      expect(op['x-requires-audit'], `${id} discloses member PII → must be x-requires-audit: true (I5)`).toBe(true);
    }
  });

  it('openapi_admin_surface_requires_shared_secret', () => {
    // ADR-0026 / I11: every admin member-management op declares the shared-secret scheme + the
    // X-Admin-Id param. OpenAPI's default with no global `security` is fail-OPEN, so a future admin op
    // that forgot `security` would be unauthenticated — this gate forbids that for the whole surface.
    const adminOps = allOps().filter(
      ({ id }) => id.includes(' /api/admin/members') || id.includes(' /api/admin/audit-log'),
    );
    expect(adminOps.length, 'expected ≥6 admin member-management ops').toBeGreaterThanOrEqual(6);
    for (const { id, op } of adminOps) {
      const security = op['security'];
      const declared =
        Array.isArray(security) && security.some((s) => isObject(s) && 'adminSharedSecret' in s);
      expect(declared, `${id} must declare security: [adminSharedSecret] (ADR-0026, fail-closed)`).toBe(true);
      const params = op['parameters'];
      const hasAdminId =
        Array.isArray(params) &&
        params.some((p) => isObject(p) && p['$ref'] === '#/components/parameters/AdminIdHeader');
      expect(hasAdminId, `${id} must include the AdminIdHeader param (the I5 audit actor — ADR-0026)`).toBe(true);
    }
  });

  it('audit_entry_is_names_only_no_value_field', () => {
    // AC9 / R6: the audit log records field NAMES, never values. The entry carries no value slot, and
    // `fields` is the names-only enum — so the audit-log read is not itself a recursive PII read.
    const { properties } = ownSchema('AuditEntry');
    for (const forbidden of ['value', 'old_value', 'new_value', 'name', 'phone', 'address']) {
      expect(properties.has(forbidden), `AuditEntry must not carry a PII value field (${forbidden})`).toBe(false);
    }
    const entry = schemas()['AuditEntry'] as Json;
    const fields = (entry['properties'] as Json)['fields'] as Json;
    const items = fields['items'] as Json;
    expect(new Set(items['enum'] as unknown[])).toEqual(new Set(['name', 'phone', 'address']));
  });

  it('member_summary_schema_has_no_tainted_field', () => {
    // AC8: the list projection carries a display name + roles + status, and NEVER phone/address.
    const { required, properties } = ownSchema('MemberSummary');
    for (const present of ['member_id', 'name', 'roles', 'onboarding_status']) {
      expect(properties.has(present), `MemberSummary must expose ${present}`).toBe(true);
    }
    for (const tainted of ['phone', 'address']) {
      expect(properties.has(tainted), `MemberSummary must NOT expose ${tainted} (AC8)`).toBe(false);
      expect(required.has(tainted)).toBe(false);
    }
  });

  it('admin_issuance_error_codes_in_registry', () => {
    // P12: the load-bearing leg is the explicit EXPECTED list (registry completeness for the 7 stable
    // spec-008 issuance codes). The second leg sweeps the contract for issuance-family literals
    // (ADMIN_MEMBER_*/ADMIN_GROUP_KEY_*) and asserts each is registered — deliberately scoped to those
    // two families (other ADMIN_*/DEV_ADMIN_* codes belong to the frozen spec-001 sections).
    const registry = readFileSync(ERROR_CODES_PATH, 'utf8');
    for (const code of EXPECTED_ADMIN_ISSUANCE_CODES) {
      expect(registry.includes(code), `${code} must be registered in docs/error-codes.md (P12)`).toBe(true);
    }
    const referenced = new Set(RAW_OPENAPI.match(/ADMIN_(?:MEMBER|GROUP_KEY)_[A-Z_]+/g) ?? []);
    expect(referenced.size, 'expected ≥1 admin issuance code referenced in the contract').toBeGreaterThan(0);
    for (const code of referenced) {
      expect(registry.includes(code), `${code} (referenced in openapi.yaml) must be in docs/error-codes.md`).toBe(true);
    }
  });

  it('openapi_admin_surface_has_no_admin_creation_path', () => {
    // I11/AC10: a member cannot be issued as an Admin. The issuance role enum is exactly {rider,driver},
    // and both member-mutation requests use IssuableRole (not the full Role, which includes admin).
    const issuable = schemas()['IssuableRole'];
    expect(isObject(issuable), 'IssuableRole schema must exist').toBe(true);
    const enumVals = (issuable as Json)['enum'];
    expect(Array.isArray(enumVals)).toBe(true);
    expect(new Set(enumVals as unknown[])).toEqual(new Set(['rider', 'driver']));
    expect((enumVals as unknown[]).includes('admin'), 'IssuableRole must not include admin (I11)').toBe(false);
    for (const reqName of ['IssueMemberRequest', 'EditMemberRequest']) {
      const refs = collectSchemaRefs(schemas()[reqName]);
      expect(refs.has('IssuableRole'), `${reqName}.roles must use IssuableRole (rider/driver only)`).toBe(true);
      expect(refs.has('Role'), `${reqName} must not admit the full Role enum (would allow admin — I11)`).toBe(false);
    }
  });
});
