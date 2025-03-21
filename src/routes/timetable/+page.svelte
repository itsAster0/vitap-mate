<script lang="ts">
  import Semid from "./semid.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Store } from "@tauri-apps/plugin-store";
  import { selsemid } from "./store.svelte";
  import { getContext, untrack } from "svelte";
  import Days from "./days.svelte";

  let timetable_before: string | undefined = $state(undefined);
  let lastUpdate: number | undefined = $state(undefined);

  interface relaod {
    [key: string]: boolean;
  }
  interface datasate {
    [key: string]: string | undefined;
  }
  let reload: relaod = getContext("reload");
  let errors: datasate = getContext("errors");

  async function loadfromstorage() {
    const store = await Store.load("timetable.json");
    if (selsemid.value != undefined) {
      lastUpdate = await store.get(
        `full_timetable_${selsemid.value}_lastupdate`,
      );
      timetable_before = await store.get(`full_timetable_${selsemid.value}`);
      //console.log("sem id from storage",selsemid.value)
    }
  }
  const time_diff_relaod = 60;
  function unixTimestamp() {
    return Math.floor(Date.now() / 1000);
  }

  async function gettimetable() {
    // console.log("in get timetable")
    if (selsemid.value == undefined) {
      return;
    }
    if (
      (timetable_before == undefined ||
        lastUpdate == undefined ||
        Math.abs(unixTimestamp() - lastUpdate) > time_diff_relaod ||
        reload.status) &&
      errors.code != "stop"
    ) {
      const store = await Store.load("timetable.json");
      reload.status = true;
      let sel_sem = selsemid.value;
      //@ts-ignore
      const [status, full_timetable_fetched] = await invoke("timetable", {
        semid: sel_sem,
      });
      reload.status = false;
      if (status && full_timetable_fetched != "") {
        const time = unixTimestamp();
        await store.set(`full_timetable_${sel_sem}_lastupdate`, time);
        lastUpdate = time;
        if (
          full_timetable_fetched != "" &&
          full_timetable_fetched != undefined &&
          full_timetable_fetched != timetable_before
        ) {
          await store.set(`full_timetable_${sel_sem}`, full_timetable_fetched);
          await store.save();
          if (sel_sem == selsemid.value) {
            timetable_before = full_timetable_fetched;
          }
        }
      } else {
        if (full_timetable_fetched == "NE") {
          //triggerInfo("No Internet")
        } else {
          //triggerInfo(full_timetable1)
        }
      }
    }
  }
  $effect(() => {
    reload.status;
    errors.code;
    if (selsemid.value != undefined) {
      loadfromstorage().then(() => gettimetable());
    }
  });
</script>

<div class="">
  <div class="">
    <Semid />
    <div>
      {#if timetable_before != undefined}
        <Days full_timetable_list={timetable_before} />
      {:else}
        <div class="skeleton h-[80vh] w-full"></div>
      {/if}
    </div>
  </div>
</div>
