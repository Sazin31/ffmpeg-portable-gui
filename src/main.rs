#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ffmpeg;
mod presets;
mod ui;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use eframe::egui::{self, Context, RichText, ViewportBuilder};
use ffmpeg::{FfmpegEvent, HardwareSupport, ProcessController};
use presets::AppSettings;
use ui::{QueueItem, QueueItemStatus, UiState};

const SETTINGS_FILENAME: &str = "settings.json";

#[cfg(windows)]
extern "system" {
    fn MessageBeep(uType: u32) -> i32;
}

fn get_settings_path() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            return parent.join(SETTINGS_FILENAME);
        }
    }
    PathBuf::from(SETTINGS_FILENAME)
}

fn load_settings() -> AppSettings {
    let path = get_settings_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                return settings;
            }
        }
    }
    AppSettings::default()
}

fn save_settings(settings: &AppSettings) {
    let path = get_settings_path();
    if let Ok(content) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(path, content);
    }
}

pub struct FfmpegGuiApp {
    ffmpeg_path: Result<PathBuf, String>,
    hardware_support: HardwareSupport,
    settings: AppSettings,
    queue: Vec<QueueItem>,
    ui_state: UiState,

    is_converting: bool,
    overall_progress: f32,

    controller: ProcessController,
    event_tx: Sender<FfmpegEvent>,
    event_rx: Receiver<FfmpegEvent>,

    probe_tx: Sender<(usize, Result<ffmpeg::MediaInfo, String>)>,
    probe_rx: Receiver<(usize, Result<ffmpeg::MediaInfo, String>)>,
}

impl FfmpegGuiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = load_settings();

        if settings.dark_theme {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
        } else {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
        }

        let ffmpeg_res = ffmpeg::find_ffmpeg();
        let hw_support = if let Ok(ref ff) = ffmpeg_res {
            ffmpeg::detect_hardware(ff)
        } else {
            HardwareSupport::default()
        };

        let (event_tx, event_rx) = channel();
        let (probe_tx, probe_rx) = channel();

        Self {
            ffmpeg_path: ffmpeg_res,
            hardware_support: hw_support,
            settings,
            queue: Vec::new(),
            ui_state: UiState::default(),

            is_converting: false,
            overall_progress: 0.0,

            controller: ProcessController::new(),
            event_tx,
            event_rx,

            probe_tx,
            probe_rx,
        }
    }

    fn add_files_to_queue(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if !path.is_file() {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Check if already in queue
            if self.queue.iter().any(|item| item.path == path) {
                continue;
            }

            let idx = self.queue.len();
            self.queue.push(QueueItem {
                path: path.clone(),
                file_name,
                info: None,
                probe_error: None,
                status: QueueItemStatus::Probing,
                progress_fraction: 0.0,
                speed: String::new(),
                out_time: String::new(),
                output_path: None,
                was_fast_copy: false,
                error_details: None,
            });

            // Launch background probe thread
            if let Ok(ffmpeg_p) = self.ffmpeg_path.clone() {
                let p_tx = self.probe_tx.clone();
                thread::spawn(move || {
                    let res = ffmpeg::probe_media(&ffmpeg_p, &path);
                    let _ = p_tx.send((idx, res));
                });
            }
        }
    }

    fn start_conversion_queue(&mut self) {
        if self.is_converting || self.queue.is_empty() {
            return;
        }

        let ffmpeg_p = match &self.ffmpeg_path {
            Ok(p) => p.clone(),
            Err(_) => return,
        };

        self.is_converting = true;
        self.overall_progress = 0.0;
        self.controller.reset();

        // Mark pending files
        for item in &mut self.queue {
            if item.status != QueueItemStatus::Done {
                item.status = QueueItemStatus::Pending;
                item.progress_fraction = 0.0;
                item.error_details = None;
            }
        }

        let queue_snapshot = self.queue.clone();
        let settings_snapshot = self.settings.clone();
        let hw_snapshot = self.hardware_support.clone();
        let controller_clone = self.controller.clone();
        let event_tx_clone = self.event_tx.clone();

        thread::spawn(move || {
            let total_jobs = queue_snapshot.len();
            let mut successful = 0;
            let mut failed = 0;

            // Handle Join tab as a special multi-file merge job
            if settings_snapshot.active_tab == presets::TabCategory::Join && total_jobs > 1 {
                let dest_dir = if let Some(custom) = &settings_snapshot.custom_output_folder {
                    custom.clone()
                } else if let Some(parent) = queue_snapshot[0].path.parent() {
                    parent.to_path_buf()
                } else {
                    PathBuf::from(".")
                };

                // Create temporary concat demuxer file
                let temp_concat_file = dest_dir.join("_concat_temp_list.txt");
                let mut concat_content = String::new();
                for item in &queue_snapshot {
                    let escaped_path = item.path.to_string_lossy().replace('\\', "/").replace('\'', "'\\''");
                    concat_content.push_str(&format!("file '{}'\n", escaped_path));
                }

                if let Err(e) = std::fs::write(&temp_concat_file, &concat_content) {
                    let _ = event_tx_clone.send(FfmpegEvent::FileFailed {
                        file_index: 0,
                        error_summary: format!("Failed to create concat playlist: {}", e),
                        stderr_tail: String::new(),
                    });
                    return;
                }

                let join_cmd = presets::build_join_command(
                    &temp_concat_file,
                    &settings_snapshot.join_output_name,
                    &dest_dir,
                );

                let _ = event_tx_clone.send(FfmpegEvent::Started {
                    file_index: 0,
                    source_name: format!("{} files merged", total_jobs),
                    target_name: settings_snapshot.join_output_name.clone(),
                });

                let total_duration: f64 = queue_snapshot.iter().map(|item| {
                    item.info.as_ref().map(|i| i.duration_secs).unwrap_or(0.0)
                }).sum();

                let res = ffmpeg::run_ffmpeg_job(
                    &ffmpeg_p,
                    &join_cmd.args,
                    total_duration,
                    &join_cmd.output_path,
                    0,
                    &controller_clone,
                    &event_tx_clone,
                );

                let _ = std::fs::remove_file(&temp_concat_file);

                match res {
                    Ok(_) => {
                        let _ = event_tx_clone.send(FfmpegEvent::FileSuccess {
                            file_index: 0,
                            output_path: join_cmd.output_path,
                            was_fast_copy: true,
                        });
                        successful += 1;
                    }
                    Err((err_sum, stderr_tail)) => {
                        let _ = event_tx_clone.send(FfmpegEvent::FileFailed {
                            file_index: 0,
                            error_summary: err_sum,
                            stderr_tail,
                        });
                        failed += 1;
                    }
                }

                let _ = event_tx_clone.send(FfmpegEvent::QueueComplete { successful, failed });
                return;
            }

            // Normal per-file queue processing
            for (idx, item) in queue_snapshot.iter().enumerate() {
                if controller_clone.is_cancelled() {
                    let _ = event_tx_clone.send(FfmpegEvent::Cancelled);
                    break;
                }

                if item.status == QueueItemStatus::Done {
                    continue;
                }

                let dest_dir = if let Some(custom) = &settings_snapshot.custom_output_folder {
                    custom.clone()
                } else if let Some(parent) = item.path.parent() {
                    parent.to_path_buf()
                } else {
                    PathBuf::from(".")
                };

                let media_info = item.info.clone().unwrap_or_default();

                // Build command
                let mut cmd_obj = presets::build_job_command(
                    &item.path,
                    &media_info,
                    &settings_snapshot,
                    &hw_snapshot,
                    &dest_dir,
                    false, // initial attempt with HW acceleration if requested
                );

                let target_name = cmd_obj
                    .output_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output")
                    .to_string();

                let _cmd_display = presets::format_command_display("ffmpeg", &cmd_obj.args);

                let _ = event_tx_clone.send(FfmpegEvent::Started {
                    file_index: idx,
                    source_name: item.file_name.clone(),
                    target_name,
                });

                // Attempt execution
                let mut res = ffmpeg::run_ffmpeg_job(
                    &ffmpeg_p,
                    &cmd_obj.args,
                    media_info.duration_secs,
                    &cmd_obj.output_path,
                    idx,
                    &controller_clone,
                    &event_tx_clone,
                );

                // Hardware fallback requirement: if hardware encoder fails, retry with software
                if res.is_err() && cmd_obj.is_hardware && !controller_clone.is_cancelled() {
                    let fallback_cmd = presets::build_job_command(
                        &item.path,
                        &media_info,
                        &settings_snapshot,
                        &hw_snapshot,
                        &dest_dir,
                        true, // force software fallback!
                    );

                    let _fallback_display = presets::format_command_display("ffmpeg", &fallback_cmd.args);
                    let _ = event_tx_clone.send(FfmpegEvent::Started {
                        file_index: idx,
                        source_name: item.file_name.clone(),
                        target_name: fallback_cmd.output_path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string(),
                    });

                    res = ffmpeg::run_ffmpeg_job(
                        &ffmpeg_p,
                        &fallback_cmd.args,
                        media_info.duration_secs,
                        &fallback_cmd.output_path,
                        idx,
                        &controller_clone,
                        &event_tx_clone,
                    );

                    cmd_obj = fallback_cmd;
                }

                match res {
                    Ok(_) => {
                        successful += 1;
                        let _ = event_tx_clone.send(FfmpegEvent::FileSuccess {
                            file_index: idx,
                            output_path: cmd_obj.output_path,
                            was_fast_copy: cmd_obj.was_fast_copy,
                        });
                    }
                    Err((err_msg, tail)) => {
                        if controller_clone.is_cancelled() {
                            let _ = event_tx_clone.send(FfmpegEvent::Cancelled);
                            break;
                        } else {
                            failed += 1;
                            let _ = event_tx_clone.send(FfmpegEvent::FileFailed {
                                file_index: idx,
                                error_summary: err_msg,
                                stderr_tail: tail,
                            });
                        }
                    }
                }
            }

            let _ = event_tx_clone.send(FfmpegEvent::QueueComplete { successful, failed });
        });
    }

    fn cancel_conversion(&mut self) {
        self.controller.cancel();
        self.is_converting = false;
        for item in &mut self.queue {
            if item.status == QueueItemStatus::Converting || item.status == QueueItemStatus::Pending {
                item.status = QueueItemStatus::Cancelled;
            }
        }
    }

    fn open_output_folder(&self) {
        let folder = if let Some(custom) = &self.settings.custom_output_folder {
            custom.clone()
        } else if let Some(first) = self.queue.first() {
            first.path.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            PathBuf::from(".")
        };

        #[cfg(windows)]
        {
            let _ = std::process::Command::new("explorer").arg(&folder).spawn();
        }
    }

    fn notify_completion(&self) {
        #[cfg(windows)]
        unsafe {
            MessageBeep(0);
        }
    }

    fn get_current_preview_command(&self) -> String {
        let target_item = self
            .ui_state
            .selected_queue_index
            .and_then(|idx| self.queue.get(idx))
            .or_else(|| self.queue.first());

        if let Some(item) = target_item {
            let dest_dir = if let Some(custom) = &self.settings.custom_output_folder {
                custom.clone()
            } else if let Some(parent) = item.path.parent() {
                parent.to_path_buf()
            } else {
                PathBuf::from(".")
            };

            let media_info = item.info.clone().unwrap_or_default();
            let cmd_obj = presets::build_job_command(
                &item.path,
                &media_info,
                &self.settings,
                &self.hardware_support,
                &dest_dir,
                false,
            );

            presets::format_command_display("ffmpeg", &cmd_obj.args)
        } else {
            "ffmpeg -i input.mp4 ... output.mp4 (Drop or browse a file to see command)".to_string()
        }
    }
}

impl eframe::App for FfmpegGuiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Handle native drag and drop files onto the window
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            let paths: Vec<PathBuf> = dropped_files.into_iter().filter_map(|f| f.path).collect();
            if !paths.is_empty() {
                self.add_files_to_queue(paths);
            }
        }

        // Receive probe results
        while let Ok((idx, res)) = self.probe_rx.try_recv() {
            if let Some(item) = self.queue.get_mut(idx) {
                match res {
                    Ok(info) => {
                        item.info = Some(info);
                        item.status = QueueItemStatus::Pending;
                    }
                    Err(err) => {
                        item.probe_error = Some(err);
                        item.status = QueueItemStatus::Pending;
                    }
                }
            }
        }

        // Receive ffmpeg execution events
        while let Ok(evt) = self.event_rx.try_recv() {
            match evt {
                FfmpegEvent::Started { file_index, source_name, target_name, .. } => {
                    if let Some(item) = self.queue.get_mut(file_index) {
                        item.status = QueueItemStatus::Converting;
                    }
                    self.ui_state.status_message = Some(format!("Converting {} ➔ {}", source_name, target_name));
                }
                FfmpegEvent::Progress { file_index, fraction, speed, out_time } => {
                    if let Some(item) = self.queue.get_mut(file_index) {
                        item.progress_fraction = fraction;
                        item.speed = speed;
                        item.out_time = out_time;
                    }

                    // Compute overall progress
                    let total = self.queue.len();
                    if total > 0 {
                        let completed = self.queue.iter().filter(|i| i.status == QueueItemStatus::Done).count() as f32;
                        let in_progress = self.queue.get(file_index).map(|i| i.progress_fraction).unwrap_or(0.0);
                        self.overall_progress = (completed + in_progress) / total as f32;
                    }
                }
                FfmpegEvent::FileSuccess { file_index, output_path, was_fast_copy } => {
                    if let Some(item) = self.queue.get_mut(file_index) {
                        item.status = QueueItemStatus::Done;
                        item.progress_fraction = 1.0;
                        item.output_path = Some(output_path);
                        item.was_fast_copy = was_fast_copy;
                    }
                }
                FfmpegEvent::FileFailed { file_index, error_summary, stderr_tail } => {
                    if let Some(item) = self.queue.get_mut(file_index) {
                        item.status = QueueItemStatus::Failed;
                        let snippet = stderr_tail.lines().rev().take(6).collect::<Vec<_>>();
                        let formatted_tail = snippet.into_iter().rev().collect::<Vec<_>>().join("\n");
                        item.error_details = Some(format!("{}\n{}", error_summary, formatted_tail));
                    }
                }
                FfmpegEvent::Cancelled => {
                    self.is_converting = false;
                    self.ui_state.status_message = Some("Conversion cancelled by user.".to_string());
                }
                FfmpegEvent::QueueComplete { successful, failed } => {
                    self.is_converting = false;
                    self.overall_progress = 1.0;
                    self.notify_completion();
                    self.ui_state.status_message = Some(format!(
                        "Queue finished: {} succeeded, {} failed.",
                        successful, failed
                    ));
                    save_settings(&self.settings);
                }
            }
        }

        if self.is_converting {
            ctx.request_repaint();
        }

        let is_hovered = ctx.input(|i| !i.raw.hovered_files.is_empty());

        egui::CentralPanel::default().show(ctx, |ui| {
            // Header
            ui::render_header(ui, &mut self.settings, &self.ffmpeg_path, &self.hardware_support);
            ui.add_space(8.0);

            // Drop zone
            let mut browse_clicked = false;
            ui::render_drop_zone(ui, is_hovered, self.queue.len(), || {
                browse_clicked = true;
            });

            if browse_clicked {
                if let Some(files) = rfd::FileDialog::new()
                    .add_filter("Media Files", &["mp4", "mkv", "ts", "avi", "mov", "webm", "mp3", "m4a", "opus", "wav", "flac", "ogg"])
                    .pick_files()
                {
                    self.add_files_to_queue(files);
                }
            }

            ui.add_space(8.0);

            // Queue List
            ui::render_queue(ui, &mut self.queue, self.is_converting, &mut self.ui_state.selected_queue_index);

            ui.add_space(8.0);

            // Preset Tabs
            ui::render_tabs(ui, &mut self.settings);

            ui.add_space(4.0);

            // Options Panel for active preset tab
            let selected_item = self
                .ui_state
                .selected_queue_index
                .and_then(|i| self.queue.get(i))
                .or_else(|| self.queue.first());

            ui::render_options_panel(ui, &mut self.settings, selected_item, &self.hardware_support);

            ui.add_space(6.0);

            // Collapsible Show Command
            let current_cmd = self.get_current_preview_command();
            ui::render_command_box(ui, &current_cmd, &mut self.ui_state);

            // Status message
            if let Some(msg) = &self.ui_state.status_message {
                ui.label(RichText::new(msg).weak().size(11.0));
            }

            ui.add_space(6.0);
            ui.separator();

            // Bottom action bar
            let mut convert_clicked = false;
            let mut cancel_clicked = false;
            let mut open_folder_clicked = false;

            ui::render_bottom_bar(
                ui,
                &self.queue,
                self.is_converting,
                self.overall_progress,
                || convert_clicked = true,
                || cancel_clicked = true,
                || open_folder_clicked = true,
            );

            if convert_clicked {
                self.start_conversion_queue();
            }
            if cancel_clicked {
                self.cancel_conversion();
            }
            if open_folder_clicked {
                self.open_output_folder();
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        save_settings(&self.settings);
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([880.0, 720.0])
            .with_min_inner_size([640.0, 500.0])
            .with_title("FFmpeg Essentials GUI")
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "FFmpeg Essentials GUI",
        native_options,
        Box::new(|cc| Ok(Box::new(FfmpegGuiApp::new(cc)))),
    )
}
