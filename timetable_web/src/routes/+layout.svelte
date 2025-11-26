<script lang="ts">
    import "./layout.css";
    import favicon from "$lib/assets/favicon.svg";
    import { ModeWatcher } from "mode-watcher";
    let { children } = $props();
</script>

<svelte:head>
    <script async defer src="https://umami.synaptic.gg/script.js" data-website-id="2153fcd3-c2a3-41bc-a1de-15bc62dd20b5"></script>
    <link rel="icon" href={favicon} />
</svelte:head>
<ModeWatcher />
{@render children()}
