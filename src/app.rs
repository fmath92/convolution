use eframe::egui;
use egui::{ColorImage, TextureHandle, TextureOptions};
use image::GrayImage;

const PREVIEW_MAX_SIZE: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelShape {
    ThreeBySix,
    SixByThree,
}

impl KernelShape {
    fn width(self) -> usize {
        match self {
            Self::ThreeBySix => 3,
            Self::SixByThree => 6,
        }
    }

    fn height(self) -> usize {
        match self {
            Self::ThreeBySix => 6,
            Self::SixByThree => 3,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ThreeBySix => "3 x 6",
            Self::SixByThree => "6 x 3",
        }
    }
}

#[derive(Default)]
struct LoadedImage {
    name: String,
    gray: Option<GrayImage>,
    texture: Option<TextureHandle>,
}

#[derive(Clone)]
struct ConvolutionPreview {
    score: f32,
    width: usize,
    height: usize,
    bytes: Vec<u8>,
}

pub struct ConvolutionApp {
    slide: LoadedImage,
    kernels_sheet: LoadedImage,
    kernel_shape: KernelShape,
    kernels: Vec<Vec<f32>>,
    kernel_rows: usize,
    kernel_cols: usize,
    previews: Vec<ConvolutionPreview>,
    selected_kernel: usize,
    status: String,
}

impl Default for ConvolutionApp {
    fn default() -> Self {
        Self {
            slide: LoadedImage::default(),
            kernels_sheet: LoadedImage::default(),
            kernel_shape: KernelShape::ThreeBySix,
            kernels: Vec::new(),
            kernel_rows: 0,
            kernel_cols: 0,
            previews: Vec::new(),
            selected_kernel: 0,
            status: "Drop two PNG files in the window: first the histological slide, then the kernels sheet.".to_owned(),
        }
    }
}

impl ConvolutionApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }

        for file in dropped {
            if let Some(bytes) = extract_bytes(&file) {
                if self.slide.gray.is_none() {
                    self.load_png_into_slot(ctx, bytes, file.name, true);
                } else if self.kernels_sheet.gray.is_none() {
                    self.load_png_into_slot(ctx, bytes, file.name, false);
                } else {
                    self.status = "Both image slots are already filled. Use Reset to load different files.".to_owned();
                }
            } else {
                self.status = "Could not read dropped file bytes.".to_owned();
            }
        }
    }

    fn load_png_into_slot(
        &mut self,
        ctx: &egui::Context,
        bytes: Vec<u8>,
        file_name: String,
        is_slide: bool,
    ) {
        match image::load_from_memory(&bytes) {
            Ok(img) => {
                let gray = img.to_luma8();
                let color = gray_to_color_image(&gray);
                let texture = ctx.load_texture(
                    if is_slide { "slide_texture" } else { "kernel_texture" },
                    color,
                    TextureOptions::LINEAR,
                );

                let target = if is_slide {
                    &mut self.slide
                } else {
                    &mut self.kernels_sheet
                };

                target.name = file_name;
                target.gray = Some(gray);
                target.texture = Some(texture);
                self.kernels.clear();
                self.previews.clear();
                self.selected_kernel = 0;
                self.status = "Image loaded. Choose kernel shape and press Split kernels.".to_owned();
            }
            Err(e) => {
                self.status = format!("Failed to decode PNG: {e}");
            }
        }
    }

    fn split_kernels(&mut self) {
        let Some(sheet) = self.kernels_sheet.gray.as_ref() else {
            self.status = "Load the kernels sheet first.".to_owned();
            return;
        };

        let kw = self.kernel_shape.width() as u32;
        let kh = self.kernel_shape.height() as u32;
        if sheet.width() % kw != 0 || sheet.height() % kh != 0 {
            self.status = format!(
                "Kernel sheet size {}x{} is not divisible by kernel size {}x{}.",
                sheet.width(),
                sheet.height(),
                kw,
                kh
            );
            return;
        }

        self.kernel_cols = (sheet.width() / kw) as usize;
        self.kernel_rows = (sheet.height() / kh) as usize;
        self.kernels.clear();
        self.previews.clear();
        self.selected_kernel = 0;

        for row in 0..self.kernel_rows {
            for col in 0..self.kernel_cols {
                let mut kernel = Vec::with_capacity((kw * kh) as usize);
                for ky in 0..kh {
                    for kx in 0..kw {
                        let px = sheet.get_pixel(col as u32 * kw + kx, row as u32 * kh + ky)[0];
                        let centered = (px as f32 / 255.0) * 2.0 - 1.0;
                        kernel.push(centered);
                    }
                }
                self.kernels.push(kernel);
            }
        }

        self.status = format!(
            "Split into {} kernels ({} rows x {} cols).",
            self.kernels.len(),
            self.kernel_rows,
            self.kernel_cols
        );
    }

    fn run_all_convolutions(&mut self) {
        let Some(slide) = self.slide.gray.as_ref() else {
            self.status = "Load the histological slide first.".to_owned();
            return;
        };
        if self.kernels.is_empty() {
            self.status = "Split kernels first.".to_owned();
            return;
        }

        let input = gray_to_f32(slide);
        let width = slide.width() as usize;
        let height = slide.height() as usize;
        let kw = self.kernel_shape.width();
        let kh = self.kernel_shape.height();

        self.previews.clear();
        self.previews.reserve(self.kernels.len());

        for kernel in &self.kernels {
            let response = convolve_same(&input, width, height, kernel, kw, kh);
            let score = response.iter().map(|v| v.abs()).sum::<f32>() / response.len() as f32;
            let (pw, ph, bytes) = build_preview(&response, width, height, PREVIEW_MAX_SIZE);
            self.previews.push(ConvolutionPreview {
                score,
                width: pw,
                height: ph,
                bytes,
            });
        }

        self.status = format!("Computed {} convolution maps.", self.previews.len());
    }
}

impl eframe::App for ConvolutionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_dropped_files(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("WASM Convolution Explorer");
            ui.label("Drop PNG files in order: 1) lame histologique 2) kernels sheet.");
            ui.label(format!("Status: {}", self.status));
        });

        egui::SidePanel::left("controls").show(ctx, |ui| {
            ui.group(|ui| {
                ui.label("Kernel shape");
                ui.radio_value(
                    &mut self.kernel_shape,
                    KernelShape::ThreeBySix,
                    KernelShape::ThreeBySix.label(),
                );
                ui.radio_value(
                    &mut self.kernel_shape,
                    KernelShape::SixByThree,
                    KernelShape::SixByThree.label(),
                );
            });

            if ui.button("Split kernels").clicked() {
                self.split_kernels();
            }
            if ui.button("Run all convolutions").clicked() {
                self.run_all_convolutions();
            }
            if ui.button("Reset").clicked() {
                *self = Self::default();
            }

            ui.separator();
            ui.label(format!(
                "Kernels: {} ({} rows x {} cols)",
                self.kernels.len(),
                self.kernel_rows,
                self.kernel_cols
            ));

            if !self.previews.is_empty() {
                self.selected_kernel = self
                    .selected_kernel
                    .min(self.previews.len().saturating_sub(1));
                ui.add(
                    egui::Slider::new(&mut self.selected_kernel, 0..=self.previews.len() - 1)
                        .text("Kernel index"),
                );
                let score = self.previews[self.selected_kernel].score;
                ui.label(format!("Selected score (mean abs response): {:.5}", score));
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                columns[0].heading("Input images");
                if let Some(tex) = &self.slide.texture {
                    columns[0].label(format!("Slide: {}", self.slide.name));
                    let size = tex.size_vec2();
                    let scale = (420.0 / size.x.max(size.y)).min(1.0);
                    columns[0].image((tex.id(), size * scale));
                } else {
                    columns[0].label("Slide not loaded.");
                }
                columns[0].separator();
                if let Some(tex) = &self.kernels_sheet.texture {
                    columns[0].label(format!("Kernels sheet: {}", self.kernels_sheet.name));
                    let size = tex.size_vec2();
                    let scale = (420.0 / size.x.max(size.y)).min(1.0);
                    columns[0].image((tex.id(), size * scale));
                } else {
                    columns[0].label("Kernels sheet not loaded.");
                }

                columns[1].heading("Convolution preview");
                if let Some(preview) = self.previews.get(self.selected_kernel) {
                    let color = ColorImage::from_gray([preview.width, preview.height], &preview.bytes);
                    let tex = ctx.load_texture(
                        format!("preview_{}", self.selected_kernel),
                        color,
                        TextureOptions::LINEAR,
                    );
                    let size = tex.size_vec2();
                    let scale = (520.0 / size.x.max(size.y)).min(1.0);
                    columns[1].image((tex.id(), size * scale));
                    columns[1].label(format!(
                        "Kernel {} preview size: {}x{}",
                        self.selected_kernel, preview.width, preview.height
                    ));
                } else {
                    columns[1].label("No convolution result yet.");
                }
            });
        });
    }
}

fn gray_to_color_image(gray: &GrayImage) -> ColorImage {
    let bytes = gray.as_raw();
    ColorImage::from_gray([gray.width() as usize, gray.height() as usize], bytes)
}

fn gray_to_f32(gray: &GrayImage) -> Vec<f32> {
    gray.pixels().map(|p| p[0] as f32 / 255.0).collect()
}

fn convolve_same(
    input: &[f32],
    width: usize,
    height: usize,
    kernel: &[f32],
    kw: usize,
    kh: usize,
) -> Vec<f32> {
    let mut output = vec![0.0; width * height];
    let kcx = kw / 2;
    let kcy = kh / 2;

    for y in 0..height {
        for x in 0..width {
            let mut acc = 0.0;
            for ky in 0..kh {
                for kx in 0..kw {
                    let ix = x as isize + kx as isize - kcx as isize;
                    let iy = y as isize + ky as isize - kcy as isize;
                    if ix >= 0 && iy >= 0 && ix < width as isize && iy < height as isize {
                        let i = iy as usize * width + ix as usize;
                        let k = ky * kw + kx;
                        acc += input[i] * kernel[k];
                    }
                }
            }
            output[y * width + x] = acc;
        }
    }
    output
}

fn build_preview(
    src: &[f32],
    width: usize,
    height: usize,
    max_dim: usize,
) -> (usize, usize, Vec<u8>) {
    let scale = (max_dim as f32 / width.max(height) as f32).min(1.0);
    let out_w = ((width as f32 * scale).round() as usize).max(1);
    let out_h = ((height as f32 * scale).round() as usize).max(1);
    let resized = resize_nearest(src, width, height, out_w, out_h);
    let (min_v, max_v) = min_max(&resized);
    let range = (max_v - min_v).max(1e-6);
    let bytes = resized
        .into_iter()
        .map(|v| (((v - min_v) / range) * 255.0).clamp(0.0, 255.0) as u8)
        .collect();
    (out_w, out_h, bytes)
}

fn resize_nearest(src: &[f32], src_w: usize, src_h: usize, dst_w: usize, dst_h: usize) -> Vec<f32> {
    let mut out = vec![0.0; dst_w * dst_h];
    for y in 0..dst_h {
        for x in 0..dst_w {
            let sx = x * src_w / dst_w;
            let sy = y * src_h / dst_h;
            out[y * dst_w + x] = src[sy * src_w + sx];
        }
    }
    out
}

fn min_max(values: &[f32]) -> (f32, f32) {
    let mut min_v = f32::INFINITY;
    let mut max_v = f32::NEG_INFINITY;
    for &v in values {
        if v < min_v {
            min_v = v;
        }
        if v > max_v {
            max_v = v;
        }
    }
    if min_v.is_infinite() || max_v.is_infinite() {
        (0.0, 0.0)
    } else {
        (min_v, max_v)
    }
}

fn extract_bytes(file: &egui::DroppedFile) -> Option<Vec<u8>> {
    if let Some(bytes) = &file.bytes {
        return Some(bytes.to_vec());
    }

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = &file.path {
        return std::fs::read(path).ok();
    }

    None
}
