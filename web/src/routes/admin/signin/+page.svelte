<script lang="ts">
	// Admin sign-in — WebAuthn assertion only, NO password field anywhere (AC2). Copy from the catalog
	// (P8). The browser ceremony helper is dynamically imported (browser-only). On success the server
	// sets the §10-F session cookie and we navigate to the (placeholder) admin home.

	import { startAuthentication } from '@simplewebauthn/browser';

	import { goto } from '$app/navigation';

	import { t } from '$lib/i18n';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale);

	type Phase = 'idle' | 'signing_in' | 'error';
	let phase = $state<Phase>('idle');

	async function signIn(): Promise<void> {
		phase = 'signing_in';

		const optionsRes = await fetch('/api/admin/auth/signin');
		const { publicKey } = (await optionsRes.json()) as { publicKey: unknown };

		let response;
		try {
			response = await startAuthentication({
				optionsJSON: publicKey as Parameters<typeof startAuthentication>[0]['optionsJSON'],
			});
		} catch {
			phase = 'error';
			return;
		}

		const res = await fetch('/api/admin/auth/signin', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ response }),
		});
		// Frozen contract: 200 { admin_id } = signed in; 400 = failure (all surfaced as a calm retry).
		if (res.ok) {
			await goto('/admin');
			return;
		}
		phase = 'error';
	}
</script>

<svelte:head><title>{t('admin.signin.title', locale)}</title></svelte:head>

<main class="bl-page">
	<div class="bl-card">
		<h1 class="bl-h1">{t('admin.signin.title', locale)}</h1>
		<p class="bl-body">{t('admin.signin.explainer', locale)}</p>

		<button class="bl-button" type="button" onclick={signIn} disabled={phase === 'signing_in'}>
			{t('admin.signin.action', locale)}
		</button>

		<p class="bl-status" role="status" aria-live="polite">
			{#if phase === 'signing_in'}{t('admin.signin.signing_in', locale)}{/if}
		</p>
		{#if phase === 'error'}
			<p class="bl-error" role="alert">{t('admin.signin.failed', locale)}</p>
		{/if}
	</div>
</main>
