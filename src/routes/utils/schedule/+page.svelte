<script lang="ts">
  import Semid from "./semid.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Store } from "@tauri-apps/plugin-store";
  import { selsemid, loading } from "./store.svelte";
  import { getContext } from "svelte";
  import Schedule from "./schedule.svelte";

  let examschedule_before: string | undefined = $state(undefined);
  let distime = $state(0);
  let stat = $state(undefined);
  interface relaod {
    [key: string]: boolean;
  }
  interface datasate {
    [key: string]: string | undefined;
  }
  let reload: relaod = getContext("reload");
  let errors: datasate = getContext("errors");

  async function loadfromstorage() {
    const store = await Store.load("examschedule.json");
    if (selsemid.value != undefined) {
      examschedule_before = await store.get(
        `full_examschedule_${selsemid.value}`,
      );
      //console.log("sem id from storage",selsemid.value)
    }
  }
  const time_diff_relaod = 60;
  function unixTimestamp() {
    return Math.floor(Date.now() / 1000);
  }

  async function getexamschedule() {
    // console.log("in get examschedule")

    if (selsemid.value == undefined) {
      return;
    }
    let sel_sem = selsemid.value;
    if (loading.value.includes(sel_sem)) {
      return;
    }
    loading.value.push(sel_sem);
    const store = await Store.load("examschedule.json");
    let last_update: undefined | number = await store.get(
      `full_examschedule_${sel_sem}_lastupdate`,
    );
    distime = last_update;
    if (
      (examschedule_before == undefined ||
        last_update == undefined ||
        Math.abs(unixTimestamp() - last_update) > time_diff_relaod ||
        reload.status) &&
      errors.code != "stop"
    ) {
      reload.status = true;
      //@ts-ignore
      const [status, full_examschedule_fetched] = await invoke("exam_shedule", {
        semid: sel_sem,
      });
      //console.log(full_examschedule_fetched);
      stat = status;
      reload.status = false;
      if (status && full_examschedule_fetched != "") {
        const time = unixTimestamp();
        await store.set(`full_examschedule_${sel_sem}_lastupdate`, time);
        distime = time;
        if (
          full_examschedule_fetched != "" &&
          full_examschedule_fetched != undefined &&
          full_examschedule_fetched != examschedule_before
        ) {
          await store.set(
            `full_examschedule_${sel_sem}`,
            full_examschedule_fetched,
          );
          if (sel_sem == selsemid.value) {
            examschedule_before = full_examschedule_fetched;
          }
        }
      } else {
        if (full_examschedule_fetched == "NE") {
          //triggerInfo("No Internet")
        } else {
          //triggerInfo(full_examschedule1)
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
        await getexamschedule();
      })();
    }
  });
</script>

<div class="">
  <div class="">
    <Semid />
    <div>
      {#if examschedule_before != undefined}
        <Schedule examshedule={examschedule_before} updatedtime={distime} />
      {:else if stat == false}
        No schedule this sem
      {:else}
        <div class="skeleton h-[80vh] w-full"></div>
      {/if}
    </div>
  </div>
</div>
