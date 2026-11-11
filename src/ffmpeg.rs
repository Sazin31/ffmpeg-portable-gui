use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::io::{BufRead, BufReader};
use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// Windows process creation flag to suppress console window
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MediaInfo {
    pub duration_secs: f64,
    pub duration_str: String,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub subtitle_codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub file_size_bytes: u64,
}

#[derive(Clone, Debug, Default)]
pub struct HardwareSupport {
    pub nvenc_h264: bool,
    pub nvenc_hevc: bool,
    pub qsv_h264: bool,
    pub qsv_hevc: bool,
    pub amf_h264: bool,
    pub amf_hevc: bool,
}

impl HardwareSupport {
    pub fn has_any(&self) -> bool {
        self.nvenc_h264 || self.nvenc_hevc || self.qsv_h264 || self.qsv_hevc || self.amf_h264 || self.amf_hevc
    }

    pub fn summary(&self) -> String {
        let mut list = Vec::new();
        if self.nvenc_h264 || self.nvenc_hevc {
            list.push("NVIDIA NVENC");
        }
        if self.qsv_h264 || self.qsv_hevc {
            list.push("Intel QSV");
        }
        if self.amf_h264 || self.amf_hevc {
            list.push("AMD AMF");
        }
        if list.is_empty() {
            "Software Only".to_string()
        } else {
            list.join(", ")
        }
    }
}

/// Locate ffmpeg.exe:
/// 1. Same folder as this app
/// 2. `bin` subfolder next to app
/// 3. Current working directory or `bin/`
/// 4. System PATH
pub fn find_ffmpeg() -> Result<PathBuf, String> {
    // 1 & 2: Next to current exe or in exe's bin/ folder
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let next_to_exe = exe_dir.join("ffmpeg.exe");
            if next_to_exe.is_file() {
                return Ok(next_to_exe);
            }
            let in_bin = exe_dir.join("bin").join("ffmpeg.exe");
            if in_bin.is_file() {
                return Ok(in_bin);
            }
        }
    }

    // 3: Current working directory or bin/
    let cwd_ffmpeg = PathBuf::from("ffmpeg.exe");
    if cwd_ffmpeg.is_file() {
        return Ok(cwd_ffmpeg);
    }
    let cwd_bin_ffmpeg = PathBuf::from("bin").join("ffmpeg.exe");
    if cwd_bin_ffmpeg.is_file() {
        return Ok(cwd_bin_ffmpeg);
    }

    // 4: Check system PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("ffmpeg.exe");
            if candidate.is_file() {
                return Ok(candidate);
            }
            let candidate_bare = dir.join("ffmpeg");
            if candidate_bare.is_file() {
                return Ok(candidate_bare);
            }
        }
    }

    Err("ffmpeg.exe not found! Place ffmpeg.exe in the same folder as this app, in a 'bin' subfolder, or add it to your system PATH.".to_string())
}

/// Probes media file metadata by parsing stderr from `ffmpeg -i <file>`.
/// Never uses ffprobe, keeping the distribution minimal and portable.
pub fn probe_media(ffmpeg_path: &Path, file_path: &Path) -> Result<MediaInfo, String> {
    let mut cmd = Command::new(ffmpeg_path);
    cmd.arg("-hide_banner")
        .arg("-i")
        .arg(file_path);

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.stdout(Stdio::null()).stderr(Stdio::piped());

    let output = cmd.output().map_err(|e| format!("Failed to run ffmpeg probe: {}", e))?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut info = MediaInfo::default();
    if let Ok(metadata) = std::fs::metadata(file_path) {
        info.file_size_bytes = metadata.len();
    }

    parse_ffmpeg_probe_output(&stderr, &mut info);
    Ok(info)
}

/// Parse stderr output from `ffmpeg -i`
pub fn parse_ffmpeg_probe_output(text: &str, info: &mut MediaInfo) {
    for line in text.lines() {
        let trimmed = line.trim();

        // Parse Duration: 00:01:23.45, start: ...
        if trimmed.starts_with("Duration:") {
            if let Some(dur_part) = trimmed.split(',').next() {
                let dur_str = dur_part.trim_start_matches("Duration:").trim();
                info.duration_str = dur_str.to_string();
                info.duration_secs = parse_timestamp_seconds(dur_str);
            }
        }

        // Parse Stream lines: Stream #0:0[0x100]: Video: h264 ... 1920x1080 ...
        if trimmed.starts_with("Stream #") {
            if trimmed.contains("Video:") && info.video_codec.is_none() {
                if let Some(video_part) = trimmed.split("Video:").nth(1) {
                    let tokens: Vec<&str> = video_part.split(',').map(|s| s.trim()).collect();
                    if let Some(first) = tokens.first() {
                        let codec = first.split_whitespace().next().unwrap_or(first);
                        info.video_codec = Some(codec.to_string());
                    }

                    // Find resolution e.g. 1920x1080
                    for tok in &tokens {
                        if let Some((w, h)) = parse_resolution_token(tok) {
                            info.width = Some(w);
                            info.height = Some(h);
                            break;
                        }
                    }

                    // Find fps e.g. 30 fps
                    for tok in &tokens {
                        if tok.ends_with("fps") {
                            let num_str = tok.trim_end_matches("fps").trim();
                            if let Ok(f) = num_str.parse::<f64>() {
                                info.fps = Some(f);
                                break;
                            }
                        }
                    }
                }
            } else if trimmed.contains("Audio:") && info.audio_codec.is_none() {
                if let Some(audio_part) = trimmed.split("Audio:").nth(1) {
                    let first_token = audio_part.split(',').next().unwrap_or("").trim();
                    let codec = first_token.split_whitespace().next().unwrap_or(first_token);
                    info.audio_codec = Some(codec.to_string());
                }
            } else if trimmed.contains("Subtitle:") && info.subtitle_codec.is_none() {
                if let Some(sub_part) = trimmed.split("Subtitle:").nth(1) {
                    let first_token = sub_part.split(',').next().unwrap_or("").trim();
                    let codec = first_token.split_whitespace().next().unwrap_or(first_token);
                    info.subtitle_codec = Some(codec.to_string());
                }
            }
        }
    }
}

fn parse_resolution_token(tok: &str) -> Option<(u32, u32)> {
    let clean = tok.split_whitespace().next()?;
    let parts: Vec<&str> = clean.split('x').collect();
    if parts.len() == 2 {
        if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            if w > 0 && h > 0 && w < 16384 && h < 16384 {
                return Some((w, h));
            }
        }
    }
    None
}

pub fn parse_timestamp_seconds(s: &str) -> f64 {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        let h = parts[0].parse::<f64>().unwrap_or(0.0);
        let m = parts[1].parse::<f64>().unwrap_or(0.0);
        let s = parts[2].parse::<f64>().unwrap_or(0.0);
        h * 3600.0 + m * 60.0 + s
    } else if parts.len() == 2 {
        let m = parts[0].parse::<f64>().unwrap_or(0.0);
        let s = parts[1].parse::<f64>().unwrap_or(0.0);
        m * 60.0 + s
    } else {
        s.parse::<f64>().unwrap_or(0.0)
    }
}

pub fn format_file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Detect active hardware encoders by testing a 1-frame mock run
pub fn detect_hardware(ffmpeg_path: &Path) -> HardwareSupport {
    let mut hw = HardwareSupport::default();

    let test_encoder = |encoder: &str| -> bool {
        let mut cmd = Command::new(ffmpeg_path);
        cmd.arg("-v")
            .arg("error")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("nullsrc=s=256x256:d=0.04")
            .arg("-c:v")
            .arg(encoder)
            .arg("-f")
            .arg("null")
            .arg("-");

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        cmd.status().map(|s| s.success()).unwrap_or(false)
    };

    hw.nvenc_h264 = test_encoder("h264_nvenc");
    hw.nvenc_hevc = test_encoder("hevc_nvenc");
    hw.qsv_h264 = test_encoder("h264_qsv");
    hw.qsv_hevc = test_encoder("hevc_qsv");
    hw.amf_h264 = test_encoder("h264_amf");
    hw.amf_hevc = test_encoder("hevc_amf");

    hw
}

/// Generate collision-free output path (e.g., name_1.mp4, name_2.mp4)
/// Never overwrites user files!
pub fn get_safe_destination_path(dir: &Path, original_name: &str, target_ext: &str) -> PathBuf {
    let stem = Path::new(original_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let clean_ext = target_ext.trim_start_matches('.');

    let mut candidate = dir.join(format!("{}.{}", stem, clean_ext));
    if !candidate.exists() {
        return candidate;
    }

    let mut counter = 1;
    loop {
        candidate = dir.join(format!("{}_{}.{}", stem, counter, clean_ext));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

#[derive(Clone, Debug)]
pub enum FfmpegEvent {
    Started {
        file_index: usize,
        source_name: String,
        target_name: String,
    },
    Progress {
        file_index: usize,
        fraction: f32,
        speed: String,
        out_time: String,
    },
    FileSuccess {
        file_index: usize,
        output_path: PathBuf,
        was_fast_copy: bool,
    },
    FileFailed {
        file_index: usize,
        error_summary: String,
        stderr_tail: String,
    },
    Cancelled,
    QueueComplete {
        successful: usize,
        failed: usize,
    },
}

pub struct ActiveProcess {
    pub child: Child,
    pub partial_output: Option<PathBuf>,
}

#[derive(Clone, Default)]
pub struct ProcessController {
    active: Arc<Mutex<Option<ActiveProcess>>>,
    cancelled: Arc<AtomicBool>,
}

impl ProcessController {
    pub fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_active(&self, child: Child, partial_output: Option<PathBuf>) {
        let mut lock = self.active.lock().unwrap();
        *lock = Some(ActiveProcess {
            child,
            partial_output,
        });
    }

    pub fn clear_active(&self) {
        let mut lock = self.active.lock().unwrap();
        *lock = None;
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let mut lock = self.active.lock().unwrap();
        if let Some(mut proc) = lock.take() {
            let _ = proc.child.kill();
            let _ = proc.child.wait();
            if let Some(path) = proc.partial_output.take() {
                if path.exists() {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
        self.clear_active();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Runs a single conversion job and parses `-progress pipe:1 -nostats`
pub fn run_ffmpeg_job(
    ffmpeg_path: &Path,
    args: &[String],
    total_duration_secs: f64,
    output_path: &Path,
    file_index: usize,
    controller: &ProcessController,
    event_sender: &std::sync::mpsc::Sender<FfmpegEvent>,
) -> Result<(), (String, String)> {
    if controller.is_cancelled() {
        return Err(("Cancelled".to_string(), "Job cancelled before start".to_string()));
    }

    let mut cmd = Command::new(ffmpeg_path);
    cmd.args(args);

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        (
            format!("Failed to start ffmpeg: {}", e),
            String::new(),
        )
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        ("Failed to capture ffmpeg stdout".to_string(), String::new())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        ("Failed to capture ffmpeg stderr".to_string(), String::new())
    })?;

    controller.set_active(child, Some(output_path.to_path_buf()));

    // Collect stderr in background thread to display if failed
    let stderr_tail = Arc::new(Mutex::new(Vec::<String>::new()));
    let stderr_tail_clone = stderr_tail.clone();
    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            let mut list = stderr_tail_clone.lock().unwrap();
            if list.len() >= 30 {
                list.remove(0);
            }
            list.push(line);
        }
    });

    let reader = BufReader::new(stdout);
    let mut current_speed = "1.0x".to_string();
    let mut current_out_time = "00:00:00".to_string();

    for line_res in reader.lines() {
        if controller.is_cancelled() {
            break;
        }

        if let Ok(line) = line_res {
            let trimmed = line.trim();
            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim();
                let val = v.trim();

                match key {
                    "speed" => {
                        current_speed = val.to_string();
                    }
                    "out_time" => {
                        current_out_time = val.split('.').next().unwrap_or(val).to_string();
                    }
                    "out_time_us" => {
                        if let Ok(us) = val.parse::<u64>() {
                            let cur_secs = us as f64 / 1_000_000.0;
                            let frac = if total_duration_secs > 0.05 {
                                (cur_secs / total_duration_secs).clamp(0.0, 1.0) as f32
                            } else {
                                0.5
                            };

                            let _ = event_sender.send(FfmpegEvent::Progress {
                                file_index,
                                fraction: frac,
                                speed: current_speed.clone(),
                                out_time: current_out_time.clone(),
                            });
                        }
                    }
                    "progress" => {
                        if val == "end" {
                            let _ = event_sender.send(FfmpegEvent::Progress {
                                file_index,
                                fraction: 1.0,
                                speed: current_speed.clone(),
                                out_time: current_out_time.clone(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let _ = stderr_handle.join();

    if controller.is_cancelled() {
        // Clean up partial output
        if output_path.exists() {
            let _ = std::fs::remove_file(output_path);
        }
        controller.clear_active();
        return Err(("Cancelled".to_string(), "Process was cancelled".to_string()));
    }

    // Wait for process to exit
    let mut active_lock = controller.active.lock().unwrap();
    if let Some(mut proc) = active_lock.take() {
        let status = proc.child.wait().map_err(|e| {
            (format!("Failed to wait on ffmpeg: {}", e), String::new())
        })?;

        if !status.success() {
            // Delete partial or failed output
            if output_path.exists() {
                let _ = std::fs::remove_file(output_path);
            }
            let tail = stderr_tail.lock().unwrap().join("\n");
            let err_msg = format!("FFmpeg exited with status code {:?}", status.code());
            return Err((err_msg, tail));
        }
    }

    Ok(())
}
