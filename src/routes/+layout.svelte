<script lang="ts">
  import "../app.css";
  import { University, SquareMenu, Notebook, Menu, Cog } from "lucide-svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { setContext } from "svelte";
  let { children } = $props();

  const currentPage = () => {
    return page.url.pathname;
  };

  let reload = $state({ status: false });
  let errors = $state({ code: undefined, msg: undefined });
  setContext("reload", reload);
  setContext("errors", errors);

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
      return p[0] + "/" + p[1];
    } else {
      return k;
    }
  }
</script>

<div class="navbar bg-base-100 shadow-sm">
  <div class="navbar-start">
    <div class="dropdown">
      <div tabindex="0" role="button" class="btn btn-ghost lg:hidden">
        <Menu />
      </div>
      <ul
        class="menu menu-sm dropdown-content bg-base-100 rounded-box z-1 mt-3 w-auto p-2 shadow"
      >
        <li><a href="/timetable">Timetable</a></li>
        <li><a href="/attendance">Attendance</a></li>
        <li><a href="/utils">utils</a></li>
        <li><a href="/settings">settings</a></li>
      </ul>
    </div>
    <div class="navbar-center btn btn-ghost text-xl">VitapPal</div>
  </div>
  <a href="/timetable" class="">{tag(currentPage().replace("/", ""))} </a>
  <div class="navbar-end">
    <button
      aria-label="settings"
      onclick={() => goto("/settings")}
      class="btn btn-ghost btn-circle"
    >
      <Cog
        class="{errors.code == undefined && reload.status != true
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

<div>
  {@render children()}
</div>

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
