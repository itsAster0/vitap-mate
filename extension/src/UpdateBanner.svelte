<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { Release } from "./live";

  let {
    release,
    version,
    behind,
    onOpenUpdate,
  }: {
    release: Release | null;
    version: string;
    behind: boolean;
    onOpenUpdate: () => void;
  } = $props();
</script>

{#if behind && release}
  <div class="banner update">
    <div class="ic"><Icon name="download" size={17} /></div>
    <div class="grow">
      <div class="bt">
        {release.version !== version ? `Update ready: ${version} → ${release.version}` : "Update ready"}
      </div>
      {#if release.subject}<div class="bs">{release.subject}</div>{/if}
      <div class="acts">
        <button type="button" class="text-btn strong" onclick={onOpenUpdate}>Get the update</button>
      </div>
    </div>
  </div>
{/if}
