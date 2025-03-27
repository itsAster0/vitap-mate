<script lang="ts">
  import { goto } from "$app/navigation";
  import { BookUp2, DotSquare } from "lucide-svelte";
  import { getContext } from "svelte";

  let errors = getContext("errors");
  let disabled = $state(true);
  $effect(() => {
    errors.code;
    if (errors.code != "stop") {
      if (disabled) {
        disabled = false;
      }
    }
  });

  function click() {
    goto("/utils/CoursePage");
  }
</script>

<div class="card w-full max-w-md bg-base-100 p-6 shadow-sm">
  <div class="flex">
    <span><BookUp2 /></span>
    <div class="flex flex-col grow">
      <div class="flex flex-col items-center">
        <h2
          class="text-center text-2xl font-semibold grow translate-x-[-1.5vh]"
        >
          CoursePage
        </h2>
        <span class="translate-x-[-1.5vh]">(beta)</span>
      </div>
    </div>
  </div>

  <div class="justify-end card-actions">
    <button class="btn btn-primary" onclick={click} {disabled}>
      Open Coursepage
    </button>
  </div>
</div>
