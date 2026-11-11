use std::path::PathBuf;
use eframe::egui::{self, Color32, CornerRadius, Frame, RichText, Stroke, Vec2};
use crate::ffmpeg::{self, HardwareSupport, MediaInfo};
use crate::presets::{
    self, AppSettings, AudioChannelMode, AudioFormat, CompressionPreset, Flip, GifThumbnailMode,
    MaxResolution, OpusBitrate, PlaybackSpeed, Rotation, SubtitleAction, TabCategory,
    ToolSubCategory, TrimMode, VideoCodecChoice, VideoContainer,
};

#[derive(Clone, Debug)]
pub struct QueueItem {
    pub path: PathBuf,
    pub file_name: String,
    pub info: Option<MediaInfo>,
    pub probe_error: Option<String>,
    pub status: QueueItemStatus,
    pub progress_fraction: f32,
    pub speed: String,
    pub out_time: String,
    pub output_path: Option<PathBuf>,
    pub was_fast_copy: bool,
    pub error_details: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueueItemStatus {
    Pending,
    Probing,
    Converting,
    Done,
    Failed,
    Cancelled,
}

#[derive(Default)]
pub struct UiState {
    pub status_message: Option<String>,
    pub selected_queue_index: Option<usize>,
}

pub fn render_header(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    ffmpeg_result: &Result<PathBuf, String>,
    hw: &HardwareSupport,
) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("⚡ FFmpeg Essentials GUI").strong().size(18.0));
        ui.label(RichText::new("v0.1.0 (Portable)").weak().size(12.0));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Theme toggle
            let theme_label = if settings.dark_theme { "☀ Light" } else { "🌙 Dark" };
            if ui.button(theme_label).clicked() {
                settings.dark_theme = !settings.dark_theme;
                if settings.dark_theme {
                    ui.ctx().set_visuals(egui::Visuals::dark());
                } else {
                    ui.ctx().set_visuals(egui::Visuals::light());
                }
            }

            ui.separator();

            // HW support pill
            let hw_text = hw.summary();
            let hw_color = if hw.has_any() {
                Color32::from_rgb(46, 160, 67)
            } else {
                Color32::GRAY
            };
            ui.label(RichText::new(format!("GPU: {}", hw_text)).color(hw_color).size(12.0));

            ui.separator();

            // FFmpeg status
            match ffmpeg_result {
                Ok(_) => {
                    ui.label(RichText::new("✔ FFmpeg ready").color(Color32::from_rgb(46, 160, 67)).size(12.0));
                }
                Err(_) => {
                    ui.label(RichText::new("✖ FFmpeg missing!").color(Color32::from_rgb(230, 80, 80)).strong().size(12.0));
                }
            }
        });
    });

    if let Err(err_msg) = ffmpeg_result {
        ui.add_space(4.0);
        Frame::new()
            .fill(Color32::from_rgba_unmultiplied(200, 50, 50, 40))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(200, 50, 50)))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").size(18.0).color(Color32::from_rgb(230, 80, 80)));
                    ui.label(RichText::new(err_msg).size(13.0).color(Color32::from_rgb(240, 100, 100)));
                });
            });
    }
}

pub fn render_drop_zone(
    ui: &mut egui::Ui,
    is_hovered: bool,
    queue_len: usize,
    on_browse: impl FnOnce(),
) {
    let desired_height = if queue_len == 0 { 110.0 } else { 65.0 };
    let border_color = if is_hovered {
        Color32::from_rgb(33, 150, 243)
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke.color
    };

    let bg_color = if is_hovered {
        Color32::from_rgba_unmultiplied(33, 150, 243, 30)
    } else {
        ui.visuals().faint_bg_color
    };

    Frame::new()
        .fill(bg_color)
        .stroke(Stroke::new(if is_hovered { 2.0_f32 } else { 1.0_f32 }, border_color))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_min_height(desired_height);
            ui.vertical_centered(|ui| {
                if queue_len == 0 {
                    ui.add_space(6.0);
                    ui.label(RichText::new("📂 Drag & Drop media files here").size(16.0).strong());
                    ui.label(RichText::new("Supports video, audio, TS, MKV, MP4, WebM, MP3, WAV, etc.").weak().size(12.0));
                    ui.add_space(4.0);
                } else {
                    ui.label(RichText::new("➕ Drag more files to add to queue").weak().size(13.0));
                }

                if ui.button(RichText::new("📁 Browse Files...").size(13.0)).clicked() {
                    on_browse();
                }
            });
        });
}

pub fn render_queue(
    ui: &mut egui::Ui,
    queue: &mut Vec<QueueItem>,
    is_running: bool,
    selected_idx: &mut Option<usize>,
) {
    if queue.is_empty() {
        return;
    }

    let queue_len = queue.len();

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Queue ({} items)", queue_len)).strong());

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_enabled(!is_running, egui::Button::new("🗑 Clear All")).clicked() {
                queue.clear();
                *selected_idx = None;
            }
        });
    });

    ui.add_space(4.0);

    let max_height = 180.0;
    egui::ScrollArea::vertical()
        .max_height(max_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut item_to_remove = None;
            let mut move_up_idx = None;
            let mut move_down_idx = None;

            for (idx, item) in queue.iter_mut().enumerate() {
                let is_selected = *selected_idx == Some(idx);

                let card_stroke = if is_selected {
                    Stroke::new(1.5_f32, Color32::from_rgb(33, 150, 243))
                } else {
                    Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color)
                };

                let card_bg = if is_selected {
                    ui.visuals().selection.bg_fill.linear_multiply(0.2)
                } else {
                    ui.visuals().widgets.noninteractive.bg_fill
                };

                Frame::new()
                    .fill(card_bg)
                    .stroke(card_stroke)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Selection / file info
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    let name_text = RichText::new(&item.file_name).strong().size(13.0);
                                    if ui.selectable_label(is_selected, name_text).clicked() {
                                        *selected_idx = Some(idx);
                                    }

                                    if let Some(info) = &item.info {
                                        let size_str = ffmpeg::format_file_size(info.file_size_bytes);
                                        ui.label(RichText::new(format!("({})", size_str)).weak().size(11.0));

                                        if !info.duration_str.is_empty() {
                                            ui.label(RichText::new(format!("⏱ {}", info.duration_str)).size(11.0));
                                        }

                                        if let Some(v_codec) = &info.video_codec {
                                            let res_str = if let (Some(w), Some(h)) = (info.width, info.height) {
                                                format!("{}x{}", w, h)
                                            } else {
                                                String::new()
                                            };
                                            ui.label(
                                                RichText::new(format!("🎥 {} {}", v_codec, res_str))
                                                    .color(Color32::from_rgb(66, 165, 245))
                                                    .size(11.0),
                                            );
                                        }

                                        if let Some(a_codec) = &info.audio_codec {
                                            ui.label(
                                                RichText::new(format!("🔊 {}", a_codec))
                                                    .color(Color32::from_rgb(102, 187, 106))
                                                    .size(11.0),
                                            );
                                        }
                                    } else if item.status == QueueItemStatus::Probing {
                                        ui.label(RichText::new("Probing metadata...").weak().size(11.0));
                                    } else if let Some(err) = &item.probe_error {
                                        ui.label(RichText::new(format!("Probe error: {}", err)).color(Color32::RED).size(11.0));
                                    }
                                });

                                // Progress and status line
                                ui.horizontal(|ui| {
                                    match item.status {
                                        QueueItemStatus::Pending => {
                                            ui.label(RichText::new("⏳ Pending").weak().size(11.0));
                                        }
                                        QueueItemStatus::Probing => {
                                            ui.label(RichText::new("🔍 Reading codecs...").weak().size(11.0));
                                        }
                                        QueueItemStatus::Converting => {
                                            let pct = (item.progress_fraction * 100.0) as u32;
                                            ui.label(
                                                RichText::new(format!("⚙ Converting: {}% ({} - {})", pct, item.speed, item.out_time))
                                                    .color(Color32::from_rgb(33, 150, 243))
                                                    .size(11.0),
                                            );
                                        }
                                        QueueItemStatus::Done => {
                                            let mode_text = if item.was_fast_copy { " (Fast Copy)" } else { "" };
                                            ui.label(
                                                RichText::new(format!("✔ Complete{}", mode_text))
                                                    .color(Color32::from_rgb(46, 160, 67))
                                                    .strong()
                                                    .size(11.0),
                                            );
                                        }
                                        QueueItemStatus::Failed => {
                                            ui.label(RichText::new("✖ Failed").color(Color32::RED).strong().size(11.0));
                                        }
                                        QueueItemStatus::Cancelled => {
                                            ui.label(RichText::new("Cancelled").weak().size(11.0));
                                        }
                                    }

                                    if let Some(out_p) = &item.output_path {
                                        if let Some(out_name) = out_p.file_name().and_then(|s| s.to_str()) {
                                            ui.label(RichText::new(format!("➔ {}", out_name)).weak().size(11.0));
                                        }
                                    }
                                });

                                if item.status == QueueItemStatus::Converting {
                                    ui.add(
                                        egui::ProgressBar::new(item.progress_fraction)
                                            .animate(true)
                                            .show_percentage(),
                                    );
                                }
                            });

                            // Action buttons right aligned
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add_enabled(!is_running, egui::Button::new("❌").small()).clicked() {
                                    item_to_remove = Some(idx);
                                }
                                if ui.add_enabled(!is_running && idx + 1 < queue_len, egui::Button::new("⬇").small()).clicked() {
                                    move_down_idx = Some(idx);
                                }
                                if ui.add_enabled(!is_running && idx > 0, egui::Button::new("⬆").small()).clicked() {
                                    move_up_idx = Some(idx);
                                }
                            });
                        });

                        // Show error details if failed
                        if let Some(err_tail) = &item.error_details {
                            ui.add_space(2.0);
                            ui.label(RichText::new(err_tail).color(Color32::from_rgb(230, 100, 100)).size(10.0));
                        }
                    });
                ui.add_space(2.0);
            }

            if let Some(idx) = item_to_remove {
                queue.remove(idx);
                if *selected_idx == Some(idx) {
                    *selected_idx = None;
                }
            } else if let Some(idx) = move_up_idx {
                queue.swap(idx, idx - 1);
                if *selected_idx == Some(idx) {
                    *selected_idx = Some(idx - 1);
                }
            } else if let Some(idx) = move_down_idx {
                queue.swap(idx, idx + 1);
                if *selected_idx == Some(idx) {
                    *selected_idx = Some(idx + 1);
                }
            }
        });
}

pub fn render_tabs(ui: &mut egui::Ui, settings: &mut AppSettings) {
    ui.horizontal(|ui| {
        let tabs = [
            (TabCategory::Convert, "🎬 Convert Video"),
            (TabCategory::Audio, "🎵 Audio / Extract"),
            (TabCategory::Compress, "🗜 Compress"),
            (TabCategory::Trim, "✂ Trim / Cut"),
            (TabCategory::Join, "🔗 Join Files"),
            (TabCategory::Tools, "🛠 Tools & Filters"),
        ];

        for (cat, label) in tabs {
            let is_active = settings.active_tab == cat;
            if ui.selectable_label(is_active, RichText::new(label).strong()).clicked() {
                settings.active_tab = cat;
            }
        }
    });
}

pub fn render_options_panel(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    selected_item: Option<&QueueItem>,
    hw: &HardwareSupport,
) {
    Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(12.0)
        .show(ui, |ui| {
            match settings.active_tab {
                TabCategory::Convert => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Target Format:").strong());
                        for container in VideoContainer::all() {
                            ui.radio_value(&mut settings.video_target_container, *container, container.label());
                        }
                    });

                    ui.add_space(6.0);
                    ui.checkbox(&mut settings.auto_remux, "⚡ Fast Remux (-c copy) when compatible (Instant, zero quality loss)");

                    if let Some(item) = selected_item {
                        if let Some(info) = &item.info {
                            let compatible = presets::is_fast_remux_compatible(info, settings.video_target_container);
                            if compatible && settings.auto_remux {
                                ui.label(
                                    RichText::new("✨ Fast copy mode will be used for this file (No re-encoding needed!)")
                                        .color(Color32::from_rgb(46, 160, 67))
                                        .strong()
                                        .size(12.0),
                                );
                            }
                        }
                    }
                }

                TabCategory::Audio => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Target Format:").strong());
                        for fmt in AudioFormat::all() {
                            ui.radio_value(&mut settings.audio_target_format, *fmt, fmt.label());
                        }
                    });

                    ui.add_space(6.0);

                    if settings.audio_target_format == AudioFormat::Opus {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Opus Bitrate:").strong());
                            for b in OpusBitrate::all() {
                                ui.radio_value(&mut settings.opus_bitrate, *b, b.label());
                            }
                        });
                    } else if settings.audio_target_format == AudioFormat::Mp3 {
                        ui.label(RichText::new("Using libmp3lame with high quality VBR (-q:a 2) by default.").weak().size(12.0));
                    }

                    if let Some(item) = selected_item {
                        if let Some(info) = &item.info {
                            let cur_codec = info.audio_codec.as_deref().unwrap_or("").to_lowercase();
                            let target_codec = settings.audio_target_format.codec_name();
                            if cur_codec == target_codec {
                                ui.label(
                                    RichText::new("✨ Source is already target codec: lossless -c:a copy will be used!")
                                        .color(Color32::from_rgb(46, 160, 67))
                                        .strong()
                                        .size(12.0),
                                );
                            }
                        }
                    }
                }

                TabCategory::Compress => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Preset:").strong());
                        for p in CompressionPreset::all() {
                            ui.radio_value(&mut settings.compression_preset, *p, p.label());
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Video Codec:").strong());
                        ui.radio_value(&mut settings.compression_codec, VideoCodecChoice::H264, "H.264 (Maximum compatibility)");
                        ui.radio_value(&mut settings.compression_codec, VideoCodecChoice::H265, "H.265 / HEVC (Higher compression)");
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Max Resolution:").strong());
                        for res in MaxResolution::all() {
                            ui.radio_value(&mut settings.max_resolution, *res, res.label());
                        }
                    });
                }

                TabCategory::Trim => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Start Time:").strong());
                        ui.text_edit_singleline(&mut settings.trim_start);
                        ui.label("(e.g. 00:01:30 or 90)");

                        ui.add_space(16.0);
                        ui.label(RichText::new("End Time:").strong());
                        ui.text_edit_singleline(&mut settings.trim_end);
                        ui.label("(e.g. 00:02:45 or 165)");
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Mode:").strong());
                        ui.radio_value(&mut settings.trim_mode, TrimMode::FastCopy, "⚡ Fast (Copy stream, instant, keyframe snap)");
                        ui.radio_value(&mut settings.trim_mode, TrimMode::PreciseEncode, "🎯 Precise (Re-encode, exact frame cut)");
                    });
                }

                TabCategory::Join => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Output Merged Filename:").strong());
                        ui.text_edit_singleline(&mut settings.join_output_name);
                    });
                    ui.label(RichText::new("Uses the concat demuxer (-c copy) for zero quality loss. Ensure files have matching codecs!").weak().size(12.0));
                    ui.label(RichText::new("Use the ⬆ and ⬇ buttons in the queue above to reorder files.").weak().size(12.0));
                }

                TabCategory::Tools => {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut settings.tool_category, ToolSubCategory::Transform, "🔄 Transform / Speed");
                        ui.selectable_value(&mut settings.tool_category, ToolSubCategory::AudioTrack, "🔇 Audio Track");
                        ui.selectable_value(&mut settings.tool_category, ToolSubCategory::GifThumbnail, "🖼 GIF / Thumbnail");
                        ui.selectable_value(&mut settings.tool_category, ToolSubCategory::Subtitles, "💬 Subtitles");
                        ui.selectable_value(&mut settings.tool_category, ToolSubCategory::AudioFilters, "🎚 Audio Filters");
                    });

                    ui.separator();

                    match settings.tool_category {
                        ToolSubCategory::Transform => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Resize:").strong());
                                for res in MaxResolution::all() {
                                    ui.radio_value(&mut settings.transform_resize, *res, res.label());
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Rotate:").strong());
                                ui.radio_value(&mut settings.transform_rotate, Rotation::None, "None");
                                ui.radio_value(&mut settings.transform_rotate, Rotation::Rotate90Cw, "90° CW");
                                ui.radio_value(&mut settings.transform_rotate, Rotation::Rotate180, "180°");
                                ui.radio_value(&mut settings.transform_rotate, Rotation::Rotate270Cw, "270° CW");
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Flip:").strong());
                                ui.radio_value(&mut settings.transform_flip, Flip::None, "None");
                                ui.radio_value(&mut settings.transform_flip, Flip::Horizontal, "Horizontal");
                                ui.radio_value(&mut settings.transform_flip, Flip::Vertical, "Vertical");
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Speed:").strong());
                                for spd in PlaybackSpeed::all() {
                                    ui.radio_value(&mut settings.transform_speed, *spd, spd.label());
                                }
                            });
                        }

                        ToolSubCategory::AudioTrack => {
                            ui.checkbox(&mut settings.audio_remove, "🔇 Remove all audio tracks (-an)");

                            if !settings.audio_remove {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Replace / Add Audio Track:").strong());
                                    if let Some(path) = &settings.audio_replace_file {
                                        ui.label(path.file_name().and_then(|s| s.to_str()).unwrap_or("audio file"));
                                        if ui.button("Clear").clicked() {
                                            settings.audio_replace_file = None;
                                        }
                                    } else if ui.button("Select Audio File...").clicked() {
                                        if let Some(file) = rfd::FileDialog::new()
                                            .add_filter("Audio Files", &["mp3", "aac", "m4a", "wav", "flac", "ogg", "opus"])
                                            .pick_file()
                                        {
                                            settings.audio_replace_file = Some(file);
                                        }
                                    }
                                });
                            }
                        }

                        ToolSubCategory::GifThumbnail => {
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut settings.gif_thumb_mode, GifThumbnailMode::VideoToGif, "🎬 Video to High-Quality GIF");
                                ui.radio_value(&mut settings.gif_thumb_mode, GifThumbnailMode::ExtractThumbnail, "📷 Extract Image / Thumbnail");
                            });

                            ui.add_space(4.0);

                            match settings.gif_thumb_mode {
                                GifThumbnailMode::VideoToGif => {
                                    ui.horizontal(|ui| {
                                        ui.label("GIF FPS:");
                                        ui.add(egui::DragValue::new(&mut settings.gif_fps).range(5..=30));
                                        ui.label("Width (px):");
                                        ui.add(egui::DragValue::new(&mut settings.gif_width).range(160..=1280));
                                    });
                                    ui.label(RichText::new("Uses 2-phase palette generation for smooth gradients and crisp colors.").weak().size(11.0));
                                }
                                GifThumbnailMode::ExtractThumbnail => {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Extract Timestamp:").strong());
                                        ui.text_edit_singleline(&mut settings.thumb_time);
                                        ui.label("(e.g. 00:00:05 or 5)");
                                    });
                                }
                            }
                        }

                        ToolSubCategory::Subtitles => {
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut settings.subtitle_action, SubtitleAction::ExtractSrt, "Extract subtitle to .srt (-map 0:s:0)");
                                ui.radio_value(&mut settings.subtitle_action, SubtitleAction::BurnIn, "Burn in subtitles (hardcode into video)");
                            });

                            if settings.subtitle_action == SubtitleAction::BurnIn {
                                ui.horizontal(|ui| {
                                    if let Some(sub) = &settings.subtitle_file {
                                        ui.label(sub.file_name().and_then(|s| s.to_str()).unwrap_or("subtitle"));
                                        if ui.button("Clear").clicked() {
                                            settings.subtitle_file = None;
                                        }
                                    } else if ui.button("Select .srt File...").clicked() {
                                        if let Some(file) = rfd::FileDialog::new().add_filter("Subtitles", &["srt", "ass", "vtt"]).pick_file() {
                                            settings.subtitle_file = Some(file);
                                        }
                                    }
                                });
                            }
                        }

                        ToolSubCategory::AudioFilters => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Volume (%):").strong());
                                ui.add(egui::Slider::new(&mut settings.volume_percent, 10..=300).suffix("%"));
                                if ui.button("Reset (100%)").clicked() {
                                    settings.volume_percent = 100;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.checkbox(&mut settings.audio_normalize, "Normalize Audio (EBU R128 loudnorm filter)");
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Channels:").strong());
                                ui.radio_value(&mut settings.audio_channels, AudioChannelMode::Original, "Keep Original");
                                ui.radio_value(&mut settings.audio_channels, AudioChannelMode::Stereo, "Stereo (2ch)");
                                ui.radio_value(&mut settings.audio_channels, AudioChannelMode::Mono, "Mono (1ch)");
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Bitrate:").strong());
                                ui.add(egui::Slider::new(&mut settings.audio_bitrate_kbps, 64..=320).suffix(" kbps"));
                            });
                        }
                    }
                }
            }

            ui.add_space(8.0);
            ui.separator();

            // Destination folder selector & Hardware toggle
            ui.horizontal(|ui| {
                ui.label(RichText::new("Output Folder:").strong());
                let same_source = settings.custom_output_folder.is_none();
                if ui.selectable_label(same_source, "📁 Same as source").clicked() {
                    settings.custom_output_folder = None;
                }

                let custom_chosen = settings.custom_output_folder.is_some();
                let custom_btn_text = if let Some(p) = &settings.custom_output_folder {
                    p.file_name().and_then(|s| s.to_str()).unwrap_or("Custom folder")
                } else {
                    "Choose Custom Folder..."
                };

                if ui.selectable_label(custom_chosen, custom_btn_text).clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        settings.custom_output_folder = Some(folder);
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if hw.has_any() {
                        ui.checkbox(&mut settings.hardware_accel, "🚀 GPU Hardware Acceleration");
                    }
                });
            });
        });
}

pub fn render_command_box(
    ui: &mut egui::Ui,
    cmd_str: &str,
    ui_state: &mut UiState,
) {
    ui.collapsing("💻 Show Command", |ui| {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::multiline(&mut cmd_str.to_string())
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            );

            if ui.button("📋 Copy").clicked() {
                ui.ctx().copy_text(cmd_str.to_string());
                ui_state.status_message = Some("Command copied to clipboard!".to_string());
            }
        });
    });
}

pub fn render_bottom_bar(
    ui: &mut egui::Ui,
    queue: &[QueueItem],
    is_running: bool,
    overall_progress: f32,
    on_convert: impl FnOnce(),
    on_cancel: impl FnOnce(),
    on_open_folder: impl FnOnce(),
) {
    ui.horizontal(|ui| {
        let has_pending = queue.iter().any(|item| item.status == QueueItemStatus::Pending || item.status == QueueItemStatus::Converting);
        let can_convert = !is_running && has_pending;

        // Big action button
        let btn_text = if is_running { "⏳ Converting..." } else { "▶ Start Conversion" };
        let convert_btn = egui::Button::new(RichText::new(btn_text).size(15.0).strong())
            .min_size(Vec2::new(150.0, 36.0));

        if ui.add_enabled(can_convert, convert_btn).clicked() {
            on_convert();
        }

        if is_running {
            if ui.button(RichText::new("⏹ Cancel").size(14.0).color(Color32::RED)).clicked() {
                on_cancel();
            }
        }

        ui.add_space(8.0);

        // Overall progress
        if is_running || overall_progress > 0.0 {
            ui.add(
                egui::ProgressBar::new(overall_progress)
                    .animate(is_running)
                    .show_percentage(),
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("📂 Open Output Folder").clicked() {
                on_open_folder();
            }
        });
    });
}
