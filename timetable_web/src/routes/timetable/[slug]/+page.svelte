<script lang="ts">
    import { page } from "$app/state";
    import Spinner from "$lib/components/ui/spinner/spinner.svelte";
    import Timetable from "$lib/Timetable.svelte";
    import { onMount } from "svelte";

    let loading = true;
    let error: string | null = null;
    let result: any = null;

    onMount(async () => {
        loading = true;

        try {
            const res = await fetch(
                "https://api.va.synaptic.gg/timetable/fetch",
                {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({
                        timetable_fetch_id: page.url.pathname
                            .split("/")
                            .reverse()[0],
                    }),
                },
            );

            if (!res.ok) {
                error = "Fetch failed: " + res.status;
            } else {
                result = await res.json();
            }
        } catch (e) {
            error = "Network error";
        }

        loading = false;
    });
</script>

{#if loading}
    <div class="flex flex-col items-center justify-center h-screen">
        <Spinner />
    </div>
{:else if error}
    <div class="flex flex-col items-center justify-center h-screen">
        <h1>Error: {error}</h1>
        <br />
        <p>Invalid Url / Url expired</p>
    </div>
{:else}
    <Timetable data={result} />
{/if}
