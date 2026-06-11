<script lang="ts">
	// Member detail (spec 008 T10) — the decrypted member shown after an AUDITED read (I5). Edit runs in
	// a focus-trapped melt-ui dialog under optimistic concurrency (the loaded `updated_at` rides as a
	// hidden field; a concurrent change → calm "someone else changed this" copy). Regenerate mints a
	// fresh show-once Onboarding Code. All copy from the catalog (P8).
	import { onMount } from 'svelte';

	import { enhance } from '$app/forms';
	import { createDialog, melt } from '@melt-ui/svelte';
	import type { SubmitFunction } from '@sveltejs/kit';

	import { t, type MessageKey } from '$lib/i18n';
	import { memberErrorCatalogKey } from '$lib/i18n/member-errors';
	import { roleKey, statusKey } from '$lib/i18n/member-labels';
	import type { MemberDetail } from '$lib/members-types';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale);
	const member = $derived(data.member);

	// — Edit dialog (auto-opens when arriving via the row menu's `?edit=1`) —
	const {
		elements: { trigger, overlay, content, title, description, close },
		states: { open },
	} = createDialog({ forceVisible: false });

	// Open the edit dialog on arrival when the row menu deep-linked with `?edit=1` (read in a closure so
	// it doesn't capture a stale reactive value).
	onMount(() => {
		if (data.openEdit) open.set(true);
	});

	let editError = $state<MessageKey | null>(null);
	let editSubmitting = $state(false);

	const submitEdit: SubmitFunction = () => {
		editSubmitting = true;
		return async ({ result, update }) => {
			editSubmitting = false;
			editError = null;
			if (result.type === 'success') {
				await update({ invalidateAll: true });
				open.set(false);
			} else if (result.type === 'failure') {
				editError = memberErrorCatalogKey((result.data as { errorCode?: string } | undefined)?.errorCode ?? '');
				await update({ reset: false });
			} else {
				await update();
			}
		};
	};

	// — Regenerate code —
	let regenerated = $state<{ onboarding_code: string | null; code_expires_at: number } | null>(null);
	let regenSubmitting = $state(false);

	const submitRegen: SubmitFunction = () => {
		regenSubmitting = true;
		return async ({ result, update }) => {
			regenSubmitting = false;
			regenerated = null;
			if (result.type === 'success') {
				const d = result.data as { ok?: boolean; regenerated?: { onboarding_code: string | null; code_expires_at: number } } | undefined;
				if (d?.ok && d.regenerated) regenerated = d.regenerated;
				await update({ invalidateAll: true });
			} else {
				await update();
			}
		};
	};

	function has(m: MemberDetail, role: 'rider' | 'driver'): boolean {
		return m.roles.includes(role);
	}
	function fmtExpiry(epochSecs: number): string {
		return new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' }).format(
			new Date(epochSecs * 1000),
		);
	}
</script>

<svelte:head><title>{t('admin.member.detail_title', locale)}</title></svelte:head>

<p class="mt-2"><a class="bl-link-inline" href="/admin/members">{t('admin.members.title', locale)}</a></p>

{#if member}
	<h1 class="bl-h1">{member.name}</h1>

	<dl class="mt-6 grid grid-cols-[max-content_1fr] gap-x-6 gap-y-2">
		<dt class="bl-label">{t('admin.member.phone', locale)}</dt>
		<dd>{member.phone}</dd>
		<dt class="bl-label">{t('admin.member.address', locale)}</dt>
		<dd>{member.address}</dd>
		<dt class="bl-label">{t('admin.member.role', locale)}</dt>
		<dd>{#each member.roles as r (r)}<span class="bl-badge">{t(roleKey(r), locale)}</span>{/each}</dd>
		<dt class="bl-label">{t('admin.member.status', locale)}</dt>
		<dd>{t(statusKey(member.onboarding_status), locale)}</dd>
	</dl>

	<div class="bl-toolbar">
		<button type="button" class="bl-btn" use:melt={$trigger}>{t('admin.member.edit', locale)}</button>
		<form method="POST" action="?/regenerate" use:enhance={submitRegen}>
			<button type="submit" class="bl-btn-secondary" disabled={regenSubmitting}>
				{t('admin.member.regenerate_code', locale)}
			</button>
		</form>
	</div>

	<p class="bl-status" role="status" aria-live="polite">
		{#if regenSubmitting}{t('admin.member.saving', locale)}{/if}
	</p>
	{#if regenerated}
		<div role="status" aria-live="polite">
			<p class="bl-status">{t('admin.member.code_regenerated', locale)}</p>
			<p class="bl-body-flush">{t('admin.member.code_explainer', locale)}</p>
			{#if regenerated.onboarding_code}
				<p class="bl-label mt-4">{t('admin.member.onboarding_code', locale)}</p>
				<p class="bl-code">{regenerated.onboarding_code}</p>
			{/if}
			<p class="bl-body-flush mt-2">
				{t('admin.member.code_expires', locale, { when: fmtExpiry(regenerated.code_expires_at) })}
			</p>
		</div>
	{/if}

	{#if $open}
		<div class="bl-overlay" use:melt={$overlay}></div>
		<div class="bl-dialog" use:melt={$content}>
			<h2 class="bl-h2" use:melt={$title}>{t('admin.member.edit', locale)}</h2>
			<p class="bl-body-flush" use:melt={$description}>{member.name}</p>
			<form method="POST" action="?/edit" use:enhance={submitEdit}>
				<input type="hidden" name="expected_updated_at" value={member.updated_at} />
				<div class="bl-field mt-4">
					<label class="bl-label" for="e-name">{t('admin.member.name', locale)}</label>
					<input class="bl-input" id="e-name" name="name" value={member.name} required autocomplete="off" />
				</div>
				<div class="bl-field mt-4">
					<label class="bl-label" for="e-phone">{t('admin.member.phone', locale)}</label>
					<input class="bl-input" id="e-phone" name="phone" type="tel" inputmode="tel" value={member.phone} required autocomplete="off" />
				</div>
				<div class="bl-field mt-4">
					<label class="bl-label" for="e-address">{t('admin.member.address', locale)}</label>
					<input class="bl-input" id="e-address" name="address" value={member.address} required autocomplete="off" />
				</div>
				<fieldset class="bl-field mt-4">
					<legend class="bl-label">{t('admin.member.role', locale)}</legend>
					<label class="inline-flex min-h-11 items-center gap-2">
						<input type="checkbox" name="roles" value="rider" checked={has(member, 'rider')} />
						{t('admin.member.role_rider', locale)}
					</label>
					<label class="inline-flex min-h-11 items-center gap-2">
						<input type="checkbox" name="roles" value="driver" checked={has(member, 'driver')} />
						{t('admin.member.role_driver', locale)}
					</label>
				</fieldset>

				{#if editError}
					<p class="bl-error" role="alert">{t(editError, locale)}</p>
				{/if}
				<p class="bl-status" role="status" aria-live="polite">
					{#if editSubmitting}{t('admin.member.saving', locale)}{/if}
				</p>

				<div class="bl-dialog-actions">
					<button type="button" class="bl-btn-secondary" use:melt={$close}>{t('admin.member.cancel', locale)}</button>
					<button type="submit" class="bl-btn" disabled={editSubmitting}>{t('admin.member.save', locale)}</button>
				</div>
			</form>
		</div>
	{/if}
{:else}
	<h1 class="bl-h1">{t('admin.member.detail_title', locale)}</h1>
	<p class="bl-error" role="alert">{t(memberErrorCatalogKey(data.errorCode ?? ''), locale)}</p>
{/if}
