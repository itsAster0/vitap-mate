<script lang="ts">
  import { creds } from "./globalstate.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Store } from "@tauri-apps/plugin-store";
  import { goto } from "$app/navigation";
  import { getContext, onMount } from "svelte";

  interface datasate {
    [key: string]: string | undefined;
  }

  let errors: datasate = getContext("errors");
  let relaod = getContext("reload");

  //settings/Credentials password to PC incidates password change

  async function loadCreds() {
    let store = await Store.load("credentials.json");
    creds.username = await store.get("username");
    creds.password = await store.get("password");
  }

  async function validateAndMove() {
    let store = await Store.load("credentials.json");
    if (creds.username == undefined && creds.password == undefined) {
      goto("/settings/Credentials");
      errors.code = "stop";
      errors.msg = "NC";
      //add toast : enter your username and password
      return;
    } else if (creds.username != undefined && creds.password == "PC") {
      errors.code = "stop";
      errors.msg = "PC";
      goto("/settings/Credentials");
      //add toast : update your password
      return;
    }
    goto("/timetable");
    for (let i = 0; i < 3; i++) {
      errors.code = "stop";
      relaod.status = true;
      //@ts-ignore
      const [status, kt] = await invoke("onstart_runcode", {
        username: creds.username,
        password: creds.password,
      });
      relaod.status = false;
      const k = String(kt);
      //console.log(k=="Invalid LoginId/Password",k == "Invalid Username/Password",k)
      if (
        k == "Invalid Username/Password" ||
        k == "Invalid LoginId/Password" ||
        k == "Number Of Maximum Fail Attempts Reached.Use Forgot Password"
      ) {
        await invoke("update_credtials_to", {
          username: "EE",
          password: "EE",
        });
        await store.set("password", "PC");
        errors.code = "stop";
        errors.msg = "PC";
        creds.password = "PC";
        goto("/settings/Credentials");
        return;
      } else if (k === "Invalid Captcha") {
        continue;
      } else if (k === "SS") {
        errors.msg = undefined;
        errors.code = undefined;
        //console.log("login ss");
        break;
      } else if (k == "VE") {
        errors.code = "stop";
        errors.msg = "VE";

        //add toast
        break;
      } else if (k == "NE") {
        errors.code = "stop";
        errors.msg = "NE";

        //add toast
        break;
      } else {
        if (status) {
          errors.code = undefined;
          break;
        }
      }
    }
    await store.save();
  }

  onMount(() => {
    (async () => {
      await loadCreds(), await validateAndMove();
    })();
  });
</script>
