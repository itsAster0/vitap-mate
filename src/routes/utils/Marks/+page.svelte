<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Semid from "./semid.svelte";
  import { Store } from "@tauri-apps/plugin-store";
  import { selsemid, loading } from "./store.svelte";
  import { getContext } from "svelte";
  import Marks from "./marks.svelte";

  let distime: undefined | number = $state(0);

  let marks = $state(undefined);
  async function loadfromstorage() {
    const store = await Store.load("marks.json");
    if (selsemid.value != undefined) {
      marks = await store.get(`marks_${selsemid.value}`);
      //console.log("sem id from storage",selsemid.value)
    }
  }
  let loaded = $state(false);

  interface relaod {
    [key: string]: boolean;
  }
  interface datasate {
    [key: string]: string | undefined;
  }
  let reload: relaod = getContext("reload");
  let errors: datasate = getContext("errors");

  const time_diff_relaod = 60;
  function unixTimestamp() {
    return Math.floor(Date.now() / 1000);
  }

  async function getmarks() {
    console.log("runing");
    // console.log("in get attendance")
    if (selsemid.value == undefined) {
      return;
    }
    let sel_sem = selsemid.value;
    if (loading.value.includes(sel_sem)) {
      return;
    }
    loading.value.push(sel_sem);
    const store = await Store.load("marks.json");
    let last_update: undefined | number = await store.get(
      `marks_${sel_sem}_lastupdate`,
    );
    distime = last_update;
    if (
      (marks == undefined ||
        last_update == undefined ||
        Math.abs(unixTimestamp() - last_update) > time_diff_relaod ||
        reload.status) &&
      errors.code != "stop"
    ) {
      reload.status = true;

      //@ts-ignore
      const [status, marks_fetched] = await invoke("marks_list", {
        semid: sel_sem,
      });
      reload.status = false;
      loaded = true;
      if (status && marks_fetched != "") {
        const time = unixTimestamp();
        await store.set(`marks_${sel_sem}_lastupdate`, time);
        distime = time;
        if (
          marks_fetched != "" &&
          marks_fetched != undefined &&
          marks_fetched != marks
        ) {
          await store.set(`marks_${sel_sem}`, marks_fetched);
          await store.save();
          if (sel_sem == selsemid.value) {
            marks = marks_fetched;
          }
        }
      } else {
        if (marks_fetched == "NE") {
          //triggerInfo("No Internet")
        } else {
          //triggerInfo(full_attendance1)
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
    selsemid.value;
    if (selsemid.value != undefined) {
      (async () => {
        await loadfromstorage();
        await getmarks();
      })();
    }
  });
</script>

<div>
  <Semid />
</div>
<div>
  {#if marks != undefined}
    <Marks marksList={marks} updatedTime={distime} />
  {:else if loaded}
    <p>No marks to show</p>
  {:else}
    <div class="skeleton h-auto w-full"></div>
  {/if}
</div>
