// Vite (vitest) loads `…?raw` imports as the file's text.
declare module "*?raw" {
  const text: string;
  export default text;
}
