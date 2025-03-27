<script lang="ts">
  import Wifi from "./wifi.svelte";
  import Coursepage from "./coursepage.svelte";
  import Marks from "./marks.svelte";
</script>

<div class="flex flex-col">
  <div>
    <Coursepage />
  </div>
  <div>
    <Marks />
  </div>
  <div class="divider"></div>
  <div>
    <Wifi />
  </div>
</div>
