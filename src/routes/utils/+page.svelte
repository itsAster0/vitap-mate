<script lang="ts">
  import Wifi from "./wifi.svelte";
  import Coursepage from "./coursepage.svelte";
  import Marks from "./marks.svelte";
  import Examschedule from "./examschedule.svelte";
  import HostelWifi from "./hostelwifi.svelte";
</script>

<div class="flex flex-col h-[80vh] overflow-auto px-2 py-4 gap-3">
  <div class="bg-base-100 rounded-lg shadow-sm p-4">
    <Marks />
  </div>

  <div class="bg-base-100 rounded-lg shadow-sm p-4">
    <Examschedule />
  </div>

  <div class="bg-base-100 rounded-lg shadow-sm p-4">
    <Wifi />
  </div>

  <div class="bg-base-100 rounded-lg shadow-sm p-4">
    <HostelWifi />
  </div>

  <div class="bg-base-100 rounded-lg shadow-sm p-4">
    <Coursepage />
  </div>
</div>
