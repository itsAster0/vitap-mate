<script lang="ts">
  import Wifi from "./wifi.svelte";
  import Coursepage from "./coursepage.svelte";
  import Marks from "./marks.svelte";
  import Examschedule from "./examschedule.svelte";
</script>

<div class="flex flex-col h-[80vh] overflow-auto">
  <div>
    <Marks />
  </div>
  <div>
    <Examschedule />
  </div>
  <div class="divider"></div>
  <div>
    <Wifi />
  </div>
  <div class="divider"></div>
  <div>
    <Coursepage />
  </div>
</div>
