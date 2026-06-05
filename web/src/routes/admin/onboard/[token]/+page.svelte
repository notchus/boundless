<script lang="ts">
	// Admin registration landing + WebAuthn ceremony (spec 001 T15; AC2/AC11b/AC16/AC20).
	// Renders from SSR `data.invite` (live → register prompt; expired → InviteExpired). All copy
	// resolves from the catalog (P8); no password field anywhere (AC2).
	//
	// `@simplewebauthn/browser` is statically imported: it is SSR-safe (no top-level browser globals)
	// and `startRegistration`/`navigator.credentials.create()` are only called from the click handler.
	// A static import keeps the helper available synchronously at click time — a *dynamic* import
	// between the user gesture and `create()` can outlive the transient-activation window (esp. for
	// keyboard activation), which fails the ceremony.

	import { startRegistration } from '@simplewebauthn/browser';

	import { t, errorCatalogKey, type MessageKey } from '$lib/i18n';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale);
	const token = $derived(data.token);

	type Phase = 'idle' | 'registering' | 'registered' | 'error';
	let phase = $state<Phase>('idle');

	// The invite can be expired from the SSR load (reactive `data`) or become expired on a client
	// retry; combine both so the initial value isn't snapshotted out of reactive props.
	let expiredOnRetry = $state(false);
	const expired = $derived(data.status === 'expired' || expiredOnRetry);
	let errorKey = $state<MessageKey | null>(null);

	type Ceremony = { status: 'live'; options: unknown } | { status: 'expired' };

	// GET the registration ceremony per the frozen contract: 200 { publicKey } = live; 410 = expired.
	async function freshCeremony(): Promise<Ceremony> {
		const res = await fetch(`/api/admin/auth/invite/${encodeURIComponent(token)}`);
		if (res.status === 410) {
			return { status: 'expired' };
		}
		const body = (await res.json()) as { publicKey: unknown };
		return { status: 'live', options: body.publicKey };
	}

	async function register(): Promise<void> {
		phase = 'registering';
		errorKey = null;

		// Always fetch a fresh challenge (the prior one is single-use / consumed on the last attempt).
		const fresh = await freshCeremony();
		if (fresh.status === 'expired') {
			expiredOnRetry = true;
			return;
		}

		let response;
		try {
			response = await startRegistration({
				optionsJSON: fresh.options as Parameters<typeof startRegistration>[0]['optionsJSON'],
			});
		} catch {
			// User cancelled / no authenticator — re-prompt to try again (no scolding).
			errorKey = 'admin.onboarding.register_credential';
			phase = 'error';
			return;
		}

		const res = await fetch('/api/admin/auth/register', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ token, response }),
		});
		// Frozen contract: 200 { admin_id } = registered; 400 { error_code } = failure.
		if (res.ok) {
			phase = 'registered';
			return;
		}
		const body = (await res.json()) as { error_code?: string };
		const code = body.error_code ?? '';
		// A dead invite (consumed/expired during the ceremony) routes to the terminal screen; any other
		// ceremony failure is a retry (errorCatalogKey maps the invite codes → the invite_expired key).
		if (errorCatalogKey(code) === 'admin.onboarding.invite_expired') {
			expiredOnRetry = true;
			return;
		}
		errorKey = errorCatalogKey(code);
		phase = 'error';
	}
</script>

<svelte:head><title>{t('admin.onboarding.register_credential', locale)}</title></svelte:head>

<main class="bl-page">
	<div class="bl-card">
		{#if expired}
			<!-- Terminal InviteExpired. role=alert so a client-side transition is announced (AC11b). -->
			<div role="alert">
				<h1 class="bl-h1">{t('admin.onboarding.invite_expired', locale)}</h1>
			</div>
		{:else if phase === 'registered'}
			<h1 class="bl-h1">{t('admin.onboarding.register_credential', locale)}</h1>
			<p class="bl-status" role="status" aria-live="polite">
				{t('admin.onboarding.registered', locale)}
			</p>
			<a class="bl-link" href="/admin/signin">{t('admin.onboarding.go_to_signin', locale)}</a>
		{:else}
			<h1 class="bl-h1">{t('admin.onboarding.register_credential', locale)}</h1>
			<p class="bl-body">{t('admin.onboarding.register_explainer', locale)}</p>

			<button class="bl-button" type="button" onclick={register} disabled={phase === 'registering'}>
				{t('admin.onboarding.register_action', locale)}
			</button>

			<!-- Polite status (ceremony in progress) + assertive error region (AC11b aria-live). -->
			<p class="bl-status" role="status" aria-live="polite">
				{#if phase === 'registering'}{t('admin.onboarding.registering', locale)}{/if}
			</p>
			{#if phase === 'error' && errorKey}
				<p class="bl-error" role="alert">{t(errorKey, locale)}</p>
			{/if}
		{/if}
	</div>
</main>
