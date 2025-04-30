<script lang="ts">
  import "../app.css";
  import { University, SquareMenu, Notebook, Menu, Cog } from "lucide-svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { setContext, untrack } from "svelte";
  import { Info } from "lucide-svelte";
  let { children } = $props();

  const currentPage = () => {
    let path = page.url.pathname;
    let n = path.split("/");
    return "/" + n[1];
  };

  function updatError() {
    $inspect(errors);
    setTimeout(() => {
      if (
        errors.code == "stop" &&
        errors.msg != "PC" &&
        errors.msg != "LI" &&
        errors.msg != "NC"
      ) {
        errors.code = undefined;
      }
    }, 2000);
  }

  let reload = $state({ status: false });
  let errors = $state({ code: undefined, msg: undefined });
  setContext("reload", reload);
  setContext("errors", errors);
  $effect(() => {
    errors.code;
    errors.msg;
    untrack(() => {
      updatError();
    });
  });

  //errors
  // undefined no errors
  // stop : stop all backend functions
  // loginfailed  : logined failed
  // (msg)
  //NE: no internet
  //VE : Vtop offline or  set code to stop
  //NC : useranme or password set code to stop
  //PC : password cahnged

  function tag(val) {
    let k = String(val).charAt(0).toUpperCase() + String(val).slice(1);
    let p = k.split("/");
    if (p.length > 2) {
      return p[0];
    } else {
      return k;
    }
  }
</script>

<div class="drawer">
  <input id="my-drawer" type="checkbox" class="drawer-toggle" />
  <div class="drawer-content bg-base-300 p-2">
    <!-- Improved Navbar -->
    <div class="navbar bg-base-100 shadow-sm rounded-box mb-4">
      <div class="navbar-start">
        <label for="my-drawer" class="btn btn-ghost btn-circle drawer-button lg:hidden">
          <Menu size={20} />
        </label>
        <a href="/timetable" class="btn btn-ghost normal-case text-xl px-3">VitapPal</a>
      </div>
      
      <div class="navbar-center">
        <a href="/timetable" class="font-medium text-xs">{tag(currentPage().replace("/", ""))}</a>
      </div>
      
      <div class="navbar-end gap-2">
        <div class="dropdown dropdown-end">
          <div tabindex="0" role="button" class="btn btn-ghost btn-circle">
            <Info size={20} />
          </div>
          <ul tabindex="0" role="menu" class="card compact dropdown-content z-[1] shadow bg-base-100 rounded-box w-64">
            <div class="card-body">
              <h2 class="card-title text-sm">Status Indicators</h2>
              <table class="table table-xs">
                <thead>
                  <tr>
                    <th>Icon</th>
                    <th>Spin</th>
                    <th>No Spin</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <th><Cog size={16} class="text-info" /></th>
                    <td>Logging in</td>
                    <td>-</td>
                  </tr>
                  <tr>
                    <th><Cog size={16} class="text-warning" /></th>
                    <td>Fetching data</td>
                    <td>No Internet</td>
                  </tr>
                  <tr>
                    <th><Cog size={16} class="text-success" /></th>
                    <td>-</td>
                    <td>Up to date</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </ul>
        </div>
        
        <button
          aria-label="settings"
          onclick={() => goto("/settings")}
          class="btn btn-ghost btn-circle"
        >
          <Cog
            size={20}
            class="{errors.code == undefined &&
            reload.status != true &&
            errors.msg == undefined
              ? 'text-success'
              : errors.code == 'stop' && errors.msg != 'NE'
                ? 'text-error'
                : 'text-warning'}  {reload.status == true
              ? 'animate-spin duration-1000 text-info '
              : ''} "
          />
        </button>
      </div>
    </div>

    <!-- Error messages -->
    <div class="w-full mb-2">
      {#if errors.msg == "NE"}
        <div class="alert alert-warning py-2 shadow-sm">
          <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" /></svg>
          <span>No Internet Connection. Data might be outdated</span>
        </div>
      {:else if errors.msg == "LI"}
        <div class="alert alert-info py-2 shadow-sm">
          <span class="loading loading-spinner text-info"></span>
          <span>Attempting to log in to VTOP</span>
        </div>
      {:else if reload.status == true}
        <div class="alert alert-info py-2 shadow-sm">
          <span class="loading loading-spinner text-warning"></span>
          <span>Updating data...</span>
        </div>
      {/if}
    </div>

    <!-- Page content -->
    <div>
      {@render children()}
    </div>
  </div>
  
  <!-- Drawer side -->
  <div class="drawer-side z-20">
    <label for="my-drawer" aria-label="close sidebar" class="drawer-overlay"></label>
    <ul class="menu p-6 w-64 min-h-full bg-base-200 text-base-content gap-2">
      <li class="menu-title font-bold text-lg mb-4">VitapPal</li>
      <li><a href="/timetable" class="text-base">Timetable</a></li>
      <li><a href="/attendance" class="text-base">Attendance</a></li>
      <li><a href="/utils" class="text-base">Utils</a></li>
      <li><a href="/settings" class="text-base">Settings</a></li>
    </ul>
  </div>
</div>

<!-- Dock remains outside the drawer -->
<div class="dock">
  <button
    onclick={() => goto("/timetable")}
    class={currentPage() != "/timetable" ? "" : "dock-active"}
  >
    <University />
    <span class="dock-label">Timetable</span>
  </button>

  <button
    onclick={() => goto("/attendance")}
    class={currentPage() != "/attendance" ? "" : "dock-active"}
  >
    <Notebook />
    <span class="dock-label">Attendance</span>
  </button>

  <button
    onclick={() => goto("/utils")}
    class={currentPage() != "/utils" ? "" : "dock-active"}
  >
    <SquareMenu />
    <span class="dock-label">Utils</span>
  </button>
</div>
