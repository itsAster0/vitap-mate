<script lang="ts">
  import { page } from "$app/state";
  import { invoke } from "@tauri-apps/api/core";
  import { selsemid } from "./../store.svelte";
  import { Download, ArrowDownToLine } from "lucide-svelte";

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
  async function table1Click(p) {
    let k = await invoke("download_coursepage", { url: p });
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

<div class="flex w-full justify-evenly">
  {#each table1 as t1, i}
    {#if t1 != "0;"}
      {#if i == 0}
        <button
          class="btn btn-primary"
          onclick={async () => await table1Click(t1)}
          ><ArrowDownToLine />Download All</button
        >
      {:else if i == 1}
        <button
          class="btn btn-primary"
          onclick={async () => await table1Click(t1)}
          ><ArrowDownToLine /> Download General Material</button
        >
      {:else if i == 2}
        <button
          class="btn btn-primary"
          onclick={async () => await table1Click(t1)}
          ><ArrowDownToLine />Download Syllabus</button
        >
      {/if}
    {/if}
  {/each}
</div>
<div>
  <div class="overflow-x-auto border border-base-content/5 bg-base-100">
    <table class="table overflow-x-auto whitespace-nowrap">
      <thead>
        <tr>
          <th></th>
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
                <button onclick={async () => await table1Click(t2)}
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
