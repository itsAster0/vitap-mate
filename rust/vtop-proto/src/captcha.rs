//! Solves the login CAPTCHA with the embedded model. There is no remote
//! solver: the image never leaves the device (or server) doing the login.
//!
//! The image comes as a `data:image/...;base64,` URL holding a baseline JPEG.
//! Base64 and JPEG decoding are done here, without dependencies. The decoder
//! reproduces zune-jpeg 0.5 (the `image` crate's JPEG backend) pixel for
//! pixel: its integer IDCT, its "fancy" chroma upsampling (vertical, then
//! horizontal, over the padded planes) and its BT.601 colour conversion. The
//! model's thresholded features are sensitive to small pixel differences, so
//! do not swap these for textbook versions.

const LABELS: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const HEIGHT: usize = 40;
const WIDTH: usize = 200;
const INPUTS: usize = 528;

/// Little-endian f32: `INPUTS` rows of `LABELS.len()` weights, then the biases.
/// Made from `assets/captcha_weights.json` (a test checks they agree).
const MODEL: &[u8] = include_bytes!("../assets/captcha_model.bin");
const _: () = assert!(MODEL.len() == (INPUTS + 1) * LABELS.len() * 4);

/// Reads the CAPTCHA in a `data:image/...;base64,` URL.
pub fn solve_data_url(data_url: &str) -> Result<String, String> {
    let (_, image_base64) = data_url
        .split_once(',')
        .filter(|(header, _)| header.starts_with("data:image/") && header.contains(";base64"))
        .ok_or_else(|| "Unsupported CAPTCHA image data URL".to_string())?;
    let raw = base64(image_base64.as_bytes())
        .ok_or_else(|| "CAPTCHA image is not valid base64".to_string())?;
    let image =
        jpeg::decode(&raw).map_err(|error| format!("Unable to decode CAPTCHA image: {error}"))?;
    solve(&image)
}

/// Standard alphabet; the trailing `=` padding is optional.
fn base64(input: &[u8]) -> Option<Vec<u8>> {
    let input = input
        .strip_suffix(b"==")
        .or(input.strip_suffix(b"="))
        .unwrap_or(input);
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let (mut acc, mut bits) = (0_u32, 0);
    for &byte in input {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    // A lone trailing symbol cannot hold a byte; leftover bits must be zero.
    (bits < 6 && acc & ((1 << bits) - 1) == 0).then_some(out)
}

fn solve(image: &jpeg::Image) -> Result<String, String> {
    if image.width != WIDTH || image.height != HEIGHT {
        return Err(format!("CAPTCHA image must be {WIDTH}x{HEIGHT} pixels"));
    }
    let model: Vec<f32> = MODEL
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect();
    let (weights, biases) = model.split_at(INPUTS * LABELS.len());

    let mut output = String::with_capacity(6);
    for block in saturation(&image.rgb) {
        let bits = threshold(&block);
        let scores = dense(&bits, weights, biases);
        let best_index = scores
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(index, _)| index)
            .ok_or_else(|| "Embedded CAPTCHA model produced no result".to_string())?;
        output.push(LABELS[best_index] as char);
    }
    Ok(output)
}

fn saturation(rgb: &[u8]) -> Vec<Vec<f32>> {
    let pixels: Vec<f32> = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&[red, green, blue]| {
            let max = red.max(green).max(blue) as f32;
            let min = red.min(green).min(blue) as f32;
            if max == 0.0 {
                f32::NAN
            } else {
                (((max - min) * 255.0) / max).round()
            }
        })
        .collect();

    (0..6)
        .map(|index| {
            let row_start = 7 + 5 * (index % 2) + 1;
            let row_end = 35 - 5 * ((index + 1) % 2);
            let column_start = (index + 1) * 25 + 2;
            let column_end = (index + 2) * 25 + 1;
            let mut block = Vec::with_capacity(INPUTS);
            for row in row_start..row_end {
                block.extend_from_slice(
                    &pixels[row * WIDTH + column_start..row * WIDTH + column_end],
                );
            }
            block
        })
        .collect()
}

fn threshold(block: &[f32]) -> Vec<f32> {
    let average = block.iter().sum::<f32>() / block.len() as f32;
    block
        .iter()
        .map(|value| if *value > average { 1.0 } else { 0.0 })
        .collect()
}

fn dense(input: &[f32], weights: &[f32], biases: &[f32]) -> Vec<f32> {
    let mut output = biases.to_vec();
    for (input_value, row) in input.iter().zip(weights.as_chunks::<{ LABELS.len() }>().0) {
        if *input_value == 0.0 {
            continue;
        }
        for (score, weight) in output.iter_mut().zip(row) {
            *score += input_value * weight;
        }
    }
    output
}

/// Baseline sequential JPEG, 8-bit, 1 or 3 components in one scan, any
/// sampling factors, restart markers.
mod jpeg {
    const ZIGZAG: [usize; 64] = [
        0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27,
        20, 13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51,
        58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
    ];

    /// Interleaved 8-bit RGB.
    pub(super) struct Image {
        pub width: usize,
        pub height: usize,
        pub rgb: Vec<u8>,
    }

    #[derive(Default, Clone)]
    struct Huffman {
        /// Per code length 1..=16: (first code, last code + 1, index into `values`).
        lengths: [(i32, i32, usize); 17],
        values: Vec<u8>,
    }

    struct Component {
        id: u8,
        h: usize,
        v: usize,
        quant: usize,
        dc: usize,
        ac: usize,
        pred: i32,
        /// Padded plane: `mcu_x * h * 8` wide, `mcu_y * v * 8` tall.
        plane: Vec<i16>,
        stride: usize,
    }

    struct Bits<'a> {
        data: &'a [u8],
        pos: usize,
        acc: u32,
        count: u32,
    }

    impl Bits<'_> {
        fn bit(&mut self) -> i32 {
            if self.count == 0 {
                // A marker ends the entropy data; past it, read zeros.
                let byte = match self.data.get(self.pos..self.pos + 2) {
                    Some([0xFF, 0x00]) => {
                        self.pos += 2;
                        0xFF
                    }
                    Some([0xFF, _]) | None => 0,
                    Some(_) => {
                        self.pos += 1;
                        self.data[self.pos - 1]
                    }
                };
                self.acc = u32::from(byte);
                self.count = 8;
            }
            self.count -= 1;
            ((self.acc >> self.count) & 1) as i32
        }

        fn receive_extend(&mut self, size: u8) -> i32 {
            let mut value = 0;
            for _ in 0..size {
                value = (value << 1) | self.bit();
            }
            if size > 0 && value < 1 << (size - 1) {
                value - (1 << size) + 1
            } else {
                value
            }
        }

        fn decode(&mut self, table: &Huffman) -> Result<u8, &'static str> {
            let mut code = 0;
            for &(first, end, index) in &table.lengths[1..] {
                code = (code << 1) | self.bit();
                if code < end {
                    return Ok(table.values[index + (code - first) as usize]);
                }
            }
            Err("bad Huffman code")
        }
    }

    fn u16_at(data: &[u8], pos: usize) -> Result<usize, &'static str> {
        data.get(pos..pos + 2)
            .map(|b| usize::from(b[0]) << 8 | usize::from(b[1]))
            .ok_or("truncated")
    }

    pub(super) fn decode(data: &[u8]) -> Result<Image, &'static str> {
        if !data.starts_with(&[0xFF, 0xD8]) {
            return Err("not a JPEG");
        }
        let mut quant = [[0_i32; 64]; 4];
        let mut dc_tables = vec![Huffman::default(); 4];
        let mut ac_tables = vec![Huffman::default(); 4];
        let mut components: Vec<Component> = Vec::new();
        let (mut width, mut height, mut restart) = (0, 0, 0);
        let mut pos = 2;
        loop {
            while data.get(pos) == Some(&0xFF) && data.get(pos + 1) == Some(&0xFF) {
                pos += 1;
            }
            let marker = match data.get(pos..pos + 2) {
                Some([0xFF, marker]) => *marker,
                _ => return Err("expected a marker"),
            };
            let length = u16_at(data, pos + 2)?;
            let segment = data.get(pos + 4..pos + 2 + length).ok_or("truncated")?;
            match marker {
                0xC0 | 0xC1 => {
                    if segment.first() != Some(&8) {
                        return Err("only 8-bit JPEG is supported");
                    }
                    height = u16_at(segment, 1)?;
                    width = u16_at(segment, 3)?;
                    let count = usize::from(*segment.get(5).ok_or("truncated")?);
                    if !(count == 1 || count == 3) || width == 0 || height == 0 {
                        return Err("unsupported frame");
                    }
                    for c in segment.get(6..6 + 3 * count).ok_or("truncated")?.chunks(3) {
                        let (h, v) = (usize::from(c[1] >> 4), usize::from(c[1] & 15));
                        if !(1..=4).contains(&h) || !(1..=4).contains(&v) || c[2] > 3 {
                            return Err("bad component");
                        }
                        // One component: blocks in raster order, factors unused.
                        let (h, v) = if count == 1 { (1, 1) } else { (h, v) };
                        components.push(Component {
                            id: c[0],
                            h,
                            v,
                            quant: usize::from(c[2]),
                            dc: 0,
                            ac: 0,
                            pred: 0,
                            plane: Vec::new(),
                            stride: 0,
                        });
                    }
                }
                0xC2 | 0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                    return Err("only baseline Huffman JPEG is supported");
                }
                0xC4 => {
                    let mut rest = segment;
                    while let [class_id, counts @ ..] = rest {
                        let counts = counts.get(..16).ok_or("truncated")?;
                        let total: usize = counts.iter().map(|&n| usize::from(n)).sum();
                        let values = rest.get(17..17 + total).ok_or("truncated")?;
                        let mut table = Huffman {
                            values: values.to_vec(),
                            ..Huffman::default()
                        };
                        let (mut code, mut index) = (0, 0);
                        for (bits, &n) in counts.iter().enumerate() {
                            table.lengths[bits + 1] = (code, code + i32::from(n), index);
                            code = (code + i32::from(n)) << 1;
                            index += usize::from(n);
                        }
                        let id = usize::from(class_id & 15);
                        match (class_id >> 4, id) {
                            (0, 0..=3) => dc_tables[id] = table,
                            (1, 0..=3) => ac_tables[id] = table,
                            _ => return Err("bad Huffman table"),
                        }
                        rest = &rest[17 + total..];
                    }
                }
                0xDB => {
                    let mut rest = segment;
                    while let [precision_id, values @ ..] = rest {
                        let wide = precision_id >> 4 == 1;
                        let size = if wide { 128 } else { 64 };
                        let values = values.get(..size).ok_or("truncated")?;
                        let table = quant
                            .get_mut(usize::from(precision_id & 15))
                            .ok_or("bad quantisation table")?;
                        for (k, &natural) in ZIGZAG.iter().enumerate() {
                            table[natural] = if wide {
                                i32::from(values[2 * k]) << 8 | i32::from(values[2 * k + 1])
                            } else {
                                i32::from(values[k])
                            };
                        }
                        rest = &rest[1 + size..];
                    }
                }
                0xDD => restart = u16_at(segment, 0)?,
                0xDA => {
                    let count = usize::from(*segment.first().ok_or("truncated")?);
                    if components.is_empty() || count != components.len() {
                        return Err("only single-scan JPEG is supported");
                    }
                    for s in segment.get(1..1 + 2 * count).ok_or("truncated")?.chunks(2) {
                        let component = components
                            .iter_mut()
                            .find(|c| c.id == s[0])
                            .ok_or("scan names an unknown component")?;
                        component.dc = usize::from(s[1] >> 4).min(3);
                        component.ac = usize::from(s[1] & 15).min(3);
                    }
                    let mut bits = Bits {
                        data,
                        pos: pos + 2 + length,
                        acc: 0,
                        count: 0,
                    };
                    scan(
                        &mut components,
                        &quant,
                        &dc_tables,
                        &ac_tables,
                        width,
                        height,
                        restart,
                        &mut bits,
                    )?;
                    return Ok(convert(&mut components, width, height));
                }
                0xD9 => return Err("no image data"),
                _ => {} // APPn, COM, ...
            }
            pos += 2 + length;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn scan(
        components: &mut [Component],
        quant: &[[i32; 64]; 4],
        dc_tables: &[Huffman],
        ac_tables: &[Huffman],
        width: usize,
        height: usize,
        restart: usize,
        bits: &mut Bits,
    ) -> Result<(), &'static str> {
        let h_max = components.iter().map(|c| c.h).max().unwrap_or(1);
        let v_max = components.iter().map(|c| c.v).max().unwrap_or(1);
        if components
            .iter()
            .any(|c| h_max % c.h != 0 || v_max % c.v != 0)
        {
            return Err("unsupported sampling factors");
        }
        let mcu_x = width.div_ceil(8 * h_max);
        let mcu_y = height.div_ceil(8 * v_max);
        for c in components.iter_mut() {
            c.stride = mcu_x * c.h * 8;
            c.plane = vec![0; c.stride * mcu_y * c.v * 8];
        }
        let mut block = [0_i32; 64];
        for mcu in 0..mcu_x * mcu_y {
            if restart > 0 && mcu > 0 && mcu % restart == 0 {
                // Byte-align, then skip the RSTn marker.
                bits.count = 0;
                if matches!(
                    bits.data.get(bits.pos..bits.pos + 2),
                    Some([0xFF, 0xD0..=0xD7])
                ) {
                    bits.pos += 2;
                }
                components.iter_mut().for_each(|c| c.pred = 0);
            }
            let (mx, my) = (mcu % mcu_x, mcu / mcu_x);
            for c in components.iter_mut() {
                let q = &quant[c.quant];
                for by in 0..c.v {
                    for bx in 0..c.h {
                        block.fill(0);
                        let size = bits.decode(&dc_tables[c.dc])?;
                        if size > 11 {
                            return Err("bad DC size");
                        }
                        c.pred = c.pred.wrapping_add(bits.receive_extend(size));
                        block[0] = c.pred.wrapping_mul(q[0]);
                        let mut k = 1;
                        while k < 64 {
                            let rs = bits.decode(&ac_tables[c.ac])?;
                            let (run, size) = (usize::from(rs >> 4), rs & 15);
                            if size == 0 {
                                if run != 15 {
                                    break;
                                }
                                k += 16;
                                continue;
                            }
                            k += run;
                            let natural = *ZIGZAG.get(k).ok_or("bad AC run")?;
                            block[natural] = bits.receive_extend(size).wrapping_mul(q[natural]);
                            k += 1;
                        }
                        let x = (mx * c.h + bx) * 8;
                        let y = (my * c.v + by) * 8;
                        idct(&mut block, &mut c.plane[y * c.stride + x..], c.stride);
                    }
                }
            }
        }
        Ok(())
    }

    /// zune-jpeg's scalar integer IDCT (stb_image's), output clamped to 0..=255.
    fn idct(v: &mut [i32; 64], out: &mut [i16], stride: usize) {
        if v[1..].iter().all(|&x| x == 0) {
            let dc = (v[0].wrapping_add(4 + 1024) >> 3).clamp(0, 255) as i16;
            for row in 0..8 {
                out[row * stride..row * stride + 8].fill(dc);
            }
            return;
        }
        // Columns (step 8, bias 512, shift 10), then rows (step 1, shift 17).
        for pass in 0..2 {
            for line in 0..8 {
                let (base, step, bias) = if pass == 0 {
                    (line, 8, 512)
                } else {
                    (line * 8, 1, 512 + 65536 + (128 << 17))
                };
                let at = |i: usize| v[base + i * step];
                let (wa, ws, wm) = (i32::wrapping_add, i32::wrapping_sub, i32::wrapping_mul);

                let p1 = wm(wa(at(2), at(6)), 2217);
                let t2 = wa(p1, wm(at(6), -7567));
                let t3 = wa(p1, wm(at(2), 3135));
                let t0 = wa(at(0), at(4)) << 12;
                let t1 = ws(at(0), at(4)) << 12;
                let x0 = wa(wa(t0, t3), bias);
                let x3 = wa(ws(t0, t3), bias);
                let x1 = wa(wa(t1, t2), bias);
                let x2 = wa(ws(t1, t2), bias);

                let (o0, o1, o2, o3) = (at(7), at(5), at(3), at(1));
                let p3 = wa(o0, o2);
                let p4 = wa(o1, o3);
                let p5 = wm(wa(p3, p4), 4816);
                let p1 = wa(p5, wm(wa(o0, o3), -3685));
                let p2 = wa(p5, wm(wa(o1, o2), -10497));
                let p3 = wm(p3, -8034);
                let p4 = wm(p4, -1597);
                let t3 = wa(wm(o3, 6149), wa(p1, p4));
                let t2 = wa(wm(o2, 12586), wa(p2, p3));
                let t1 = wa(wm(o1, 8410), wa(p2, p4));
                let t0 = wa(wm(o0, 1223), wa(p1, p3));

                let results = [
                    wa(x0, t3),
                    wa(x1, t2),
                    wa(x2, t1),
                    wa(x3, t0),
                    ws(x3, t0),
                    ws(x2, t1),
                    ws(x1, t2),
                    ws(x0, t3),
                ];
                for (i, value) in results.into_iter().enumerate() {
                    if pass == 0 {
                        v[base + i * 8] = value >> 10;
                    } else {
                        out[line * stride + i] = (value >> 17).clamp(0, 255) as i16;
                    }
                }
            }
        }
    }

    /// Brings a padded plane up to full size by `h` x `v`. A factor of 2 uses
    /// zune-jpeg's triangle filter, `(3 * near + far + 2) >> 2` with repeated
    /// edges, vertical first; any other factor repeats samples.
    fn upsample(plane: Vec<i16>, width: usize, h: usize, v: usize) -> Vec<i16> {
        let tri = |near: i16, far: i16| (3 * near + far + 2) >> 2;
        match (h, v) {
            (1, 1) => plane,
            (1, 2) | (2, 1) | (2, 2) => {
                let mut out = plane;
                if v == 2 {
                    let rows: Vec<&[i16]> = out.chunks_exact(width).collect();
                    let last = rows.len() - 1;
                    let mut next = Vec::with_capacity(out.len() * 2);
                    for (r, row) in rows.iter().enumerate() {
                        let (up, down) = (rows[r.saturating_sub(1)], rows[(r + 1).min(last)]);
                        next.extend(row.iter().zip(up).map(|(&a, &b)| tri(a, b)));
                        next.extend(row.iter().zip(down).map(|(&a, &b)| tri(a, b)));
                    }
                    out = next;
                }
                if h == 2 {
                    let mut next = Vec::with_capacity(out.len() * 2);
                    for row in out.chunks_exact(width) {
                        for x in 0..width {
                            next.push(tri(row[x], row[x.saturating_sub(1)]));
                            next.push(tri(row[x], row[(x + 1).min(width - 1)]));
                        }
                    }
                    out = next;
                }
                out
            }
            _ => {
                let mut out = Vec::with_capacity(plane.len() * h * v);
                for row in plane.chunks_exact(width) {
                    let wide: Vec<i16> = row
                        .iter()
                        .flat_map(|&s| std::iter::repeat_n(s, h))
                        .collect();
                    for _ in 0..v {
                        out.extend_from_slice(&wide);
                    }
                }
                out
            }
        }
    }

    fn convert(components: &mut [Component], width: usize, height: usize) -> Image {
        let h_max = components.iter().map(|c| c.h).max().unwrap_or(1);
        let v_max = components.iter().map(|c| c.v).max().unwrap_or(1);
        let mut planes = Vec::with_capacity(components.len());
        let mut stride = 0;
        for c in components.iter_mut() {
            let plane = std::mem::take(&mut c.plane);
            planes.push(upsample(plane, c.stride, h_max / c.h, v_max / c.v));
            stride = stride.max(c.stride * (h_max / c.h));
        }
        let mut rgb = Vec::with_capacity(width * height * 3);
        for y in 0..height {
            for x in 0..width {
                let at = y * stride + x;
                if let [luma] = planes.as_slice() {
                    rgb.extend_from_slice(&[luma[at] as u8; 3]);
                    continue;
                }
                // BT.601 full range, 14-bit fixed point.
                let y0 = i32::from(planes[0][at]) * 16384 + (1 << 13) - 1;
                let cb = i32::from(planes[1][at]) - 128;
                let cr = i32::from(planes[2][at]) - 128;
                let r = (y0 + cr * 22970) >> 14;
                let g = (y0 + cr * -11700 + cb * -5638) >> 14;
                let b = (y0 + cb * 29032) >> 14;
                rgb.extend([r, g, b].map(|c| c.clamp(0, 255) as u8));
            }
        }
        Image { width, height, rgb }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn samples() -> Vec<(String, Vec<u8>)> {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/captchas");
        (1..=50)
            .map(|n| {
                let name = format!("{n}.jpg");
                let bytes = std::fs::read(dir.join(&name)).expect("sample");
                (name, bytes)
            })
            .collect()
    }

    fn data_url(bytes: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::from("data:image/jpeg;base64,");
        for chunk in bytes.chunks(3) {
            let n = chunk.iter().fold(0_u32, |acc, &b| acc << 8 | u32::from(b))
                << (8 * (3 - chunk.len()));
            for i in 0..4 {
                out.push(if i <= chunk.len() {
                    ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char
                } else {
                    '='
                });
            }
        }
        out
    }

    #[test]
    fn model_matches_json() {
        #[derive(serde::Deserialize)]
        struct Model {
            weights: Vec<Vec<f32>>,
            biases: Vec<f32>,
        }
        let json = include_str!("../assets/captcha_weights.json");
        let model: Model = serde_json::from_str(json).expect("model json");
        assert_eq!(model.weights.len(), INPUTS);
        assert!(model.weights.iter().all(|row| row.len() == LABELS.len()));
        let expected: Vec<u8> = model
            .weights
            .iter()
            .flatten()
            .chain(&model.biases)
            .flat_map(|x| x.to_le_bytes())
            .collect();
        assert!(
            expected == MODEL,
            "captcha_model.bin differs from captcha_weights.json"
        );
    }

    #[test]
    fn answers_match_saved_answers() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/captchas");
        let answers: BTreeMap<String, String> =
            serde_json::from_str(&std::fs::read_to_string(dir.join("answers.json")).unwrap())
                .unwrap();
        let wrong: Vec<_> = samples()
            .into_iter()
            .filter_map(|(name, bytes)| {
                let got = solve_data_url(&data_url(&bytes)).unwrap();
                (got != answers[&name]).then(|| format!("{name}: {got} != {}", answers[&name]))
            })
            .collect();
        assert!(wrong.is_empty(), "{wrong:?}");
    }

    #[test]
    fn pixels_match_image_crate() {
        let mut max_diff = 0;
        for (name, bytes) in samples() {
            let ours = jpeg::decode(&bytes).unwrap();
            let theirs = image::load_from_memory(&bytes).unwrap().to_rgb8();
            assert_eq!(
                (ours.width, ours.height),
                (theirs.width() as usize, theirs.height() as usize)
            );
            let diff = ours
                .rgb
                .iter()
                .zip(theirs.as_raw())
                .map(|(&a, &b)| a.abs_diff(b))
                .max()
                .unwrap_or(0);
            if diff > 0 {
                eprintln!("{name}: max pixel diff {diff}");
            }
            max_diff = max_diff.max(diff);
        }
        assert_eq!(max_diff, 0);
    }

    #[test]
    fn base64_round_trips() {
        for len in 0..10 {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            let url = data_url(&bytes);
            let encoded = url.split_once(',').unwrap().1;
            assert_eq!(base64(encoded.as_bytes()).unwrap(), bytes);
            assert_eq!(
                base64(encoded.trim_end_matches('=').as_bytes()).unwrap(),
                bytes
            );
        }
        assert!(base64(b"ab$d").is_none());
        assert!(base64(b"A").is_none());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(solve_data_url("not-an-image").is_err());
        assert!(solve_data_url("data:image/jpeg;base64,AAAA").is_err());
        let (_, mut bytes) = samples().swap_remove(0);
        let sof = bytes.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
        bytes[sof + 1] = 0xC2;
        let error = solve_data_url(&data_url(&bytes)).unwrap_err();
        assert!(error.contains("baseline"), "{error}");
    }

    /// Run with `--nocapture` (and `--release`) to see the time.
    #[test]
    fn solve_time() {
        let url = data_url(&samples()[0].1);
        let start = std::time::Instant::now();
        let runs = 20;
        for _ in 0..runs {
            solve_data_url(&url).unwrap();
        }
        eprintln!("solve_data_url: {:?} per CAPTCHA", start.elapsed() / runs);
    }
}
