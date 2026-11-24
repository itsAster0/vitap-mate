// The help content the app fetches (backend/guides/*.json), bundled at deploy:
// edit the JSON there and deploy, as before.

import { query } from "./_generated/server";
import demoJson from "../guides/demo.json";
import forwardingJson from "../guides/forwarding.json";
import updatesJson from "../guides/updates.json";

export const updates = query({ args: {}, handler: () => updatesJson });
export const demo = query({ args: {}, handler: () => demoJson });
export const forwarding = query({ args: {}, handler: () => forwardingJson });
