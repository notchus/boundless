<script lang="ts">
	// Admin-app chrome (spec 008 T10): a skip link, the product brand, and the primary nav (Members /
	// Audit log) with `aria-current="page"` on the active route. All copy from the catalog (P8). RTL is
	// handled by `dir` (hooks.server.ts) + Tailwind logical properties.
	import { page } from '$app/state';
	import type { Snippet } from 'svelte';

	import { t } from '$lib/i18n';
	import type { LayoutData } from './$types';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();
	const locale = $derived(data.locale);
	const path = $derived(page.url.pathname);
</script>

<div class="bl-app">
	<a href="#main" class="bl-skip">{t('admin.nav.skip', locale)}</a>
	<header class="bl-topbar">
		<span class="bl-brand">{t('admin.nav.brand', locale)}</span>
		<nav class="bl-nav" aria-label={t('admin.members.title', locale)}>
			<a
				class="bl-navlink"
				href="/admin/members"
				aria-current={path.startsWith('/admin/members') ? 'page' : undefined}
			>
				{t('admin.members.title', locale)}
			</a>
			<a
				class="bl-navlink"
				href="/admin/audit-log"
				aria-current={path === '/admin/audit-log' ? 'page' : undefined}
			>
				{t('admin.members.audit_log', locale)}
			</a>
		</nav>
	</header>
	<main id="main" class="bl-main">
		{@render children()}
	</main>
</div>
