<script lang="ts">
  let { marksList, updatedTime } = $props();
  let render = $derived(JSON.parse(marksList));
  let group = $state("");
  function unixToDate(k) {
    let n = k != undefined ? k : 0;
    let date = new Date(n * 1000);
    return date.toLocaleString("en-IN", {
      month: "short",
      day: "2-digit",
      hour: "numeric",
      minute: "2-digit",
    });
  }
</script>

<div class="flex flex-col gap-4 overflow-auto min-h-[85vh]">
  {#each render as cl}
    <div class="flex flex-col">
      <div class="collapse collapse-plus bg-base-100 grow">
        <input
          type="radio"
          value={cl.slot}
          name="my-accordion-3"
          bind:group
          onclick={() => {
            if (group === cl.slot) {
              group = null;
            } else {
              group = cl.slot;
            }
          }}
        />
        <div class="collapse-title font-semibold">
          <div>
            <div class="card w-full bg-base-100 card-xs">
              <div class="card-body">
                <div class="flex gap-4">
                  <div>
                    <div class="flex flex-col gap-0">
                      <div class="flex gap-2">
                        <div class="text-4xl font-thin tabular-nums">
                          <div
                            class="status animate-bounce {!cl.coursetype.endsWith(
                              'Lab',
                            )
                              ? 'status-primary'
                              : 'status-secondary'}"
                          ></div>
                        </div>
                        <h2 class="card-title">
                          {cl.coursecode}
                        </h2>
                      </div>
                    </div>
                  </div>
                  <div class="">
                    <div class="flex flex-col gap-2">
                      <div><strong>{cl.coursetitle}</strong></div>
                      <div><strong>{cl.faculity}</strong></div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="collapse-content text-sm overflow-auto">
          <div class=" overflow-x-auto">
            <div
              class="overflow-x-auto rounded-box border border-base-content/5 bg-base-100"
            >
              <table class="table overflow-auto">
                <!-- head -->
                <thead>
                  <tr>
                    <th></th>
                    <th>Title</th>
                    <th>Scored.Mark</th>
                    <th>Max.Mark</th>
                    <th>Weightage.Mark </th>
                    <th>Weightage %</th>
                    <th>Status</th>
                    <th>Remark</th>
                  </tr>
                </thead>
                <tbody>
                  {#each cl.marks as m}
                    <tr>
                      <th>{m.split(":")[0]}</th>
                      <td>{m.split(":")[1]}</td>
                      <td>{m.split(":")[5]}</td>
                      <td>{m.split(":")[2]}</td>
                      <td>{m.split(":")[6]}</td>
                      <td>{m.split(":")[3]}</td>
                      <td>{m.split(":")[4]}</td>
                      <td>{m.split(":")[7]}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
    </div>
  {/each}
  <div class="collapse collapse-plus bg-base-100"></div>
  <div class="divider">
    <p class="text-sm text-info">Data updated on {unixToDate(updatedTime)}</p>
  </div>
  <div class="w-full translate-y-[-3rem]">
    <footer class=" flex items-center justify-evenly gap-4 p-4">
      <small
        ><div class="status animate-bounce status-primary"></div>
        Theory</small
      >
      <small>
        <div class="status animate-bounce status-secondary"></div>
        Lab</small
      >
    </footer>
  </div>
</div>
