// `bun run docs`: rewrites API.md's route table (between
// `<!-- routes:start -->` and `<!-- routes:end -->`) from ROUTES.

import { withRouteTable } from "../src/routes";

const file = new URL("../API.md", import.meta.url);
const before = await Bun.file(file).text();
const after = withRouteTable(before);
if (after === before) {
  console.log("API.md: route table up to date");
} else {
  await Bun.write(file, after);
  console.log("API.md: route table rewritten");
}
