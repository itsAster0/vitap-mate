import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();
crons.interval("sweep expired tickets, codes and requests", { minutes: 5 }, internal.sweeps.fiveMinutes, {});
crons.interval("sweep mail, sessions and vaults", { minutes: 15 }, internal.sweeps.quarter, {});
export default crons;
