import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { type EventSocket, eventStream } from "./event-stream";

class FakeSocket implements EventSocket {
  accepted = false;
  sent: string[] = [];
  closed = false;
  private listeners = new Map<string, ((event: { data: unknown }) => void)[]>();

  accept() {
    this.accepted = true;
  }
  send(message: string) {
    if (this.closed) throw new Error("closed");
    this.sent.push(message);
  }
  close() {
    this.closed = true;
  }
  addEventListener(type: string, listener: (event: { data: unknown }) => void) {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener]);
  }
  emit(type: string, data?: unknown) {
    for (const listener of this.listeners.get(type) ?? []) listener({ data });
  }
}

/** Everything the stream has queued, as text, and whether it ended. */
async function drain(reader: ReadableStreamDefaultReader<Uint8Array>): Promise<{ text: string; done: boolean }> {
  const decoder = new TextDecoder();
  let text = "";
  for (;;) {
    const next = await Promise.race([reader.read(), new Promise<null>((r) => setTimeout(() => r(null), 0))]);
    if (next === null) return { text, done: false };
    if (next.done) return { text, done: true };
    text += decoder.decode(next.value);
  }
}

describe("eventStream", () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "Date"] });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("passes frames on as data lines and drops pongs", async () => {
    const socket = new FakeSocket();
    const reader = eventStream(socket, 1000).getReader();
    expect(socket.accepted).toBe(true);
    socket.emit("message", '{"type":"hello","latest":3}');
    socket.emit("message", "pong");
    expect((await drain(reader)).text).toBe('data: {"type":"hello","latest":3}\n\n');
  });

  it("pings the object and beats every interval", async () => {
    const socket = new FakeSocket();
    const reader = eventStream(socket, 1000).getReader();
    vi.advanceTimersByTime(1000);
    expect(socket.sent).toEqual(["ping"]);
    expect((await drain(reader)).text).toBe(": ping\n\n");
  });

  it("ends when the object stops answering", async () => {
    const socket = new FakeSocket();
    const reader = eventStream(socket, 1000).getReader();
    vi.advanceTimersByTime(3000);
    expect(socket.closed).toBe(true);
    expect((await drain(reader)).done).toBe(true);
  });

  it("ends when the object's socket closes", async () => {
    const socket = new FakeSocket();
    const reader = eventStream(socket, 1000).getReader();
    socket.emit("close");
    expect((await drain(reader)).done).toBe(true);
  });

  it("closes the socket when the app goes away", async () => {
    const socket = new FakeSocket();
    const reader = eventStream(socket, 1000).getReader();
    await reader.cancel();
    expect(socket.closed).toBe(true);
    vi.advanceTimersByTime(5000);
    expect(socket.sent).toEqual([]);
  });
});
