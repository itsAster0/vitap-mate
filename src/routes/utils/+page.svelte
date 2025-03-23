<script lang="ts">
  import Wifi from "./wifi.svelte";
  import Coursepage from "./coursepage.svelte";
  import { invoke } from "@tauri-apps/api/core";

async function test() {
  let  k = await invoke("marks_page")
}
</script>

<div class="flex flex-col">
  <div>
    <Coursepage />
  </div>
  <div class="divider">-</div>
  <div>
    <Wifi />
  </div>
</div>
<button class="btn btn-primary " onclick={test}>
  test
</button>
