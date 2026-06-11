<script lang="ts">
	// Audit-log view (spec 008 T10 / AC9). A semantic table of PII-read records — FIELD NAMES only,
	// never values. Wrapped in an `aria-live` region so its loaded/empty state is announced. All copy
	// from the catalog (P8).
	import { t, type MessageKey } from '$lib/i18n';
	import type { AuditField } from '$lib/members-types';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale);

	const fieldKey = (f: AuditField): MessageKey => `admin.member.${f}` as MessageKey;
	const fmtWhen = (epochSecs: number): string =>
		new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(epochSecs * 1000));
</script>

<svelte:head><title>{t('admin.members.audit_log', locale)}</title></svelte:head>

<h1 class="bl-h1">{t('admin.members.audit_log', locale)}</h1>
<p class="bl-body">{t('admin.audit.explainer', locale)}</p>

<div aria-live="polite">
	{#if data.entries.length === 0}
		<p class="bl-status" role="status">{t('admin.audit.empty', locale)}</p>
	{:else}
		<!-- Contain the wide-uuid table's horizontal scroll so the page never scrolls 2D at 320px (WCAG 1.4.10). -->
		<div class="overflow-x-auto">
			<table class="bl-table">
				<caption class="sr-only">{t('admin.members.audit_log', locale)}</caption>
			<thead>
				<tr>
					<th class="bl-th" scope="col">{t('admin.audit.when', locale)}</th>
					<th class="bl-th" scope="col">{t('admin.audit.admin', locale)}</th>
					<th class="bl-th" scope="col">{t('admin.audit.member', locale)}</th>
					<th class="bl-th" scope="col">{t('admin.audit.fields', locale)}</th>
					<th class="bl-th" scope="col">{t('admin.audit.request', locale)}</th>
				</tr>
			</thead>
			<tbody>
				{#each data.entries as e (e.request_id)}
					<tr>
						<td class="bl-td">{fmtWhen(e.timestamp)}</td>
						<td class="bl-td"><code>{e.admin_id}</code></td>
						<td class="bl-td">
							<a class="bl-link-inline" href={`/admin/members/${e.member_id}`}><code>{e.member_id}</code></a>
						</td>
						<td class="bl-td">
							{#each e.fields as f (f)}<span class="bl-badge">{t(fieldKey(f), locale)}</span>{/each}
						</td>
						<td class="bl-td"><code>{e.request_id}</code></td>
					</tr>
				{/each}
			</tbody>
			</table>
		</div>
	{/if}
</div>
