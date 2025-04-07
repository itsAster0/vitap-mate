<script lang="ts">
  import Semid from "./semid.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Store } from "@tauri-apps/plugin-store";
  import { selsemid, loading } from "./store.svelte";
  import { getContext } from "svelte";
  import Days from "./days.svelte";
  import { goto } from "$app/navigation";

  let timetable_before: string | undefined = $state(undefined);
  let distime = $state(0);

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
      timetable_before = await store.get(`full_timetable_${selsemid.value}`);
      //console.log("sem id from storage",selsemid.value)
      let last_update: undefined | number = await store.get(
        `full_timetable_${selsemid.value}_lastupdate`,
      );
      distime = last_update;
    }
  }
  const time_diff_relaod = 10;
  function unixTimestamp() {
    return Math.floor(Date.now() / 1000);
  }

  async function gettimetable() {
    // console.log("in get timetable")

    if (selsemid.value == undefined) {
      return;
    }
    let sel_sem = selsemid.value;
    if (loading.value.includes(sel_sem)) {
      return;
    }
    loading.value.push(sel_sem);
    const store = await Store.load("timetable.json");
    let last_update: undefined | number = await store.get(
      `full_timetable_${sel_sem}_lastupdate`,
    );
    distime = last_update;
    if (
      (timetable_before == undefined ||
        last_update == undefined ||
        Math.abs(unixTimestamp() - last_update) > time_diff_relaod ||
        reload.status) &&
      errors.code != "stop"
    ) {
      reload.status = true;
      //@ts-ignore
      const [status, full_timetable_fetched] = await invoke("timetable", {
        semid: sel_sem,
      });
      reload.status = false;
      if (status && full_timetable_fetched != "") {
        if (errors.msg == "NE") {
          errors.msg = undefined;
        }
        const time = unixTimestamp();
        await store.set(`full_timetable_${sel_sem}_lastupdate`, time);
        distime = time;
        if (
          full_timetable_fetched != "" &&
          full_timetable_fetched != undefined &&
          full_timetable_fetched != timetable_before
        ) {
          await store.set(`full_timetable_${sel_sem}`, full_timetable_fetched);
          if (sel_sem == selsemid.value) {
            timetable_before = full_timetable_fetched;
          }
        }
      } else {
        if (full_timetable_fetched == "NE") {
          if (errors.code != "stop") {
            errors.code = "stop";
            errors.msg = "NE";
          }
          //triggerInfo("No Internet")
        } else {
          if (errors.code != "stop") {
            errors.code = "stop";
          }
          //triggerInfo(full_timetable1)
        }
      }
    }
    try {
      let index = loading.value.indexOf(sel_sem);
      if (index > -1) {
        loading.value.splice(index, 1);
      }
    } catch {}
  }
  $effect(() => {
    errors.code;
    //$inspect("tt",loading.value);
    if (selsemid.value != undefined) {
      (async () => {
        await loadfromstorage();
        await gettimetable();
      })();
    }
  });
</script>

<div class="">
  <div class="">
    <Semid />
    <div>
      {#if timetable_before != undefined}
        <Days full_timetable_list={timetable_before} updatedTime={distime} />
      {:else if errors.msg == "NE"}
        <p>Oops! No Internet Connection Detected</p>
      {:else if errors.msg == "NC"}
        <p>
          Please enter your credentials by navigating to the <button
            class=" link-primary"
            onclick={() => {
              goto("/settings/Credentials");
            }}>settings</button
          > menu.
        </p>
      {:else}
        <div class="skeleton h-[80vh] w-full"></div>
      {/if}
    </div>
  </div>
</div>
