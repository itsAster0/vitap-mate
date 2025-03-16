<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Store } from "@tauri-apps/plugin-store";
  import { onMount } from "svelte";
  import { Wifi } from "lucide-svelte";
  import { CircleCheck } from "lucide-svelte";
  import { CircleX } from "lucide-svelte";
  import { Pencil } from "lucide-svelte";

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
      <div class="flex">
        <span><Wifi /></span>
        <h2 class="text-center text-2xl font-semibold grow">Wi-Fi Utils</h2>
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
                Add your Wi-Fi details by clicking <Pencil />
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
                Save
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  </div>
</dialog>
