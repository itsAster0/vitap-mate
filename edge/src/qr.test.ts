import { describe, expect, it } from "vitest";

import { qrModules, qrSvg } from "./qr";

describe("qr", () => {
  it("draws the same modules as the extension's encoder (uqr, mask 0, level M)", () => {
    const rows = qrModules("ABCDE-12345")!.map((row) => row.map((dark) => (dark ? "#" : ".")).join(""));
    expect(rows).toEqual([
      "#######..#..#.#######",
      "#.....#.##.##.#.....#",
      "#.###.#..##...#.###.#",
      "#.###.#.......#.###.#",
      "#.###.#.##.##.#.###.#",
      "#.....#....#..#.....#",
      "#######.#.#.#.#######",
      "..........#..........",
      "#.#.#.#.....#...#..#.",
      "....#...##.#.........",
      ".#...##.#.##..#.#.##.",
      "##.......#####..#..#.",
      "#.#..##....#..#..###.",
      "........###...##.#...",
      "#######...#.#........",
      "#.....#..#....##.##..",
      "#.###.#.#.#.#.#....##",
      "#.###.#..#.#.#..#..#.",
      "#.###.#.##.#.#####..#",
      "#.....#....###....###",
      "#######.##.#.##.#.#.#",
    ]);
  });

  it("refuses text it cannot draw", () => {
    expect(qrModules("abc")).toBeNull();
    expect(qrModules("A".repeat(21))).toBeNull();
    expect(qrSvg("ABCDE-12345")).toContain('viewBox="0 0 25 25"');
  });
});
