<script lang="ts">
    import Spinner from "$lib/components/ui/spinner/spinner.svelte";
    import Timetable from "$lib/Timetable.svelte";
    import type { PageProps } from "./$types";
    let { data }: PageProps = $props();

    let error = data.error;
    let result = data["result"];
</script>

{#if error}
    <div class="flex flex-col items-center justify-center h-screen">
        <h1>Error: {error}</h1>
        <br />
        <p>Invalid Url / Url Expired</p>
    </div>
{:else}
    <Timetable data={result} />
{/if}
