<script lang="ts">
	// Member list (spec 008 T10) — search/filter + a semantic table of PII-free summaries, plus the
	// "Add a member" flow in a focus-trapped melt-ui dialog. Issuance returns the show-once Onboarding
	// Code (read aloud / printed), surfaces-and-links a duplicate phone, or shows a calm validation
	// error (aria-live). All copy from the catalog (P8); the a11y bar (AC14) via semantic HTML + melt.
	import { enhance } from '$app/forms';
	import { createDialog, melt } from '@melt-ui/svelte';
	import type { SubmitFunction } from '@sveltejs/kit';

	import { t, type MessageKey } from '$lib/i18n';
	import { memberErrorCatalogKey } from '$lib/i18n/member-errors';
	import { roleKey, statusKey } from '$lib/i18n/member-labels';
	import type { MemberSummary, OnboardingStatus } from '$lib/members-types';
	import MemberActionsMenu from './MemberActionsMenu.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale);

	const STATUSES: readonly OnboardingStatus[] = [
		'issued_not_onboarded',
		'onboarded',
		'code_expired_or_lost',
		'needs_reonboarding',
	];

	// — Add-member dialog (melt: focus trap, Esc-to-close, focus return) —
	const {
		elements: { trigger, overlay, content, title, description, close },
		states: { open },
	} = createDialog({ forceVisible: false });

	type Issued = { member: MemberSummary; onboarding_code: string | null; code_expires_at: number };
	let issued = $state<Issued | null>(null);
	let duplicate = $state<MemberSummary | null>(null);
	let errorKey = $state<MessageKey | null>(null);
	let submitting = $state(false);

	// Reset the dialog result whenever it closes, so reopening "Add a member" starts fresh.
	$effect(() => {
		if (!$open) {
			issued = null;
			duplicate = null;
			errorKey = null;
		}
	});

	function fmtExpiry(epochSecs: number): string {
		return new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' }).format(
			new Date(epochSecs * 1000),
		);
	}

	const submitIssue: SubmitFunction = () => {
		submitting = true;
		return async ({ result, update }) => {
			submitting = false;
			issued = null;
			duplicate = null;
			errorKey = null;
			if (result.type === 'success') {
				const d = result.data as { ok?: boolean; issued?: Issued } | undefined;
				if (d?.ok && d.issued) issued = d.issued;
				await update({ reset: true, invalidateAll: true }); // refresh the list; clear fields
			} else if (result.type === 'failure') {
				const d = result.data as { errorCode?: string; existing?: MemberSummary } | undefined;
				if (d?.errorCode === 'ADMIN_MEMBER_DUPLICATE_PHONE' && d.existing) duplicate = d.existing;
				else errorKey = memberErrorCatalogKey(d?.errorCode ?? '');
				await update({ reset: false }); // keep the typed values for correction
			} else {
				await update();
			}
		};
	};
</script>

<svelte:head><title>{t('admin.members.title', locale)}</title></svelte:head>

<h1 class="bl-h1">{t('admin.members.title', locale)}</h1>

<p class="mt-4">
	<button type="button" class="bl-btn" use:melt={$trigger}>{t('admin.members.add', locale)}</button>
</p>

<form method="GET" class="bl-toolbar" role="search">
	<div class="bl-field">
		<label class="bl-label" for="f-search">{t('admin.members.search', locale)}</label>
		<input class="bl-input" id="f-search" type="search" name="search" value={data.filters.search} />
	</div>
	<div class="bl-field">
		<label class="bl-label" for="f-role">{t('admin.members.filter_role', locale)}</label>
		<select class="bl-select" id="f-role" name="role">
			<option value="">{t('admin.members.filter_all', locale)}</option>
			<option value="rider" selected={data.filters.role === 'rider'}>{t('admin.member.role_rider', locale)}</option>
			<option value="driver" selected={data.filters.role === 'driver'}>{t('admin.member.role_driver', locale)}</option>
		</select>
	</div>
	<div class="bl-field">
		<label class="bl-label" for="f-status">{t('admin.members.filter_status', locale)}</label>
		<select class="bl-select" id="f-status" name="status">
			<option value="">{t('admin.members.filter_all', locale)}</option>
			{#each STATUSES as s (s)}
				<option value={s} selected={data.filters.status === s}>{t(statusKey(s), locale)}</option>
			{/each}
		</select>
	</div>
	<button class="bl-btn-secondary" type="submit">{t('admin.members.search', locale)}</button>
</form>

{#if data.members.length === 0}
	<p class="bl-status" role="status">{t('admin.members.empty', locale)}</p>
{:else}
	<!-- A data table is a WCAG 1.4.10 reflow exception: contain its horizontal scroll so the PAGE never
	     scrolls 2-dimensionally at 320px (the table scrolls within this wrapper instead). -->
	<div class="overflow-x-auto">
		<table class="bl-table">
			<caption class="sr-only">{t('admin.members.title', locale)}</caption>
		<thead>
			<tr>
				<th class="bl-th" scope="col">{t('admin.member.name', locale)}</th>
				<th class="bl-th" scope="col">{t('admin.member.role', locale)}</th>
				<th class="bl-th" scope="col">{t('admin.member.status', locale)}</th>
				<th class="bl-th" scope="col">{t('admin.member.actions', locale)}</th>
			</tr>
		</thead>
		<tbody>
			{#each data.members as m (m.member_id)}
				<tr>
					<td class="bl-td">
						<a class="bl-link-inline" href={`/admin/members/${m.member_id}`}>{m.name}</a>
					</td>
					<td class="bl-td">
						{#each m.roles as r (r)}<span class="bl-badge">{t(roleKey(r), locale)}</span>{/each}
					</td>
					<td class="bl-td">{t(statusKey(m.onboarding_status), locale)}</td>
					<td class="bl-td">
						<MemberActionsMenu memberId={m.member_id} memberName={m.name} {locale} />
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
	</div>
{/if}

{#if $open}
	<div class="bl-overlay" use:melt={$overlay}></div>
	<div class="bl-dialog" use:melt={$content}>
		<h2 class="bl-h2" use:melt={$title}>{t('admin.members.add', locale)}</h2>

		{#if issued}
			<p class="bl-body-flush" use:melt={$description}>{t('admin.member.code_explainer', locale)}</p>
			<p class="bl-status" role="status" aria-live="polite">{t('admin.member.issued', locale)}</p>
			{#if issued.onboarding_code}
				<p class="bl-label mt-4">{t('admin.member.onboarding_code', locale)}</p>
				<p class="bl-code">{issued.onboarding_code}</p>
			{/if}
			<p class="bl-body-flush mt-2">
				{t('admin.member.code_expires', locale, { when: fmtExpiry(issued.code_expires_at) })}
			</p>
			<div class="bl-dialog-actions">
				<a class="bl-btn-secondary" href={`/admin/members/${issued.member.member_id}`}>
					{t('admin.member.view', locale)}
				</a>
				<button type="button" class="bl-btn" use:melt={$close}>{t('admin.member.cancel', locale)}</button>
			</div>
		{:else}
			<p class="bl-body-flush" use:melt={$description}>{t('admin.member.code_explainer', locale)}</p>
			<form method="POST" action="?/issue" use:enhance={submitIssue}>
				<div class="bl-field mt-4">
					<label class="bl-label" for="m-name">{t('admin.member.name', locale)}</label>
					<input class="bl-input" id="m-name" name="name" required autocomplete="off" />
				</div>
				<div class="bl-field mt-4">
					<label class="bl-label" for="m-phone">{t('admin.member.phone', locale)}</label>
					<input
						class="bl-input"
						id="m-phone"
						name="phone"
						type="tel"
						inputmode="tel"
						autocomplete="off"
						required
					/>
				</div>
				<div class="bl-field mt-4">
					<label class="bl-label" for="m-address">{t('admin.member.address', locale)}</label>
					<input class="bl-input" id="m-address" name="address" autocomplete="off" required />
				</div>
				<fieldset class="bl-field mt-4">
					<legend class="bl-label">{t('admin.member.role', locale)}</legend>
					<label class="inline-flex min-h-11 items-center gap-2">
						<input type="checkbox" name="roles" value="rider" /> {t('admin.member.role_rider', locale)}
					</label>
					<label class="inline-flex min-h-11 items-center gap-2">
						<input type="checkbox" name="roles" value="driver" /> {t('admin.member.role_driver', locale)}
					</label>
				</fieldset>

				{#if duplicate}
					<p class="bl-error" role="alert">
						{t('admin.member.duplicate_phone', locale)}
						<a class="bl-link-inline" href={`/admin/members/${duplicate.member_id}`}>
							{t('admin.member.duplicate_view', locale)}
						</a>
					</p>
				{:else if errorKey}
					<p class="bl-error" role="alert">{t(errorKey, locale)}</p>
				{/if}
				<p class="bl-status" role="status" aria-live="polite">
					{#if submitting}{t('admin.member.saving', locale)}{/if}
				</p>

				<div class="bl-dialog-actions">
					<button type="button" class="bl-btn-secondary" use:melt={$close}>{t('admin.member.cancel', locale)}</button>
					<button type="submit" class="bl-btn" disabled={submitting}>{t('admin.member.save', locale)}</button>
				</div>
			</form>
		{/if}
	</div>
{/if}
