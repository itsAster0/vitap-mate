// The Expressive shapes the popup draws: the cookie (a circle whose radius waves, like the app's
// MaterialShapes.Cookie) and the wavy progress line.

/** A cookie with `n` scallops on a 100×100 grid. */
export function cookiePath(n: number, depth = n >= 9 ? 0.05 : 0.09): string {
  const points: string[] = [];
  for (let i = 0; i <= 180; i++) {
    const t = (i / 180) * Math.PI * 2;
    const r = 46 * (1 - depth + depth * Math.cos(n * t));
    points.push(`${(50 + r * Math.cos(t)).toFixed(2)} ${(50 + r * Math.sin(t)).toFixed(2)}`);
  }
  return `M${points.join(" L")}Z`;
}

/** A sine line `width` long on a 14-high strip, one wave every 16, with a wave to spare for its drift. */
export function wavePath(width: number): string {
  let d = "M0 7";
  for (let x = 0; x <= width + 16; x += 2) d += ` L${x} ${(7 + 3.2 * Math.sin((x / 16) * Math.PI * 2)).toFixed(2)}`;
  return d;
}
