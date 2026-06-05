<script lang="ts">
	import { onMount } from 'svelte';

	import '../app.css';

	let { children } = $props();

	// Deterministic hydration signal for e2e: interactive tests wait for `html[data-hydrated]` before
	// driving the UI, so a keyboard/click can't land before Svelte has wired its handlers. Harmless in
	// production (a single data attribute set once on mount).
	onMount(() => {
		document.documentElement.dataset.hydrated = 'true';
	});
</script>

{@render children()}
