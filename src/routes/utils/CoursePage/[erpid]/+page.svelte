<script lang="ts">
  import { page } from "$app/state";
  import { invoke } from "@tauri-apps/api/core";
  import { selsemid } from "./../store.svelte";
  import { Download, ArrowDownToLine, Info } from "lucide-svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    isPermissionGranted,
    requestPermission,
    sendNotification,
  } from "@tauri-apps/plugin-notification";

  let tmp = page.params.erpid;
  let erpid = tmp.split(":")[0];
  let selclass = tmp.split(":")[1];
  let table1 = $state(undefined);
  let table2 = $state(undefined);
  let table3 = $state(undefined);
  //get_params_coursepage

  function parseDate(date) {
    let k = date.split("[");
    return k[0];
  }
  async function tableClick(p, btn) {
    btn.disabled = true;
    btn.innerText = "0%";

    let unlisten = await listen<number>(p, (event) => {
      btn.innerText = `${event.payload}%`;
    });

    let k: string = await invoke("download_coursepage", { url: p });
    let permissionGranted = await isPermissionGranted();
    unlisten();
    btn.innerText = "100%";
    setTimeout(() => {
      btn.disabled = false;
    }, 100);

    if (!permissionGranted) {
      const permission = await requestPermission();
      permissionGranted = permission === "granted";
    }

    if (permissionGranted) {
      sendNotification({ title: `Downloaded ${k}`, body: k });
    }
  }

  async function load_data() {
    let [state, k] = await invoke("coursepage_dlist", {
      semid: selsemid.value,
      classid: selclass,
      erpId: erpid,
    });
    if (state) {
      let p = JSON.parse(k);
      table1 = p[0].links;
      table2 = p[1].links;
      table3 = p.slice(2);
    }
  }
  load_data();
</script>

<div class="flex justify-evenly">
  {#each table1 as t1, i}
    {#if t1 != "0;"}
      {#if i == 0}
        <div class="flex flex-col items-center">
          <button
            class="btn btn-primary btn-sm w-24 items-center"
            onclick={async (e) => await tableClick(t1, e.currentTarget)}
            ><ArrowDownToLine /></button
          >
          <p class="mt-1 text-center">
            &nbsp;&nbsp; Download all &nbsp;&nbsp;&nbsp;
          </p>
        </div>
      {:else if i == 1}
        <div class="flex flex-col items-center">
          <button
            class="btn btn-primary btn-sm w-24 items-center"
            onclick={async (e) => await tableClick(t1, e.currentTarget)}
            ><ArrowDownToLine /></button
          >
          <p class="mt-1 text-center">Download material</p>
        </div>
      {:else if i == 2}
        <div class="flex flex-col items-center">
          <button
            class="btn btn-primary btn-sm w-24 items-center"
            onclick={async (e) => await tableClick(t1, e.currentTarget)}
            ><ArrowDownToLine /></button
          >
          <p class="mt-1 text-center">Download syllabus</p>
        </div>
      {/if}
    {/if}
  {/each}
</div>
<div>
  <div class="overflow-x-auto border border-base-content/5 bg-base-100">
    <table class="table overflow-x-auto whitespace-nowrap">
      <thead>
        <tr>
          <th>
            <div class="tooltip tooltip-bottom">
              <div class="tooltip-content translate-x-[18vh]">
                You can find downloaded files in the /Downloads folder of your
                internal storage.
              </div>
              <button><Info class=" size-5 translate-y-[0.5vh]" /></button>
            </div></th
          >
          <th>Date</th>
          <th>Day</th>
          <th>Topic</th>
          <th><Download /></th>
        </tr>
      </thead>
      <tbody>
        {#each table3 as t}
          <tr>
            <th>{t.serial}</th>
            <td>{parseDate(t.date)}<br />{t.day}</td>
            <td>{t.day}</td>
            <td>{t.topic}</td>
            <th>
              {#each t.links as t2, i}
                <button
                  onclick={async (e) => await tableClick(t2, e.currentTarget)}
                  ><ArrowDownToLine></ArrowDownToLine></button
                >
              {/each}
            </th>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
