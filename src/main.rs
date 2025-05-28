use clap::{Parser, ValueEnum};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use std::{path::PathBuf, process::Command};
use tempfile::tempdir;

#[derive(Debug, Clone, ValueEnum)]
enum FadeStyle {
    ToDark,
    FromDark,
    ToDarkAndBack,
    FromDarkAndBack,
}

#[derive(Parser, Debug)]
#[command(name = "ImageFader")]
struct Args {
    /// Input image path
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output video path. Defaults to <input>.mp4
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Frame rate of the output video
    #[arg(short, long, default_value = "10")]
    framerate: u32,

    /// Duration of the fade effect in seconds
    #[arg(short, long, default_value = "2")]
    duration: f32,

    /// Style of the fade effect
    #[arg(short, long, value_enum, default_value = "to-dark")]
    style: FadeStyle,
}

fn main() {
    let args = Args::parse();

    let output_path = args.output.unwrap_or_else(|| {
        let stem = args.input.file_stem().unwrap_or_default();
        let mut output = PathBuf::from(stem);
        output.set_extension("mp4");
        output
    });

    let img = image::open(&args.input).expect("Failed to open input image");

    let frame_count = (args.duration * args.framerate as f32).ceil() as usize;
    let fade_factors = match args.style {
        FadeStyle::ToDark => (0..frame_count)
            .map(|i| 1.0 - i as f32 / (frame_count - 1) as f32)
            .collect(),
        FadeStyle::FromDark => (0..frame_count)
            .map(|i| i as f32 / (frame_count - 1) as f32)
            .collect(),
        FadeStyle::ToDarkAndBack => {
            let half = frame_count / 2;
            let down: Vec<f32> = (0..half)
                .map(|i| 1.0 - i as f32 / (half - 1) as f32)
                .collect();
            let up: Vec<f32> = (0..(frame_count - half))
                .map(|i| i as f32 / (frame_count - half - 1) as f32)
                .collect();
            [down, up].concat()
        }
        FadeStyle::FromDarkAndBack => {
            let half = frame_count / 2;
            let up: Vec<f32> = (0..half).map(|i| i as f32 / (half - 1) as f32).collect();
            let down: Vec<f32> = (0..(frame_count - half))
                .map(|i| 1.0 - i as f32 / (frame_count - half - 1) as f32)
                .collect();
            [up, down].concat()
        }
    };

    let tmpdir = tempdir().expect("Failed to create temp dir");

    for (i, factor) in fade_factors.iter().enumerate() {
        let faded = fade_image(&img, *factor);
        let path = tmpdir.path().join(format!("frame_{:04}.png", i));
        faded.save(&path).expect("Failed to save frame");
    }

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-framerate",
            &args.framerate.to_string(),
            "-i",
            &tmpdir.path().join("frame_%04d.png").to_string_lossy(),
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            &output_path.to_string_lossy(),
        ])
        .status()
        .expect("Failed to run ffmpeg");

    if !status.success() {
        eprintln!("FFmpeg failed");
    } else {
        println!("Video saved to {}", output_path.display());
    }
}

fn fade_image(img: &DynamicImage, alpha: f32) -> DynamicImage {
    let (width, height) = img.dimensions();
    let mut output = ImageBuffer::new(width, height);

    for (x, y, pixel) in img.to_rgba8().enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let faded_pixel = Rgba([
            ((r as f32) * alpha) as u8,
            ((g as f32) * alpha) as u8,
            ((b as f32) * alpha) as u8,
            a,
        ]);
        output.put_pixel(x, y, faded_pixel);
    }

    DynamicImage::ImageRgba8(output)
}
