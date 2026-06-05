<script lang="ts">
	// Signed-in placeholder (spec 001 T15). The admin dashboard itself is spec 008.

	import { t } from '$lib/i18n';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head><title>{t('admin.home.signed_in', data.locale)}</title></svelte:head>

<main class="bl-page">
	<div class="bl-card">
		<h1 class="bl-h1">{t('admin.home.signed_in', data.locale)}</h1>
	</div>
</main>
