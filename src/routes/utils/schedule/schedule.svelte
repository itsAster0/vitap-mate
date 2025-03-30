<script lang="ts">
  let { examshedule, updatedtime } = $props();
  let render = $derived(JSON.parse(examshedule));
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

<div class="h-[75vh] w-full overflow-auto">
  {#each render as exam}
    <h1 class=" text-xl text-accent text-center">{exam.exam_type}</h1>
    <div
      class="overflow-x-auto rounded-box border border-base-content/5 bg-base-100"
    >
      <table class="table whitespace-nowrap w-full overflow-auto p-4">
        <thead>
          <tr>
            <th></th>
            <th>Course</th>
            <th>ExamDate</th>
            <th>Venue</th>
            <th>Seat</th>
            <th>SeatNo</th>
            <th>ExamTime</th>
            <th>ReportingTime</th>
            <th>Session</th>
            <th>Slot</th>
            <th>CourseType</th>
            <th>CourseName</th>
            <th>ClassId</th>
          </tr>
        </thead>
        <tbody class="">
          {#each exam.course as c}
            <tr>
              <th>{c.serial}</th>
              <th>{c.course_code}</th>
              <th>{c.exam_date}</th>
              <th>{c.venue}</th>
              <th>{c.seat_location}</th>
              <th>{c.seat_no}</th>
              <th>{c.exam_time}</th>
              <th>{c.reporting_time}</th>
              <th>{c.exam_session}</th>
              <th>{c.slot}</th>
              <th>{c.course_type}</th>
              <th>{c.course_name}</th>
              <th>{c.course_id}</th>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/each}
  <div class=" divider text-sm text-info">
    Data updated on {unixToDate(updatedtime)}
  </div>
</div>
