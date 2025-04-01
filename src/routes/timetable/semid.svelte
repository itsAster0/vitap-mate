<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Store } from "@tauri-apps/plugin-store";
  import { selsemid } from "./store.svelte";
  import { getContext, onMount } from "svelte";
  import { untrack } from "svelte";
  let isCollapsed = $state(true);
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

  let semid: string | undefined = $state(undefined);
  let lastUpdate: number | undefined = $state(undefined);

  async function loadfromstorage() {
    const store = await Store.load("timetable.json");
    let selsem: string | undefined = await store.get("sel_timetable_semid");
    semid = await store.get("timetable_semid");
    selsemid.value = selsem;
    //console.log("from semid loadfrom storage",selsemid.value)
  }

  async function getsemids() {
    const store = await Store.load("timetable.json");
    let semids: string | undefined = await store.get("timetable_semid");
    lastUpdate = await store.get("timetable_semid_lastupdate");
    let check = semids == undefined ? true : false;
    if (
      (selsemid.value == undefined ||
        lastUpdate == undefined ||
        Math.abs(unixTimestamp() - lastUpdate) > time_diff_relaod ||
        reload.status) &&
      errors.code != "stop"
    ) {
      if (check) {
        reload.status = true;
      }
      //@ts-ignore
      let [status, semid_rs] = await invoke("timetable_semid");
      //("updating semids")
      if (check) {
        reload.status = false;
      }

      if (status) {
        let temp = JSON.parse(semid_rs);
        store.set("timetable_semid", temp);
        let time = unixTimestamp();
        await store.set("timetable_semid_lastupdate", time);
        await store.save();
        // console.log("updating timetable semids tie");
        if (temp != semid && temp != "") {
          semid = temp;
        }
      }
    }
  }
  //loadfromstorage().then(()=>{getsemids()})

  //@ts-ignore
  async function onClickHandel(id) {
    let selectedValue = id.target.value;

    const store = await Store.load("timetable.json");
    if (selectedValue != "" && selectedValue != undefined) {
      await store.set("sel_timetable_semid", selectedValue);
      await store.save();
      selsemid.value = selectedValue;
      //console.log("from semid",selsemid.value)
    }
  }

  let semname = $derived.by(() => {
    if (selsemid.value) {
    }
    if (semid == undefined) {
      getsemids();
      return "Loading";
    }
    if (selsemid.value == undefined) {
      return "Pick a semester";
    } else {
      if (semid == undefined) {
        getsemids();
        return "Loading";
      }
      for (let i = 0; semid.length > i; i++) {
        let k = semid[i].split(":");
        if (k[1] == selsemid.value) {
          return k[0];
        }
      }
    }
  });

  $effect(() => {
    if (reload.status | errors.code) {
    }
    //console.log("running effect from timetable semid");
    untrack(() => {
      loadfromstorage().then(() => getsemids());
    });
  });
  function toogleCollapsed() {
    isCollapsed = !isCollapsed;
  }
  function capitalizeFirstLetter(val) {
    try {
      var splitStr = val.toLowerCase().split(" ");
      for (var i = 0; i < splitStr.length; i++) {
        splitStr[i] =
          splitStr[i].charAt(0).toUpperCase() + splitStr[i].substring(1);
      }
      return splitStr.join(" ");
    } catch {
      return val;
    }
  }
</script>

<div class=" overflow-auto px-1 max-h-[24vh]">
  <div
    tabindex="0"
    role="button"
    class="collapse collapse-arrow bg-base-100 border-base-300 border {isCollapsed
      ? 'collapse-close'
      : 'collapse-open'}"
  >
    <input type="checkbox" onclick={toogleCollapsed} />
    <div class="collapse-title font-semibold">
      {capitalizeFirstLetter(semname)}
    </div>
    <div class="collapse-content text-sm">
      <div class="flex flex-col gap-1 overflow-auto">
        {#each semid as value}
          <div>
            <input
              type="radio"
              name="radio-1"
              class="radio"
              id={value.split(":")[1]}
              value={value.split(":")[1]}
              bind:group={selsemid.value}
              onclick={async (e) => {
                isCollapsed = !isCollapsed;
                await onClickHandel(e);
              }}
            />
            <label for={value.split(":")[1]}
              >{capitalizeFirstLetter(value.split(":")[0])}</label
            >
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>
