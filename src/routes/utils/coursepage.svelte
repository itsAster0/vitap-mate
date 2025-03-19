<script lang="ts">
import { invoke } from "@tauri-apps/api/core";

    //AP2023241

    async function click() {
        let k = await invoke("coursepage_classes",{semid:"AP2024255",classid:"AP2024255000142"});
    }
</script>

<button class="btn"
onclick={click}
>
test
</button>