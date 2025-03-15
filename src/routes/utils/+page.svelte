    <script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { Store } from "@tauri-apps/plugin-store";
        import { onMount } from "svelte";
    

    let wifiusername = $state(undefined);
    let wifipassword = $state(undefined);
    let result = $state("");
    async function loadData() {
        let store = await Store.load("utils.json");
        wifiusername = await store.get("wifiusername");
         wifipassword = await  store.get("wifipassword");
    }




onMount(async () => {
    await loadData();
    console.log(wifipassword,wifiusername);
  });

    async function saveDate(){
        let store = await Store.load("utils.json");
        await store.set("wifiusername",wifiusername);
        await  store.set("wifipassword",wifipassword);
    }
    async function login(){
        await saveDate();
        console.log(1)
        let [status,k]= await invoke("wifi",{i:0,username:wifiusername,password:wifipassword});
        result = [status,k];
        console.log(result)


    }

    async function logout() {
        await saveDate();
        let [status,k]=  await invoke("wifi",{i:1,username:wifiusername,password:wifipassword});
        console.log(k);
        result = [status,k];
    }


    </script>



    <div class="flex flex-col ">
        <div class=" grow">
        <div class="card w-full max-w-md bg-base-100 p-6">
            <h2 class="text-center text-2xl font-semibold mb-6">wifi credentials</h2>
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
    <div>
        <p>{result}</p>
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
            <div>
                <p>
                    {result}
                </p>
            </div>
    
            <!-- Submit Button -->
            <div class="form-control mt-6 flex justify-evenly">

                    <button
                    type="submit"
                    class="btn btn-primary"
                    onclick={login}

                    >
        Login
                    </button>
                    <button
                    type="submit"
                    class="btn btn-primary"
                    onclick={logout}
                    >
                    
    Logout
                </button>


            </form>
        </div>
        </div>
    </div>