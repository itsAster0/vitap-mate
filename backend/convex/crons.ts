import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();
crons.interval("sweep expired tickets, codes and requests", { minutes: 1 }, internal.sweeps.minute, {});
crons.interval("sweep mail, sessions and vaults", { minutes: 15 }, internal.sweeps.quarter, {});
crons.interval("sweep refresh tokens", { hours: 24 }, internal.sweeps.day, {});
export default crons;
