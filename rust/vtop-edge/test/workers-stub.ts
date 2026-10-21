// `cloudflare:workers` outside the Workers runtime: enough for the unit
// tests to import modules that define Durable Objects.

export class DurableObject<Env = unknown> {
  constructor(
    readonly ctx: unknown,
    readonly env: Env,
  ) {}
}
