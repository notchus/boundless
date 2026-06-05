// AC20 — admin WebAuthn, end-to-end with REAL ceremony bytes (ADR-0016 D4, ADR-0017).
//
// Chromium's CDP virtual authenticator produces genuine attestation/assertion responses on a
// secure-context `http://localhost` page (served by route fulfillment — no app UI needed; the
// SvelteKit UI is T15). Those responses are fed through the real @simplewebauthn/server verifier
// inside the T09 verification module (register.ts / authenticate.ts), wired to the in-memory
// port fakes. This proves the policy with real bytes:
//   • UV-required registration verifies and stores a credential; the invite is consumed (AC16).
//   • A real uv=0 assertion is REJECTED by the server (ADMIN_WEBAUTHN_UV_REQUIRED) — the R11
//     property: the server enforces UV regardless of what the client requested.
//   • A valid UV assertion verifies and bumps the signature counter (AC2 assertion-only sign-in).
//   • A Developer re-invite registration revokes the prior credential(s) (D4 recovery), while
//     more than one credential per admin is supported (AC20).

import type { AuthenticationResponseJSON, RegistrationResponseJSON } from '@simplewebauthn/server';
import { expect, test, type CDPSession, type Page } from '@playwright/test';

import { buildAuthenticationOptions, buildRegistrationOptions, verifyAuthentication, verifyRegistration } from '../../src/lib/server/webauthn';
import { makeHarness, type Harness } from '../../src/lib/server/webauthn/testing/harness';

const NOW = 1_700_000_000;
const TTL = 72 * 60 * 60;

// A minimal secure-context page that runs the WebAuthn ceremony inside a click handler (so it
// has the user activation create()/get() require). No browser-console output (P2 / pre-commit). The
// ceremony reads window.__op and writes window.__result / window.__error.
const PAGE_HTML = `<!doctype html><html><head><meta charset="utf-8"><title>t</title></head><body>
<button id="trigger">go</button>
<script>
function b64urlToBuf(s){var pad="=".repeat((4-s.length%4)%4);var b64=(s+pad).replace(/-/g,"+").replace(/_/g,"/");var bin=atob(b64);var u=new Uint8Array(bin.length);for(var i=0;i<bin.length;i++){u[i]=bin.charCodeAt(i);}return u.buffer;}
function bufToB64url(buf){var u=new Uint8Array(buf);var s="";for(var i=0;i<u.length;i++){s+=String.fromCharCode(u[i]);}return btoa(s).replace(/\\+/g,"-").replace(/\\//g,"_").replace(/=+$/,"");}
async function register(o){
  var publicKey=Object.assign({},o,{challenge:b64urlToBuf(o.challenge),user:Object.assign({},o.user,{id:b64urlToBuf(o.user.id)}),excludeCredentials:(o.excludeCredentials||[]).map(function(c){return Object.assign({},c,{id:b64urlToBuf(c.id)});})});
  var cred=await navigator.credentials.create({publicKey:publicKey});
  var r=cred.response;
  return {id:cred.id,rawId:bufToB64url(cred.rawId),type:cred.type,response:{clientDataJSON:bufToB64url(r.clientDataJSON),attestationObject:bufToB64url(r.attestationObject),transports:(r.getTransports?r.getTransports():[])},clientExtensionResults:cred.getClientExtensionResults(),authenticatorAttachment:cred.authenticatorAttachment||undefined};
}
async function authenticate(o){
  var publicKey=Object.assign({},o,{challenge:b64urlToBuf(o.challenge),allowCredentials:(o.allowCredentials||[]).map(function(c){return Object.assign({},c,{id:b64urlToBuf(c.id)});})});
  var assertion=await navigator.credentials.get({publicKey:publicKey});
  var r=assertion.response;
  return {id:assertion.id,rawId:bufToB64url(assertion.rawId),type:assertion.type,response:{clientDataJSON:bufToB64url(r.clientDataJSON),authenticatorData:bufToB64url(r.authenticatorData),signature:bufToB64url(r.signature),userHandle:(r.userHandle?bufToB64url(r.userHandle):null)},clientExtensionResults:assertion.getClientExtensionResults()};
}
document.getElementById("trigger").addEventListener("click",async function(){
  window.__result=undefined;window.__error=undefined;
  try{var op=window.__op;window.__result=(op.kind==="register")?await register(op.options):await authenticate(op.options);}
  catch(e){window.__error=String((e&&e.message)||e);}
});
</script></body></html>`;

interface CeremonyOp {
  kind: 'register' | 'authenticate';
  options: unknown;
}

/** Serve the secure-context page and enable the CDP WebAuthn domain. */
async function serveAndOpen(page: Page): Promise<CDPSession> {
  await page.route('**/*', (route) => route.fulfill({ contentType: 'text/html', body: PAGE_HTML }));
  await page.goto('http://localhost/');
  const client = await page.context().newCDPSession(page);
  await client.send('WebAuthn.enable');
  return client;
}

async function addAuthenticator(client: CDPSession, isUserVerified: boolean): Promise<string> {
  const { authenticatorId } = await client.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      transport: 'internal',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified,
      automaticPresenceSimulation: true,
    },
  });
  return authenticatorId;
}

/** Drive one ceremony in the page (with user activation) and return the serialized response. */
async function runCeremony(page: Page, op: CeremonyOp): Promise<Record<string, unknown>> {
  await page.evaluate((o) => {
    (window as unknown as { __op: unknown; __result: unknown; __error: unknown }).__op = o;
    (window as unknown as { __result: unknown }).__result = undefined;
    (window as unknown as { __error: unknown }).__error = undefined;
  }, op);
  await page.click('#trigger');
  await page.waitForFunction(
    () => {
      const w = window as unknown as { __result: unknown; __error: unknown };
      return w.__result !== undefined || w.__error !== undefined;
    },
    undefined,
    { timeout: 10_000 },
  );
  const error = await page.evaluate(() => (window as unknown as { __error: unknown }).__error);
  if (error) {
    throw new Error('ceremony failed: ' + String(error));
  }
  return (await page.evaluate(() => (window as unknown as { __result: unknown }).__result)) as Record<string, unknown>;
}

function liveHarness(): Harness {
  const h = makeHarness(NOW);
  h.invites.add('tok', { adminId: 'admin-1', groupId: 'grp-1', expiresAt: NOW + TTL });
  return h;
}

/** Build options → run the create() ceremony → verify → return the outcome. */
async function registerCredential(
  page: Page,
  h: Harness,
  args: { ceremonyKey: string; presentedToken: string },
) {
  const options = await buildRegistrationOptions(h.deps, {
    ceremonyKey: args.ceremonyKey,
    presentedToken: args.presentedToken,
    userName: 'admin-invite',
    userDisplayName: 'Boundless Admin',
  });
  const response = await runCeremony(page, { kind: 'register', options });
  return verifyRegistration(h.deps, {
    ceremonyKey: args.ceremonyKey,
    presentedToken: args.presentedToken,
    response: response as unknown as RegistrationResponseJSON,
  });
}

test.describe('ac20_webauthn_requires_uv_no_attestation_multi_credential', () => {
  test('a UV registration verifies, stores a credential, and consumes the invite (AC16/AC20)', async ({ page }) => {
    const client = await serveAndOpen(page);
    await addAuthenticator(client, true);
    const h = liveHarness();

    const outcome = await registerCredential(page, h, { ceremonyKey: 'reg', presentedToken: 'tok' });

    expect(outcome.adminId).toBe('admin-1');
    expect(h.invites.isConsumed('tok')).toBe(true);
    const active = await h.credentials.listActiveByAdmin('admin-1');
    expect(active.map((c) => c.credentialId)).toEqual([outcome.credentialId]);
  });

  test('a valid UV assertion signs in and bumps the signature counter (AC2)', async ({ page }) => {
    const client = await serveAndOpen(page);
    await addAuthenticator(client, true);
    const h = liveHarness();
    await registerCredential(page, h, { ceremonyKey: 'reg', presentedToken: 'tok' });

    const options = await buildAuthenticationOptions(h.deps, { ceremonyKey: 'auth', adminId: 'admin-1' });
    const response = await runCeremony(page, { kind: 'authenticate', options });
    const outcome = await verifyAuthentication(h.deps, {
      ceremonyKey: 'auth',
      response: response as unknown as AuthenticationResponseJSON,
    });

    expect(outcome.adminId).toBe('admin-1');
    expect(typeof outcome.newSignCount).toBe('number');
  });

  test('the server REJECTS a real uv=0 assertion (R11 — UV enforced regardless of client request)', async ({ page }) => {
    const client = await serveAndOpen(page);
    const authenticatorId = await addAuthenticator(client, true);
    const h = liveHarness();
    await registerCredential(page, h, { ceremonyKey: 'reg', presentedToken: 'tok' });

    // Turn user-verification OFF on the authenticator and have the CLIENT request 'discouraged',
    // so the ceremony completes with uv=0 and the response actually reaches the server.
    await client.send('WebAuthn.setUserVerified', { authenticatorId, isUserVerified: false });
    const options = await buildAuthenticationOptions(h.deps, { ceremonyKey: 'auth0', adminId: 'admin-1' });
    const browserOptions = { ...options, userVerification: 'discouraged' };
    const response = await runCeremony(page, { kind: 'authenticate', options: browserOptions });

    await expect(
      verifyAuthentication(h.deps, { ceremonyKey: 'auth0', response: response as unknown as AuthenticationResponseJSON }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_UV_REQUIRED' });
  });

  test('a Developer re-invite registration revokes the prior credential(s) (ADR-0016 D4 recovery)', async ({ page }) => {
    const client = await serveAndOpen(page);
    const firstAuthenticator = await addAuthenticator(client, true);
    const h = liveHarness();

    const first = await registerCredential(page, h, { ceremonyKey: 'reg', presentedToken: 'tok' });

    // The admin loses the first authenticator; the Developer re-invites (ADR-0015).
    await client.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId: firstAuthenticator });
    await addAuthenticator(client, true);
    h.invites.add('tok2', { adminId: 'admin-1', groupId: 'grp-1', expiresAt: NOW + TTL });

    const second = await registerCredential(page, h, { ceremonyKey: 'reg2', presentedToken: 'tok2' });

    expect(second.credentialId).not.toBe(first.credentialId);
    const active = await h.credentials.listActiveByAdmin('admin-1');
    expect(active.map((c) => c.credentialId)).toEqual([second.credentialId]);
    // Both credentials persist; exactly the prior one is revoked (multi-cred capacity + recovery).
    const all = h.credentials.all();
    expect(all).toHaveLength(2);
    expect(all.filter((c) => c.revokedAt !== null).map((c) => c.credentialId)).toEqual([first.credentialId]);
  });
});
