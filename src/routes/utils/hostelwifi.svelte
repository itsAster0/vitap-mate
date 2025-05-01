<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Store } from "@tauri-apps/plugin-store";
  import { onMount } from "svelte";
  import { Wifi } from "lucide-svelte";
  import { CircleCheck } from "lucide-svelte";
  import { CircleX } from "lucide-svelte";
  import { Pencil, Info } from "lucide-svelte";
  import { WifiOff } from "lucide-svelte";

  let hostelWifiUsername = $state(undefined);
  let hostelWifiPassword = $state(undefined);
  let button = $state(false);
  let status: undefined | boolean = $state(undefined);
  let result = $state("");
  
  async function loadData() {
    let store = await Store.load("utils.json");
    hostelWifiUsername = await store.get("hostelWifiUsername");
    hostelWifiPassword = await store.get("hostelWifiPassword");
  }
  
  let dialog;

  onMount(async () => {
    await loadData();
    usernamepass();
    dialog = document.getElementById("hostelWifiCreds");
  });

  function usernamepass() {
    if (
      hostelWifiPassword == "" ||
      hostelWifiUsername == "" ||
      hostelWifiUsername == undefined ||
      hostelWifiPassword == undefined
    ) {
      return false;
    } else {
      return true;
    }
  }

  const showDialogClick = () => {
    dialog["showModal"]();
  };

  const closeDialog = async () => {
    await saveData();
    dialog.close();
  };

  async function saveData() {
    let store = await Store.load("utils.json");
    await store.set("hostelWifiUsername", hostelWifiUsername);
    await store.set("hostelWifiPassword", hostelWifiPassword);
    await store.save();
  }
  
  async function login() {
    let check = usernamepass();
    if (check) {
      status = undefined;
      button = true;
      result = "";
      
      try {
        let [stat, k] = await invoke("hostel_wifi", {
          i: 0,
          username: hostelWifiUsername,
          password: hostelWifiPassword,
        });
        
        result = k;
        status = stat;
      } catch (error) {
        result = `Error during login: ${error}`;
        status = false;
      } finally {
        button = false;
      }
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
      
      try {
        let [stat, k] = await invoke("hostel_wifi", {
          i: 1,
          username: hostelWifiUsername,
          password: hostelWifiPassword,
        });
        
        result = k;
        status = stat;
      } catch (error) {
        result = `Error during logout: ${error}`;
        status = false;
      } finally {
        button = false;
      }
    } else {
      result = "ND";
    }
  }
  
  function parseResult(result) {
    let output = "..";
    if (status == true) {
      return result;
    }
    if (result.includes("Network error")) {
      output = "Looks like you aren't connected to hostel Wi-Fi";
    } else {
      output = result;
    }
    return output;
  }
</script>

<div class="flex flex-col">
  <div class="grow">
    <div class="card w-full max-w-md bg-base-100 p-6 shadow-sm">
      <div class="flex gap-4">
        <span><WifiOff /></span>

        <h2 class="text-center text-2xl font-semibold grow">
          Hostel Wi-Fi <div class="tooltip tooltip-top">
            <div class="tooltip-content translate-x-[-4vh]">
              Make sure you're connected to the hostel Wi-Fi network (172.18.8.8)
            </div>
            <button><Info class="size-5 translate-y-[0.5vh]" /></button>
          </div>
        </h2>
        <button class="" onclick={() => showDialogClick()}><Pencil /></button>
      </div>
      <p class="text-center">{hostelWifiUsername}</p>
      <div class="flex justify-center">
        <div>
          {#if button == true}
            <progress class="progress w-56"></progress>
          {/if}
        </div>
        <div class="flex gap-2">
          <div>
            {#if status == true}
              <div class="text-success">
                <CircleCheck />
              </div>
            {:else if status == false}
              <div class="text-error">
                <CircleX />
              </div>
            {/if}
          </div>
          <div>
            {#if result == "ND"}
              <div class="flex gap-2">
                Add your Hostel Wi-Fi details by clicking <button
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
        <button class="btn btn-secondary" onclick={logout} disabled={button}>
          Logout
        </button>
      </div>
    </div>
  </div>
</div>

<dialog id="hostelWifiCreds" class="modal">
  <div class="modal-box">
    <form method="dialog">
      <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
        >✕</button
      >
    </form>
    <div class="flex flex-col">
      <div class="grow">
        <div class="card w-full max-w-md bg-base-100 p-6">
          <h2 class="text-center text-2xl font-semibold mb-6">
            Hostel Wi-Fi Credentials
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
                bind:value={hostelWifiUsername}
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
                bind:value={hostelWifiPassword}
              />
            </div>
            
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
