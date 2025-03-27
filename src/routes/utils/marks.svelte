<script lang="ts">
  import { goto } from "$app/navigation";
  import { BookPlus } from "lucide-svelte";

  function click() {
    goto("/utils/Marks");
  }
</script>

<div class="card w-full max-w-md bg-base-100 p-6 shadow-sm">
  <div class="flex">
    <span><BookPlus /></span>
    <div class="flex flex-col grow">
      <div class="flex flex-col items-center">
        <h2
          class="text-center text-2xl font-semibold grow translate-x-[-1.5vh]"
        >
          Marks
        </h2>
        <span class="translate-x-[-1.5vh]"></span>
      </div>
    </div>
  </div>
  <div class="justify-end card-actions">
    <button class="btn btn-primary" onclick={click}> Open MarksPage </button>
  </div>
</div>
