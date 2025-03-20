<script lang="ts">
  import { goto } from "$app/navigation";
  import { BookUp2 } from "lucide-svelte";

  function click() {
    goto("/utils/CoursePage");
  }
</script>

<div class="card w-full max-w-md bg-base-100 p-6 shadow-sm">
  <div class="flex">
    <span><BookUp2 /></span>
    <div class="flex flex-col grow">
      <h2 class="text-center text-2xl font-semibold grow translate-x-[-1.5vh]">
        CoursePage
      </h2>
    </div>
  </div>
  <div class="form-control mt-6 flex justify-evenly">
    <button class="btn btn-primary" onclick={click}> Open Coursepage </button>
  </div>
</div>
