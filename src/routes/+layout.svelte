<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>DataForge Compute</title>
</svelte:head>

<div class="min-h-screen bg-[hsl(var(--background))] text-[hsl(var(--foreground))]">
	{@render children()}
</div>
