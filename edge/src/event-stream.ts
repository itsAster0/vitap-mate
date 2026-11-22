// The event channel as server-sent events, for apps that read it on their
// own HTTP stack (HTTP/3 where it can) instead of opening a WebSocket: the
// stream shares the connection the app's other calls use and keeps it
// warm. The Worker holds the stream and a WebSocket to the install's
// Device object, so the object still hibernates between events.

/** How often the stream says it is alive and the Device socket is pinged.
 * The app gives up after two missed beats. */
export const HEARTBEAT_MS = 15_000;

/** The part of a WebSocket the stream uses. */
export interface EventSocket {
  accept(): void;
  send(message: string): void;
  close(code?: number, reason?: string): void;
  addEventListener(type: "message", listener: (event: { data: unknown }) => void): void;
  addEventListener(type: "close" | "error", listener: () => void): void;
}

/** The Device socket's frames as `data:` lines, with a comment line every
 * beat. Ends when either side goes away or the object stops answering
 * pings; the app then reconnects and catches up from its cursor. */
export function eventStream(socket: EventSocket, heartbeatMs = HEARTBEAT_MS): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  let timer: ReturnType<typeof setInterval> | null = null;
  let lastPong = Date.now();
  let done = false;
  let controller: ReadableStreamDefaultController<Uint8Array> | undefined;

  const finish = () => {
    if (done) return;
    done = true;
    clearInterval(timer);
    try {
      socket.close(1000, "bye");
    } catch {}
    try {
      controller?.close();
    } catch {}
  };
  const write = (text: string) => {
    if (done) return;
    try {
      controller?.enqueue(encoder.encode(text));
    } catch {
      finish();
    }
  };

  return new ReadableStream<Uint8Array>({
    start(c) {
      controller = c;
      socket.accept();
      socket.addEventListener("message", ({ data }) => {
        lastPong = Date.now();
        // Frames are single-line JSON, so one `data:` line each.
        if (typeof data === "string" && data !== "pong") write(`data: ${data}\n\n`);
      });
      socket.addEventListener("close", finish);
      socket.addEventListener("error", finish);
      timer = setInterval(() => {
        if (Date.now() - lastPong > heartbeatMs * 2) return finish();
        try {
          // Answered by the object's auto-response, without waking it.
          socket.send("ping");
        } catch {
          return finish();
        }
        write(": ping\n\n");
      }, heartbeatMs);
    },
    cancel: finish,
  });
}
