<script lang="ts">
  import Analysis from "./analysis.svelte";
  import Classattn from "./classattn.svelte";

  let { attendance, updatedTime } = $props();

  let render = $derived(JSON.parse(attendance));
  let time = $derived(updatedTime)

  //@ts-ignore
  function get_course(strk) {
    let stri = String(strk);
    let parts = stri.split(" - ");

    let firstPart = parts[0];
    let lastPart = parts[parts.length - 1];
    //console.log(firstPart, lastPart);
    return [firstPart, lastPart];
  }
  //@ts-ignore
  function parseval(n) {
    let k = parseInt(String(n).replace("%", ""));
    if (Number.isNaN(k)) {
      return 0;
    }
    return k;
  }

  function getRingColor(percentage) {
    if (percentage < 75) {
      return "text-error";
    } else {
      return "text-success";
    }
  }

  let group = $state("");
  $effect(() => {
    //console.log(group);
  });

  function padNumbers(num1, num2) {
    const maxLength = Math.max(num1.length, num2.length);

    const paddedStr1 = num1.padStart(maxLength, "0");
    const paddedStr2 = num2.padStart(maxLength, "0");

    return [paddedStr1, paddedStr2];
  }

function unixToDate(k){
  let n = k !=undefined?k:0;
  let date = new Date(n * 1000);
  return date.toLocaleString( "en-IN",{
  year: '2-digit',
  month: 'short',
  day: '2-digit',
  hour: 'numeric',
  minute: '2-digit',
  second: '2-digit'
});
}

</script>

<div class="flex flex-col gap-4 overflow-auto min-h-[85vh]">
  {#each render as cl}
    <div class="flex flex-col">
      <div class="collapse collapse-plus bg-base-100 grow">
        <input
          type="radio"
          value={cl.course_name}
          name="my-accordion-3"
          bind:group
          onclick={() => {
            if (group === cl.course_name) {
              group = null;
            } else {
              group = cl.course_name;
            }
          }}
        />
        <div class="collapse-title font-semibold">
          <div>
            <div class="card w-full bg-base-100 card-xs">
              <div class="card-body">
                <div class="flex">
                  <div class="grow">
                    <div class="flex flex-col gap-0">
                      <div class="flex gap-2">
                        <div class="text-4xl font-thin tabular-nums">
                          <div
                            class="status animate-bounce {!get_course(
                              cl.course_name,
                            )[1].endsWith('Lab')
                              ? 'status-primary'
                              : 'status-secondary'}"
                          ></div>
                        </div>
                        <h2 class="card-title">
                          {get_course(cl.course_name)[0]}
                        </h2>
                      </div>

                      <div class="">
                        <ul class="flex flex-col gap-1">
                          <li>
                            <div class="flex flex-col">
                              <p>{cl.faculty_detail.split("-")[0]}</p>
                            </div>
                          </li>
                          {#if String(cl.debar_status) != "-"}
                            <li>
                              <div class="flex flex-col">
                                <p>{cl.debar_status}</p>
                              </div>
                            </li>
                          {/if}
                        </ul>
                      </div>
                    </div>
                  </div>

                  <div class=" flex gap-3 right-0 inset-y-0">
                    <div class="flex flex-col">
                      <div
                        class="radial-progress {getRingColor(
                          parseval(cl.attendance_percentage),
                        )}"
                        style="--value:{parseval(
                          cl.attendance_percentage,
                        )}; ; --size:4rem"
                        aria-valuenow={parseval(cl.attendance_percentage)}
                        role="progressbar"
                      >
                        {parseval(cl.attendance_percentage)}%
                      </div>
                      <strong class=" text-center">Total</strong>
                    </div>

                    <div class="flex flex-col">
                      <div
                        class="radial-progress {getRingColor(
                          parseval(cl.attendence_fat_cat),
                        )}"
                        style="--value:{parseval(
                          cl.attendence_fat_cat,
                        )};  --size:4rem"
                        aria-valuenow={parseval(cl.attendence_fat_cat)}
                        role="progressbar"
                      >
                        {parseval(cl.attendence_fat_cat)}%
                      </div>
                      <strong class="text-center">BetweenExams</strong>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="collapse-content text-sm overflow-auto">
          <div class=" overflow-x-auto">
            {#if cl.course_name == group}
              <div>
                <p
                  class=" text-center {!get_course(cl.course_name)[1].endsWith(
                    'Lab',
                  )
                    ? 'text-primary'
                    : 'text-secondary'} whitespace-nowrap overflow-auto"
                >
                  {cl.course_name.split("-")[1]}
                </p>
              </div>
              <Classattn
                courseid={String(cl.course_id)}
                coursetype={String(cl.course_type)}
              />
            {/if}
          </div>
        </div>
      </div>
      <div>
        <ul class="flex gap-1 justify-evenly">
          <li>
            <div class="flex text-sm">
              <p>TotalClasses&nbsp</p>
              <span>
                <kbd class="kbd">
                  {padNumbers(cl.total_classes, cl.classes_attended)[0]}
                </kbd></span
              >
            </div>
          </li>

          <li>
            <div class="flex text-sm">
              <p>AttendedClasses&nbsp</p>
              <span class="">
                <kbd class="kbd">
                  {padNumbers(cl.total_classes, cl.classes_attended)[1]}
                </kbd></span
              >
            </div>
          </li>
        </ul>
      </div>
    </div>
  {/each}
  <div class="collapse collapse-plus bg-base-100"></div>
  <div class="divider"><p class=" text-sm">Data updated at {unixToDate(time)} </p></div>
  <div class="w-full translate-y-[-3rem]">
    <footer class=" flex items-center justify-evenly gap-4 p-4">
      <small
        ><div class="status animate-bounce status-primary"></div>
        Therory</small
      >
      <small>
        <div class="status animate-bounce status-secondary"></div>
        Labs</small
      >
    </footer>
  </div>
</div>
