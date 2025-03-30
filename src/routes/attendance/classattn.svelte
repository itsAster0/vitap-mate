<script lang="ts">
  import { getContext } from "svelte";
  import { Store } from "@tauri-apps/plugin-store";
  import { invoke } from "@tauri-apps/api/core";
  import { selsemid } from "./store.svelte";
  import Analays from "./analysis.svelte";
  let { courseid, coursetype } = $props();

  const time_diff_relaod = 60;
  let full_attendance = $state(undefined);
  let details = $state(undefined);
  let privateload = $state(false);
  let lastUpdate = $state(0);
  let errors = getContext("errors");
  let data = $state("");

  async function loadData() {
    const store = await Store.load("attendance.json");
    let sel_sem = selsemid.value;
    if (sel_sem != undefined) {
      full_attendance = await store.get(
        `full_attendance_${courseid}_${coursetype}`,
      );
      if (full_attendance != undefined) {
        try {
          data = JSON.parse(full_attendance).slice(2);
        } catch {}
      }
      let last_update: number | undefined = await store.get(
        `full_attendance_${courseid}_${coursetype}_lastupdate`,
      );
      lastUpdate = last_update == undefined ? 0 : last_update;
      if (
        (full_attendance == undefined ||
          last_update == undefined ||
          Math.abs(unixTimestamp() - last_update) > time_diff_relaod ||
          privateload) &&
        errors.code != "stop"
      ) {
        privateload = true;

        //@ts-ignore
        const [status, full_attendance_cl] = await invoke("full_attendance", {
          semid: sel_sem,
          courseId: courseid,
          courseType: coursetype,
        });
        privateload = false;

        if (status && full_attendance_cl != "") {
          const time = unixTimestamp();
          await store.set(
            `full_attendance_${courseid}_${coursetype}_lastupdate`,
            time,
          );
          lastUpdate = time;
          if (
            full_attendance_cl != "" &&
            full_attendance_cl != undefined &&
            full_attendance_cl != full_attendance
          ) {
            await store.set(
              `full_attendance_${courseid}_${coursetype}`,
              full_attendance_cl,
            );
            await store.save();
            full_attendance = full_attendance_cl;
          }
        } else {
          if (full_attendance_cl == "NE") {
            //triggerInfo("No Internet")
          } else {
            //triggerInfo(full_attendance_cl)
          }
        }
      }
    }
    details = JSON.parse(full_attendance)[0];
    data = JSON.parse(full_attendance).slice(2);
    privateload = false;
  }

  function unixTimestamp() {
    return Math.floor(Date.now() / 1000);
  }

  $effect(() => {
    // console.log("effect from class attn");
    errors.code;
    loadData();
  });

  function unixToDate(k) {
    let n = k != undefined ? k : 0;
    let date = new Date(n * 1000);
    return date.toLocaleString("en-IN", {
      month: "short",
      day: "2-digit",
      hour: "numeric",
      minute: "2-digit",
    });
  }
</script>

{#if data != ""}
  <div>
    <div class=" flex flex-col gap-1 max-h-[38vh] overflow-auto">
      <div
        class="overflow-x-auto whitespace-nowrap rounded-box border border-base-content/5 bg-base-100"
      >
        <table class="table table-pin-rows table-zebra overflow-x-auto">
          <!-- head -->
          <thead>
            <tr>
              <th></th>
              <th>Date</th>
              <th>Status</th>
              <th>Time</th>
              <th>Slot</th>
              <th>Remark</th>
            </tr>
          </thead>
          <tbody>
            {#each data as row}
              <tr>
                <td>{row.serial}</td>
                <td>{row.date}</td>
                <td
                  ><div
                    class=" badge badge-soft {row.status == 'Present'
                      ? 'badge-success'
                      : row.status == 'Absent'
                        ? 'badge-error'
                        : 'badge-warning'}"
                  >
                    {row.status}
                  </div></td
                >

                <td>{row.day_time.replace("/", "")}</td>
                <td>{row.slot}</td>

                <td>{row.remark}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <div class=" text-center text-info">
        <p class="text-sm">
          Data updated on {unixToDate(lastUpdate)}
        </p>
      </div>
    </div>
  </div>
{:else}
  <div class="skeleton h-[30vh] w-full"></div>
{/if}
