// The help content the app fetches (edge/guides/*.json), bundled at deploy:
// edit the JSON there and deploy, as before.

import { query } from "./_generated/server";
import demoJson from "../../edge/guides/demo.json";
import forwardingJson from "../../edge/guides/forwarding.json";
import updatesJson from "../../edge/guides/updates.json";

export const updates = query({ args: {}, handler: () => updatesJson });
export const demo = query({ args: {}, handler: () => demoJson });
export const forwarding = query({ args: {}, handler: () => forwardingJson });
