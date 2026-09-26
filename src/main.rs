#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use eframe::egui;
use ico::{IconDir, IconDirEntry, IconImage, ResourceType};

const SIZES: &[u32] = &[16, 32, 48, 64, 128, 256];

#[derive(Default, PartialEq)]
enum Status {
    #[default]
    Empty,
    Ok,
    Warn,
    Err,
}

#[derive(Default)]
struct App {
    file_name: String,
    report: String,
    status: Status,
    out_path: Option<PathBuf>,
}

fn convert_image(in_path: &Path) -> Result<(PathBuf, Vec<String>), String> {
    let mut warnings = Vec::new();

    if !in_path.exists() {
        return Err("File not found.".into());
    }

    let ext = in_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext != "png" && ext != "jpg" && ext != "jpeg" {
        return Err("PNG or JPG file required.".into());
    }

    let out_path = in_path.with_extension("ico");

    if out_path.exists() {
        warnings.push("Existing ICO will be overwritten.".into());
    }

    let fmt = if ext == "png" {
        image::ImageFormat::Png
    } else {
        image::ImageFormat::Jpeg
    };

    let file = File::open(in_path).map_err(|e| format!("Cannot open file: {e}."))?;
    let img = image::load(BufReader::new(file), fmt)
        .map_err(|e| format!("Cannot decode image: {e}."))?;

    let (w0, h0) = (img.width(), img.height());

    if w0 < 16 || h0 < 16 {
        warnings.push(format!(
            "Image is very small ({w0}x{h0}) - ICO may look blurry."
        ));
    }

    let square = if w0 != h0 {
        let side = w0.min(h0);
        let x = (w0 - side) / 2;
        let y = (h0 - side) / 2;
        warnings.push(format!(
            "Image is not square ({w0}x{h0}) - cropped center {side}x{side}."
        ));
        img.crop_imm(x, y, side, side)
    } else {
        img
    };

    let rgba = square.to_rgba8();

    let mut dir = IconDir::new(ResourceType::Icon);

    for &size in SIZES {
        let small = image::imageops::resize(&rgba, size, size, image::imageops::Triangle);
        let pic = IconImage::from_rgba_data(size, size, small.into_raw());
        let entry =
            IconDirEntry::encode(&pic).map_err(|e| format!("Failed to encode ICO: {e}."))?;
        dir.add_entry(entry);
    }

    let out =
        File::create(&out_path).map_err(|e| format!("Cannot save ICO: {e}."))?;
    dir.write(out)
        .map_err(|e| format!("Cannot write ICO: {e}."))?;

    Ok((out_path, warnings))
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.heading("Image to ICO");
                ui.label("Click the button and pick a PNG or JPG file.");
                ui.add_space(15.0);

                let btn =
                    egui::Button::new("Select file...").min_size(egui::vec2(280.0, 60.0));

                if ui.add(btn).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg"])
                        .pick_file()
                    {
                        self.file_name = path
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();

                        match convert_image(&path) {
                            Ok((out, warns)) => {
                                self.out_path = Some(out.clone());
                                if warns.is_empty() {
                                    self.status = Status::Ok;
                                    self.report =
                                        format!("Done! Saved to:\n{}", out.display());
                                } else {
                                    self.status = Status::Warn;
                                    self.report = format!(
                                        "Done with notes:\n- {}\n\nSaved to:\n{}",
                                        warns.join("\n- "),
                                        out.display()
                                    );
                                }
                            }
                            Err(e) => {
                                self.status = Status::Err;
                                self.report = format!("Failed: {e}");
                                self.out_path = None;
                            }
                        }
                    }
                }

                ui.add_space(10.0);

                if !self.file_name.is_empty() {
                    ui.label(format!("File: {}", self.file_name));
                }

                ui.add_space(10.0);

                let color = match self.status {
                    Status::Ok => egui::Color32::DARK_GREEN,
                    Status::Warn => egui::Color32::from_rgb(150, 110, 0),
                    Status::Err => egui::Color32::RED,
                    Status::Empty => egui::Color32::GRAY,
                };

                if !self.report.is_empty() {
                    ui.colored_label(color, &self.report);
                }

                if self.status == Status::Ok || self.status == Status::Warn {
                    ui.add_space(10.0);
                    if ui.button("Open result folder").clicked() {
                        if let Some(out) = &self.out_path {
                            let folder = out
                                .parent()
                                .map(PathBuf::from)
                                .unwrap_or_else(|| PathBuf::from("."));
                            let _ = std::process::Command::new("explorer")
                                .arg(folder)
                                .spawn();
                        }
                    }
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 420.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "ICO Converter",
        options,
        Box::new(|_| Ok(Box::new(App::default()))),
    )
}
