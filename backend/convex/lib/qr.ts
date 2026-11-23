// A QR code for the `/authorize` page's pairing code, drawn on the server
// so the page needs no script for it. Only what that needs: version 1
// (21×21), error correction M, alphanumeric mode (up to 20 characters of
// `0-9 A-Z space $%*+-./:`), mask 0. Scanners read any mask, so none is
// chosen by score. Follows ISO/IEC 18004 as Nayuki's QR library does.

const SIZE = 21;
const DATA_CODEWORDS = 16;
const ECC_CODEWORDS = 10;
const ALPHANUMERIC = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/** GF(256) product, modulo x^8 + x^4 + x^3 + x^2 + 1. */
function multiply(x: number, y: number): number {
  let z = 0;
  for (let i = 7; i >= 0; i--) {
    z = (z << 1) ^ ((z >>> 7) * 0x11d);
    z ^= ((y >>> i) & 1) * x;
  }
  return z;
}

function eccFor(data: number[]): number[] {
  const divisor = new Array<number>(ECC_CODEWORDS).fill(0);
  divisor[ECC_CODEWORDS - 1] = 1;
  let root = 1;
  for (let i = 0; i < ECC_CODEWORDS; i++) {
    for (let j = 0; j < ECC_CODEWORDS; j++) {
      divisor[j] = multiply(divisor[j], root);
      if (j + 1 < ECC_CODEWORDS) divisor[j] ^= divisor[j + 1];
    }
    root = multiply(root, 0x02);
  }
  const result = new Array<number>(ECC_CODEWORDS).fill(0);
  for (const byte of data) {
    const factor = byte ^ result.shift()!;
    result.push(0);
    divisor.forEach((coef, i) => (result[i] ^= multiply(coef, factor)));
  }
  return result;
}

/** The data codewords of `text` in alphanumeric mode, padded. */
function codewords(text: string): number[] {
  const bits: number[] = [];
  const put = (value: number, length: number) => {
    for (let i = length - 1; i >= 0; i--) bits.push((value >>> i) & 1);
  };
  put(0b0010, 4);
  put(text.length, 9);
  for (let i = 0; i < text.length; i += 2) {
    const a = ALPHANUMERIC.indexOf(text[i]);
    if (i + 1 < text.length) put(a * 45 + ALPHANUMERIC.indexOf(text[i + 1]), 11);
    else put(a, 6);
  }
  const capacity = DATA_CODEWORDS * 8;
  put(0, Math.min(4, capacity - bits.length));
  put(0, (8 - (bits.length % 8)) % 8);
  for (let pad = 0xec; bits.length < capacity; pad ^= 0xec ^ 0x11) put(pad, 8);
  const out: number[] = [];
  for (let i = 0; i < bits.length; i += 8) out.push(bits.slice(i, i + 8).reduce((acc, bit) => (acc << 1) | bit, 0));
  return out;
}

/** The modules of `text` as rows of booleans (true is dark), without a
 * quiet zone; null when it does not fit or has other characters. */
export function qrModules(text: string): boolean[][] | null {
  if (text.length > 20 || [...text].some((c) => !ALPHANUMERIC.includes(c))) return null;
  const modules = Array.from({ length: SIZE }, () => new Array<boolean>(SIZE).fill(false));
  const fixed = Array.from({ length: SIZE }, () => new Array<boolean>(SIZE).fill(false));
  const set = (x: number, y: number, dark: boolean) => {
    modules[y][x] = dark;
    fixed[y][x] = true;
  };
  for (let i = 0; i < SIZE; i++) {
    set(6, i, i % 2 === 0);
    set(i, 6, i % 2 === 0);
  }
  for (const [cx, cy] of [
    [3, 3],
    [SIZE - 4, 3],
    [3, SIZE - 4],
  ]) {
    for (let dy = -4; dy <= 4; dy++) {
      for (let dx = -4; dx <= 4; dx++) {
        const x = cx + dx;
        const y = cy + dy;
        const ring = Math.max(Math.abs(dx), Math.abs(dy));
        if (x >= 0 && x < SIZE && y >= 0 && y < SIZE) set(x, y, ring !== 2 && ring !== 4);
      }
    }
  }
  // Format information: level M (00), mask 0, then BCH(15, 5).
  const format = 0;
  let rem = format;
  for (let i = 0; i < 10; i++) rem = (rem << 1) ^ ((rem >>> 9) * 0x537);
  const bits = ((format << 10) | rem) ^ 0x5412;
  const bit = (i: number) => ((bits >>> i) & 1) !== 0;
  for (let i = 0; i <= 5; i++) set(8, i, bit(i));
  set(8, 7, bit(6));
  set(8, 8, bit(7));
  set(7, 8, bit(8));
  for (let i = 9; i < 15; i++) set(14 - i, 8, bit(i));
  for (let i = 0; i < 8; i++) set(SIZE - 1 - i, 8, bit(i));
  for (let i = 8; i < 15; i++) set(8, SIZE - 15 + i, bit(i));
  set(8, SIZE - 8, true);

  const data = codewords(text);
  const all = [...data, ...eccFor(data)];
  let i = 0;
  for (let right = SIZE - 1; right >= 1; right -= 2) {
    if (right === 6) right = 5;
    for (let vert = 0; vert < SIZE; vert++) {
      for (let j = 0; j < 2; j++) {
        const x = right - j;
        const upward = ((right + 1) & 2) === 0;
        const y = upward ? SIZE - 1 - vert : vert;
        if (fixed[y][x]) continue;
        const dark = i < all.length * 8 && ((all[i >>> 3] >>> (7 - (i & 7))) & 1) !== 0;
        i++;
        // Mask 0.
        modules[y][x] = dark !== ((x + y) % 2 === 0);
      }
    }
  }
  return modules;
}

/** An SVG of `text`'s QR code, dark on white with a quiet zone of 2. */
export function qrSvg(text: string): string | null {
  const modules = qrModules(text);
  if (modules === null) return null;
  const border = 2;
  const path = modules
    .flatMap((row, y) => row.map((dark, x) => (dark ? `M${x + border} ${y + border}h1v1h-1z` : "")))
    .join("");
  const side = SIZE + border * 2;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${side} ${side}" shape-rendering="crispEdges" role="img" aria-label="Pairing QR code"><rect width="${side}" height="${side}" fill="#fff"/><path d="${path}" fill="#000"/></svg>`;
}
