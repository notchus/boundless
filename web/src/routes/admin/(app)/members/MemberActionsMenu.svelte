<script lang="ts">
	// Per-row actions menu (spec 008 T10). A melt-ui dropdown menu (roving focus, arrow keys, Esc,
	// type-ahead — WAI-ARIA) offering View + Edit for one member. The trigger's accessible name includes
	// the member name so the many per-row triggers are distinguishable to a screen reader (WCAG 2.5.3:
	// "Actions" is in the name). All copy from the catalog (P8).
	import { createDropdownMenu, melt } from '@melt-ui/svelte';

	import { t } from '$lib/i18n';

	let { memberId, memberName, locale }: { memberId: string; memberName: string; locale: string } =
		$props();

	const {
		elements: { trigger, menu, item },
		states: { open },
	} = createDropdownMenu({ positioning: { placement: 'bottom-end' }, loop: true });
</script>

<button
	type="button"
	class="bl-btn-secondary"
	use:melt={$trigger}
	aria-label={t('admin.member.actions_for', locale, { name: memberName })}
>
	{t('admin.member.actions', locale)}
</button>

{#if $open}
	<div class="bl-menu" use:melt={$menu}>
		<a class="bl-menuitem" href={`/admin/members/${memberId}`} use:melt={$item}>
			{t('admin.member.view', locale)}
		</a>
		<a class="bl-menuitem" href={`/admin/members/${memberId}?edit=1`} use:melt={$item}>
			{t('admin.member.edit', locale)}
		</a>
	</div>
{/if}
