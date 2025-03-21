<script lang="ts">
  import Semid from "./semid.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Store } from "@tauri-apps/plugin-store";
  import { selsemid } from "./store.svelte";
  import { getContext, untrack } from "svelte";
  import Smallatten from "./smallatten.svelte";

  let attendance_before: string | undefined = $state(undefined);
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
    const store = await Store.load("attendance.json");
    if (selsemid.value != undefined) {
      lastUpdate = await store.get(
        `full_attendance_${selsemid.value}_lastupdate`,
      );
      attendance_before = await store.get(`full_attendance_${selsemid.value}`);
      //console.log("sem id from storage",selsemid.value)
    }
  }
  const time_diff_relaod = 60;
  function unixTimestamp() {
    return Math.floor(Date.now() / 1000);
  }

  async function getattendance() {
    // console.log("in get attendance")
    if (selsemid.value == undefined) {
      return;
    }
    const store = await Store.load("attendance.json");
    let sel_sem = selsemid.value;
    let last_update : undefined | number = await store.get(
        `full_attendance_${sel_sem}_lastupdate`,
      );
    if (
      (attendance_before == undefined ||
      last_update == undefined ||
        Math.abs(unixTimestamp() - last_update) > time_diff_relaod ||
        reload.status) &&
      errors.code != "stop"
    ) {
      
      reload.status = true;
      
      //@ts-ignore
      const [status, full_attendance_fetched] = await invoke("attendance", {
        semid: sel_sem,
      });
      reload.status = false;
      if (status && full_attendance_fetched != "") {
        const time = unixTimestamp();
        await store.set(`full_attendance_${sel_sem}_lastupdate`, time);
        if (sel_sem == selsemid.value) {
          lastUpdate = time;
          }
        if (
          full_attendance_fetched != "" &&
          full_attendance_fetched != undefined &&
          full_attendance_fetched != attendance_before
        ) {
          await store.set(
            `full_attendance_${sel_sem}`,
            full_attendance_fetched,
          );
          await store.save();
          if (sel_sem == selsemid.value) {
            attendance_before = full_attendance_fetched;
          }
        }
      } else {
        if (full_attendance_fetched == "NE") {
          //triggerInfo("No Internet")
        } else {
          //triggerInfo(full_attendance1)
        }
      }
    }
  }
  $effect(() => {
    reload.status;
    errors.code;
    if (selsemid.value != undefined) {
      loadfromstorage().then(() => getattendance());
    }
  });
</script>

<div class="">
  <div class="">
    <div class="">
      <Semid />
      <div>
        {#if attendance_before != undefined}
          <Smallatten attendance={attendance_before} />
        {:else}
          <div class="skeleton h-[80vh] w-full"></div>
        {/if}
      </div>
    </div>
  </div>
</div>
