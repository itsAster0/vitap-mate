<script>
  import { invoke } from "@tauri-apps/api/core";
  import { getContext } from "svelte";
  import {
    selsemid,
    selclass,
    classes,
    classids,
    isCollapsed1,
    isCollapsed,
  } from "./store.svelte";
  import { Store } from "@tauri-apps/plugin-store";
  import { View } from "lucide-svelte";
  import { goto } from "$app/navigation";

  let semid = $state(undefined);
  let errors = getContext("errors");

  async function get_page() {
    let [status, k] = await invoke("coursepage");
    if (status) {
      semid = JSON.parse(k);
      let store = await Store.load("utils.json");
      let p = await store.get("coursepage_selsem");
      if (p != undefined) {
        selsemid.value = p;
        ///classids.value = undefined;
        //selclass.value = undefined;
        //classes.value = undefined;
        let [status, c] = await invoke("coursepage_courses", {
          semid: selsemid.value,
        });
        classids.value = JSON.parse(c);
      }
    }
  }
  async function handelclick(id) {
    if (selsemid.value != id.target.value) {
      selsemid.value = id.target.value;
      //console.log(selsemid.value);
      classids.value = undefined;
      selclass.value = undefined;
      classes.value = undefined;
      let [status, k] = await invoke("coursepage_courses", {
        semid: selsemid.value,
      });
      classids.value = JSON.parse(k);
      if (status) {
        let store = await Store.load("utils.json");
        store.set("coursepage_selsem", selsemid.value);
        classids.value = JSON.parse(k);
      }
    }
  }

  async function classselhandel(e) {
    if (selclass.value != e.target.value) {
      let p = e.target.value;
      selclass.value = p;
      classes.value = undefined;
      //console.log(selclass.value);
      let [status, k] = await invoke("coursepage_classes", {
        semid: selsemid.value,
        classid: selclass.value,
      });
      classes.value = JSON.parse(k);
      if (status) {
        classes.value = JSON.parse(k);
      }
    }
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

  $effect(() => {
    errors.code;
    console.log("inside effect");
    get_page();
  });

  let semname = $derived.by(() => {
    if (selsemid.value) {
    }
    semid;
    if (semid == undefined) {
      return "Loading";
    }
    if (selsemid.value == undefined) {
      return "Pick a semester";
    } else {
      for (let i = 0; semid.length > i; i++) {
        let k = semid[i].split(":");
        if (k[1] == selsemid.value) {
          return k[0];
        }
      }
    }
  });
  let classname = $derived.by(() => {
    if (classids.value == undefined) {
      return "Loading";
    } else if (classids.value == "") {
      return "No classes  this sem";
    }
    if (selclass.value == undefined) {
      return "Pick a class";
    } else {
      for (let i = 0; classids.value.length > i; i++) {
        let k = classids.value[i].split(":");
        if (k[1] == selclass.value) {
          return k[0];
        }
      }
    }
  });
  function toogleCollapsed() {
    isCollapsed.value = !isCollapsed.value;
  }
  function toogleCollapsed1() {
    isCollapsed1.value = !isCollapsed1.value;
  }
  function formateClass(name) {
    try {
      let k = name.split("-");
      if (k.length < 2) {
        return name;
      }
      return k[0] + "-" + k[2] + "-" + k[1];
    } catch {
      return name;
    }
  }
</script>

<div class="px-2 py-3 max-h-[24vh] overflow-auto">
  <div
    tabindex="0"
    role="button"
    class="collapse collapse-arrow bg-base-100 border-base-300 border rounded-lg shadow-sm {isCollapsed.value
      ? 'collapse-close'
      : 'collapse-open'}"
  >
    <input type="checkbox" onclick={toogleCollapsed} />
    <div class="collapse-title font-semibold flex items-center">
      <span class="mr-2">📚</span>
      {capitalizeFirstLetter(semname)}
    </div>
    <div class="collapse-content text-sm">
      <div class="flex flex-col gap-2 py-2">
        {#each semid as value}
          <div class="hover:bg-base-200 rounded-md px-2 py-1 transition-all duration-200">
            <input
              type="radio"
              name="radio-1"
              class="radio radio-sm radio-primary mr-2"
              id={value.split(":")[1]}
              value={value.split(":")[1]}
              onclick={async (e) => {
                toogleCollapsed();
                await handelclick(e);
              }}
              bind:group={selsemid.value}
            />
            <label for={value.split(":")[1]} class="cursor-pointer">
              {capitalizeFirstLetter(value.split(":")[0])}
            </label>
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<div>
  {#if selsemid.value != undefined}
    <div class="px-2 py-3 max-h-[24vh] overflow-auto">
      <div
        tabindex="0"
        role="button"
        class="collapse collapse-arrow bg-base-100 border-base-300 border rounded-lg shadow-sm mt-2 {isCollapsed1.value
          ? 'collapse-close'
          : 'collapse-open'}"
      >
        <input type="checkbox" onclick={toogleCollapsed1} />
        <div class="collapse-title font-semibold flex items-center">
          <span class="mr-2">📘</span>
          {formateClass(classname)}
        </div>
        <div class="collapse-content text-sm">
          <div class="flex flex-col gap-2 py-2">
            {#each classids.value as value}
              <div class="hover:bg-base-200 rounded-md px-2 py-1 transition-all duration-200">
                <input
                  type="radio"
                  name="radio-2"
                  class="radio radio-sm radio-primary mr-2"
                  id={value.split(":")[1]}
                  value={value.split(":")[1]}
                  onclick={async (e) => {
                    toogleCollapsed1();
                    await classselhandel(e);
                  }}
                  bind:group={selclass.value}
                />
                <label class="cursor-pointer" for={value.split(":")[1]}>
                  {formateClass(value.split(":")[0])}
                </label>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

{#if classes.value != undefined}
  <div class="overflow-auto mt-4">
    <table class="table table-pin-cols">
      <thead>
        <tr>
          <th></th>
          <th>course Name</th>
          <th>course Code</th>
          <th>Faculty</th>
          <th>Slot</th>
          <th>View</th>
        </tr>
      </thead>
      <tbody>
        {#each classes.value as cl}
          <tr>
            <th>{cl.serial}</th>
            <td>{cl.course_name}</td>
            <td>{cl.course_code}</td>
            <td>{cl.faculty_detail}</td>
            <td>{cl.slot}</td>
            <th>
              <button
                class="btn btn-sm btn-ghost btn-circle"
                onclick={() => {
                  goto("/utils/CoursePage/" + cl.erpid + ":" + cl.class_id);
                }}><View /></button>
            </th>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
