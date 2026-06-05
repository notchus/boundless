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
