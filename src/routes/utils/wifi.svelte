<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Store } from "@tauri-apps/plugin-store";
  import { onMount } from "svelte";
  import { Wifi } from "lucide-svelte";
  import { CircleCheck } from "lucide-svelte";
  import { CircleX } from "lucide-svelte";
  import { Pencil, Info } from "lucide-svelte";

  let wifiusername = $state(undefined);
  let wifipassword = $state(undefined);
  let button = $state(false);
  let status: undefined | boolean = $state(undefined);

  let result = $state("");
  async function loadData() {
    let store = await Store.load("utils.json");
    wifiusername = await store.get("wifiusername");
    wifipassword = await store.get("wifipassword");
  }
  //@ts-ignore
  let dialog;

  onMount(async () => {
    await loadData();
    usernamepass();
    dialog = document.getElementById("wificreds");
  });

  function usernamepass() {
    if (
      wifipassword == "" ||
      wifiusername == "" ||
      wifiusername == undefined ||
      wifipassword == undefined
    ) {
      return false;
    } else {
      return true;
    }
  }

  const showDialogClick = () => {
    //@ts-ignore
    dialog["showModal"]();
  };

  const closeDialog = async () => {
    await saveDate();
    //@ts-ignore
    dialog.close();
  };

  async function saveDate() {
    let store = await Store.load("utils.json");
    await store.set("wifiusername", wifiusername);
    await store.set("wifipassword", wifipassword);
    await store.save();
  }
  async function login() {
    let check = usernamepass();
    if (check) {
      status = undefined;
      button = true;
      result = "";
      //@ts-ignore
      let [stat, k] = await invoke("wifi", {
        i: 0,
        username: wifiusername,
        password: wifipassword,
      });
      result = k;
      status = stat;
      button = false;
    } else {
      result = "ND";
    }
  }

  //¹²³⁴⁵⁶⁷⁸⁹⁰

  function convertToSuperstring(k: string) {
    let map = {
      "1": "¹",
      "2": "²",
      "3": "³",
      "5": "⁵",
      "6": "⁶",
      "7": "⁷",
      "8": "⁸",
      "9": "⁹",
      "0": "⁰",
      "4": "⁴",
    };

    let n = k.split("");
    let runs = Math.floor(Math.random() * n.length);
    while (runs == 0 || runs == 1) {
      runs = Math.floor(Math.random() * n.length);
    }
    for (let i = 0; i <= runs; i++) {
      let c = Math.floor(Math.random() * n.length);
      let k = Number(n[c]);
      if (!isNaN(k)) {
        n[c] = map[n[c]];
      }
    }
    return n.join("");
  }

  async function loginBypass() {
    let check = usernamepass();
    if (check) {
      status = undefined;
      button = true;
      result = "";
      let newwifiusername = convertToSuperstring(wifiusername);
      //@ts-ignore
      let [stat, k] = await invoke("wifi", {
        i: 0,
        username: newwifiusername,
        password: wifipassword,
      });
      result = k;
      status = stat;
      button = false;
    } else {
      result = "ND";
    }
  }
  async function logout() {
    let check = usernamepass();
    if (check) {
      status = undefined;
      button = true;
      result = "";
      //@ts-ignore
      let [stat, k] = await invoke("wifi", {
        i: 1,
        username: wifiusername,
        password: wifipassword,
      });
      result = k;
      status = stat;
      button = false;
    } else {
      result = "ND";
    }
  }
  //@ts-ignore
  function parseResult(result) {
    let output = "..";
    if (status == true) {
      return result;
    }
    if (result == "NE") {
      output = "Looks like you weren't on college Wi-Fi";
    } else if (result == "NL") {
      output = "Looks like you were not logged in";
    } else {
      output = result;
    }
    return output;
  }
</script>

<div class="flex flex-col">
  <div class=" grow">
    <div class="card w-full max-w-md bg-base-100 p-6 shadow-sm">
      <div class="flex gap-4">
        <span><Wifi /></span>

        <h2 class="text-center text-2xl font-semibold grow">
          Wi-Fi Utils <div class="tooltip tooltip-top">
            <div class="tooltip-content translate-x-[-4vh]">
              If login fails, disable mobile data and connect to the college
              Wi-Fi.
            </div>
            <button><Info class=" size-5 translate-y-[0.5vh]" /></button>
          </div>
        </h2>
        <button class="" onclick={() => showDialogClick()}><Pencil /></button>
      </div>
      <p class=" text-center">{wifiusername}</p>
      <div class="flex justify-center">
        <div>
          {#if button == true}
            <progress class="progress w-56"></progress>
          {/if}
        </div>
        <div class="flex gap-2">
          <div>
            {#if status == true}
              <div class=" text-success">
                <CircleCheck />
              </div>
            {:else if status == false}
              <div class=" text-error">
                <CircleX />
              </div>
            {/if}
          </div>
          <div>
            {#if result == "ND"}
              <div class="flex gap-2">
                Add your Wi-Fi details by clicking <button
                  class=""
                  onclick={() => showDialogClick()}><Pencil /></button
                >
              </div>
            {:else}
              {parseResult(result)}
            {/if}
          </div>
        </div>
      </div>

      <div class="form-control mt-6 flex justify-evenly">
        <button class="btn btn-primary" onclick={login} disabled={button}>
          Login
        </button>
        <button class="btn btn-primary" onclick={logout} disabled={button}>
          Logout
        </button>
        <div>
          <button class="btn btn-error" onclick={loginBypass} disabled={button}>
            Login
          </button>
          <div class="tooltip tooltip-top">
            <div class="tooltip-content translate-x-[-10vh]">
              Allows login even when over the limit
            </div>
            <button><Info class=" size-5 translate-y-[0.5vh]" /></button>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>

<dialog id="wificreds" class="modal">
  <div class="modal-box">
    <form method="dialog">
      <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
        >✕</button
      >
    </form>
    <div class="flex flex-col">
      <div class=" grow">
        <div class="card w-full max-w-md bg-base-100 p-6">
          <h2 class="text-center text-2xl font-semibold mb-6">
            Wi-Fi Credentials
          </h2>

          <form>
            <div class="form-control mb-4">
              <label for="username" class="label">
                <span class="label-text">Username</span>
              </label>
              <input
                type="text"
                id="username"
                placeholder="Enter your username"
                class="input input-bordered w-full"
                bind:value={wifiusername}
              />
            </div>

            <div class="form-control mb-4">
              <label for="password" class="label">
                <span class="label-text">Password</span>
              </label>
              <input
                type="password"
                id="password"
                placeholder="Enter your password"
                class="input input-bordered w-full"
                bind:value={wifipassword}
              />
            </div>
            <!-- Save Button -->
            <div class="form-control mt-6 flex justify-evenly">
              <button
                type="submit"
                class="btn btn-primary"
                onclick={closeDialog}
              >
                save
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  </div>
</dialog>
