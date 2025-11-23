import type { PageServerLoad } from "./$types";
import { env } from "$env/dynamic/private";
export const load: PageServerLoad = async ({ params }) => {
  let fetchTt = async () => {
    let url = env.DB_URL;

    try {
      const res = await fetch(url + "/timetable/fetch", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          timetable_fetch_id: params.slug,
        }),
      });

      if (!res.ok) {
        return { error: "Fetch failed: " + res.status };
      } else {
        return { error: null, result: await res.json() };
      }
    } catch (e) {
      return { error: "Network error" };
    }
  };
  return await fetchTt();
};
