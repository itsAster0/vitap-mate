<script lang="ts">
  import { goto } from "$app/navigation";
  import { creds } from "../globalstate.svelte";
  import { invoke } from "@tauri-apps/api/core";

  async function click() {
    let k = await invoke("plugin:nativeapi|ping", { payload: {} });
    console.log(k.value);
  }
</script>

<div class="min-h-[85vh] flex flex-col">
  <div class="grow">
    <div class="card w-full max-w-md bg-base-100 p-6 shadow-sm">
      <div class="card-body">
        <div class=" card-title">Vtop Credentials</div>
        <div>
          <p><strong>Useranme :</strong> {(() => creds.username)()}</p>
          <p>
            <strong>password :</strong>
            {(() => "*".repeat(creds.password?.length))()}
          </p>
        </div>
      </div>
      <div class=" card-actions justify-end">
        <button
          class=" btn btn-primary"
          onclick={() => goto("/settings/Credentials")}>Edit</button
        >
      </div>
    </div>
  </div>
  <!-- <div>
    <button class="btn btn-accent" onclick={click}>test</button>
  </div>-->
  <div class="w-full">
    <footer class=" flex items-center justify-between gap-4 p-4">
      <small class="opacity-70"
        >Made By <a
          href="https://discordapp.com/users/1147202428404519013"
          class="link-primary"
          target="_blank">Synaptic</a
        ></small
      >
      <small class="opacity-70"
        > <a
          href="https://linktr.ee/synapticsurge"
          class="link-primary"
          target="_blank">Conatct me</a
        >
        </small
      >
    </footer>
  </div>
</div>
