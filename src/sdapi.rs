use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine};
use image::{imageops, DynamicImage, ImageBuffer, ImageOutputFormat};
use reqwest::Client;
use serde_json::{json, Value};
use std::io::Cursor;
use tracing::error;

#[derive(Clone)]
pub struct SDApi {
    pub reqwest: Client,
}

impl SDApi {
    pub fn init() -> Self {
        SDApi {
            reqwest: reqwest::Client::new(),
        }
    }
    pub async fn txt2img(self, prompt: &str) -> Result<Vec<u8>> {
        let body = get_json_body(prompt, 4);

        let mut rsp = self
            .reqwest
            .post("http://127.0.0.1:7860/sdapi/v1/txt2img")
            .header("Content-Type", "application/json")
            .header("accept", "application/json")
            .body(body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        let images = rsp["images"].take();

        let img: Vec<String> = match serde_json::from_value(images) {
            Ok(img) => img,
            Err(_) => {
                error!("图片没有生成,body:{}", rsp.to_string());
                return Err(anyhow!("调用sdapi失败"));
            }
        };

        let imgs_u8: Vec<Vec<u8>> = img
            .into_iter()
            .map(|f| general_purpose::STANDARD.decode(f).unwrap())
            .collect();

        img_build(&imgs_u8)
    }
}

fn get_json_body(prompt: &str, count: u16) -> String {
    let prompts = format!("ultra detailed 8k cg,{}", prompt);

    let body_json = json!(
            {
      "enable_hr": false,
      "denoising_strength": 0,
      "firstphase_width": 0,
      "firstphase_height": 0,
      "hr_scale": 2,
      "hr_upscaler": "",
      "hr_second_pass_steps": 0,
      "hr_resize_x": 0,
      "hr_resize_y": 0,
      "prompt": prompts,
      "styles": [
      ],
      "seed": -1,
      "subseed": -1,
      "subseed_strength": 0,
      "seed_resize_from_h": -1,
      "seed_resize_from_w": -1,
      "sampler_name": "DPM++ 2M Karras",
      "batch_size": count,
      "n_iter": 1,
      "steps": 30,
      "cfg_scale": 10,
      "width": 768,
      "height": 768,
      "restore_faces": false,
      "tiling": false,
      "do_not_save_samples": false,
      "do_not_save_grid": false,
      "negative_prompt": " (worst quality, low quality:1.3), logo, watermark, signature, text,easynegative",
      "eta": 0.68,
      "s_churn": 0,
      "s_tmax": 0,
      "ENSD":31337,
      "s_tmin": 0,
      "s_noise": 1,
      "override_settings": {},
      "override_settings_restore_afterwards": true,
      "script_args": [],
      "sampler_index": "DDIM",
      "script_name": "",
      "send_images": true,
      "save_images": false,
      "alwayson_scripts": {}
    }
        );
    body_json.to_string()
}

fn img_build(imgs: &[Vec<u8>]) -> Result<Vec<u8>> {
    let mut images = Vec::new();

    for i in imgs {
        let img = image::load_from_memory(i)?;
        images.push(img);
    }

    let mut output = ImageBuffer::new(2 * images[0].width(), 2 * images[0].height());

    for (i, img) in images.iter().enumerate() {
        let x = (i % 2) as u32 * img.width();
        let y = (i / 2) as u32 * img.height();
        imageops::overlay(&mut output, img, x as i64, y as i64);
    }

    let result = DynamicImage::ImageRgba8(output);

    //转换为jpeg
    let mut jpeg_buffer: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut jpeg_buffer);
    result
        .write_to(&mut cursor, ImageOutputFormat::Jpeg(100))
        .expect("Failed to convert the image");

    Ok(jpeg_buffer)
}
