use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::ImageReader;
use serde::Deserialize;
use std::sync::OnceLock;

const LABELS: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const HEIGHT: usize = 40;
const WIDTH: usize = 200;

// include_bytes! embeds the model in the native Rust library as a byte array.
const MODEL_BYTES: &[u8] = include_bytes!("../../../assets/captcha_weights.json");

#[derive(Debug, Deserialize)]
struct Model {
    weights: Vec<Vec<f32>>,
    biases: Vec<f32>,
}

static MODEL: OnceLock<Result<Model, String>> = OnceLock::new();

pub(super) fn solve_data_url(data_url: &str) -> Result<String, String> {
    let (_, image_base64) = data_url
        .split_once(",")
        .filter(|(header, _)| header.starts_with("data:image/") && header.contains(";base64"))
        .ok_or_else(|| "Unsupported CAPTCHA image data URL".to_string())?;

    let raw = STANDARD
        .decode(image_base64.as_bytes())
        .map_err(|_| "CAPTCHA image is not valid base64".to_string())?;
    let image = ImageReader::new(std::io::Cursor::new(raw))
        .with_guessed_format()
        .map_err(|_| "Unable to detect CAPTCHA image format".to_string())?
        .decode()
        .map_err(|_| "Unable to decode CAPTCHA image".to_string())?
        .to_rgb8();

    solve(&image, model()?)
}

fn model() -> Result<&'static Model, String> {
    match MODEL.get_or_init(|| {
        let model: Model = serde_json::from_slice(MODEL_BYTES)
            .map_err(|error| format!("Unable to load embedded CAPTCHA model: {error}"))?;
        validate_model(&model)?;
        Ok(model)
    }) {
        Ok(model) => Ok(model),
        Err(error) => Err(error.clone()),
    }
}

fn validate_model(model: &Model) -> Result<(), String> {
    if model.weights.len() != 528
        || model.weights.iter().any(|row| row.len() != LABELS.len())
        || model.biases.len() != LABELS.len()
    {
        return Err("Embedded CAPTCHA model has invalid dimensions".to_string());
    }
    Ok(())
}

fn solve(image: &image::RgbImage, model: &Model) -> Result<String, String> {
    if image.width() as usize != WIDTH || image.height() as usize != HEIGHT {
        return Err(format!("CAPTCHA image must be {WIDTH}x{HEIGHT} pixels"));
    }

    let saturated = saturation(image);
    let mut output = String::with_capacity(6);

    for block in saturated {
        let bits = threshold(&block);
        let scores = dense(&bits, model);
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

fn saturation(image: &image::RgbImage) -> Vec<Vec<f32>> {
    let mut pixels = vec![0.0_f32; HEIGHT * WIDTH];

    for (index, pixel) in image.pixels().enumerate() {
        let max = *pixel.0.iter().max().expect("RGB pixel has channels") as f32;
        let min = *pixel.0.iter().min().expect("RGB pixel has channels") as f32;
        pixels[index] = if max == 0.0 {
            f32::NAN
        } else {
            (((max - min) * 255.0) / max).round()
        };
    }

    let mut blocks = Vec::with_capacity(6);
    for index in 0..6 {
        let row_start = 7 + 5 * (index % 2) + 1;
        let row_end = 35 - 5 * ((index + 1) % 2);
        let column_start = (index + 1) * 25 + 2;
        let column_end = (index + 2) * 25 + 1;

        let mut block = Vec::with_capacity(528);
        for row in row_start..row_end {
            for column in column_start..column_end {
                block.push(pixels[row * WIDTH + column]);
            }
        }
        blocks.push(block);
    }

    blocks
}

fn threshold(block: &[f32]) -> Vec<f32> {
    let average = block.iter().sum::<f32>() / block.len() as f32;
    block
        .iter()
        .map(|value| if *value > average { 1.0 } else { 0.0 })
        .collect()
}

fn dense(input: &[f32], model: &Model) -> Vec<f32> {
    let mut output = model.biases.clone();

    for (input_value, weights) in input.iter().zip(&model.weights) {
        if *input_value == 0.0 {
            continue;
        }
        for (score, weight) in output.iter_mut().zip(weights) {
            *score += input_value * weight;
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_model_has_expected_dimensions() {
        let model = model().expect("embedded model should be valid");
        assert_eq!(model.weights.len(), 528);
        assert_eq!(model.biases.len(), LABELS.len());
    }

    #[test]
    fn rejects_non_image_data_urls() {
        assert!(solve_data_url("not-an-image").is_err());
    }
}
