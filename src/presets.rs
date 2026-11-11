use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::ffmpeg::{HardwareSupport, MediaInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabCategory {
    Convert,
    Audio,
    Compress,
    Trim,
    Join,
    Tools,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoContainer {
    Mp4,
    Mkv,
    Webm,
    Avi,
    Mov,
    Ts,
}

impl VideoContainer {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::Webm => "webm",
            Self::Avi => "avi",
            Self::Mov => "mov",
            Self::Ts => "ts",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Mp4 => "MP4 (Universal)",
            Self::Mkv => "MKV (Matroska)",
            Self::Webm => "WebM (Web Optimized)",
            Self::Avi => "AVI (Legacy)",
            Self::Mov => "MOV (QuickTime)",
            Self::Ts => "TS (Transport Stream)",
        }
    }

    pub fn all() -> &'static [VideoContainer] {
        &[
            Self::Mp4,
            Self::Mkv,
            Self::Webm,
            Self::Avi,
            Self::Mov,
            Self::Ts,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    Aac,
    Opus,
    Flac,
    Wav,
    Ogg,
}

impl AudioFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Aac => "m4a",
            Self::Opus => "opus",
            Self::Flac => "flac",
            Self::Wav => "wav",
            Self::Ogg => "ogg",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Mp3 => "MP3 (libmp3lame)",
            Self::Aac => "AAC / M4A",
            Self::Opus => "Opus (High Efficiency)",
            Self::Flac => "FLAC (Lossless)",
            Self::Wav => "WAV (Uncompressed PCM)",
            Self::Ogg => "OGG (Vorbis)",
        }
    }

    pub fn codec_name(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Aac => "aac",
            Self::Opus => "opus",
            Self::Flac => "flac",
            Self::Wav => "pcm_s16le",
            Self::Ogg => "vorbis",
        }
    }

    pub fn all() -> &'static [AudioFormat] {
        &[
            Self::Mp3,
            Self::Aac,
            Self::Opus,
            Self::Flac,
            Self::Wav,
            Self::Ogg,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpusBitrate {
    B32k,
    B64k,
    B96k,
    B128k,
}

impl OpusBitrate {
    pub fn label(&self) -> &'static str {
        match self {
            Self::B32k => "32 kbps (Voice / Speech)",
            Self::B64k => "64 kbps (Podcasts / Clean Voice)",
            Self::B96k => "96 kbps (Standard Music)",
            Self::B128k => "128 kbps (High Fidelity Music)",
        }
    }

    pub fn value(&self) -> &'static str {
        match self {
            Self::B32k => "32k",
            Self::B64k => "64k",
            Self::B96k => "96k",
            Self::B128k => "128k",
        }
    }

    pub fn all() -> &'static [OpusBitrate] {
        &[Self::B32k, Self::B64k, Self::B96k, Self::B128k]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionPreset {
    Small,    // CRF 28
    Balanced, // CRF 23
    High,     // CRF 18
}

impl CompressionPreset {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Small => "Small Size (CRF 28, max compression)",
            Self::Balanced => "Balanced (CRF 23, recommended)",
            Self::High => "High Quality (CRF 18, visually lossless)",
        }
    }

    pub fn crf(&self) -> u32 {
        match self {
            Self::Small => 28,
            Self::Balanced => 23,
            Self::High => 18,
        }
    }

    pub fn all() -> &'static [CompressionPreset] {
        &[Self::Small, Self::Balanced, Self::High]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodecChoice {
    H264,
    H265,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaxResolution {
    Original,
    P1080,
    P720,
    P480,
}

impl MaxResolution {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Original => "Original Resolution",
            Self::P1080 => "Max 1080p (1920x1080)",
            Self::P720 => "Max 720p (1280x720)",
            Self::P480 => "Max 480p (854x480)",
        }
    }

    pub fn scale_filter(&self) -> Option<&'static str> {
        match self {
            Self::Original => None,
            Self::P1080 => Some("scale='min(1920,iw)':-2"),
            Self::P720 => Some("scale='min(1280,iw)':-2"),
            Self::P480 => Some("scale='min(854,iw)':-2"),
        }
    }

    pub fn all() -> &'static [MaxResolution] {
        &[Self::Original, Self::P1080, Self::P720, Self::P480]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrimMode {
    FastCopy,
    PreciseEncode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rotation {
    None,
    Rotate90Cw,
    Rotate180,
    Rotate270Cw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flip {
    None,
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackSpeed {
    Half,
    ThreeQuarters,
    Normal,
    OneAndQuarter,
    OneAndHalf,
    Double,
}

impl PlaybackSpeed {
    pub fn factor(&self) -> f64 {
        match self {
            Self::Half => 0.5,
            Self::ThreeQuarters => 0.75,
            Self::Normal => 1.0,
            Self::OneAndQuarter => 1.25,
            Self::OneAndHalf => 1.5,
            Self::Double => 2.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Half => "0.5x (Slow motion)",
            Self::ThreeQuarters => "0.75x",
            Self::Normal => "1.0x (Normal)",
            Self::OneAndQuarter => "1.25x",
            Self::OneAndHalf => "1.5x",
            Self::Double => "2.0x (Double speed)",
        }
    }

    pub fn all() -> &'static [PlaybackSpeed] {
        &[
            Self::Half,
            Self::ThreeQuarters,
            Self::Normal,
            Self::OneAndQuarter,
            Self::OneAndHalf,
            Self::Double,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubtitleAction {
    ExtractSrt,
    BurnIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GifThumbnailMode {
    VideoToGif,
    ExtractThumbnail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioChannelMode {
    Original,
    Stereo,
    Mono,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub active_tab: TabCategory,
    pub dark_theme: bool,
    pub hardware_accel: bool,
    pub custom_output_folder: Option<PathBuf>,
    pub auto_remux: bool,

    // Convert tab settings
    pub video_target_container: VideoContainer,

    // Audio tab settings
    pub audio_target_format: AudioFormat,
    pub opus_bitrate: OpusBitrate,
    pub extract_audio_only: bool,

    // Compress tab settings
    pub compression_preset: CompressionPreset,
    pub compression_codec: VideoCodecChoice,
    pub max_resolution: MaxResolution,

    // Trim tab settings
    pub trim_start: String,
    pub trim_end: String,
    pub trim_mode: TrimMode,

    // Join tab settings
    pub join_output_name: String,

    // Tools tab settings
    pub tool_category: ToolSubCategory,
    pub transform_resize: MaxResolution,
    pub transform_rotate: Rotation,
    pub transform_flip: Flip,
    pub transform_speed: PlaybackSpeed,

    pub audio_remove: bool,
    pub audio_replace_file: Option<PathBuf>,

    pub gif_thumb_mode: GifThumbnailMode,
    pub gif_fps: u32,
    pub gif_width: u32,
    pub thumb_time: String,

    pub subtitle_action: SubtitleAction,
    pub subtitle_file: Option<PathBuf>,

    pub volume_percent: u32,
    pub audio_normalize: bool,
    pub audio_channels: AudioChannelMode,
    pub audio_bitrate_kbps: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolSubCategory {
    Transform,
    AudioTrack,
    GifThumbnail,
    Subtitles,
    AudioFilters,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            active_tab: TabCategory::Convert,
            dark_theme: true,
            hardware_accel: true,
            custom_output_folder: None,
            auto_remux: true,

            video_target_container: VideoContainer::Mp4,

            audio_target_format: AudioFormat::Mp3,
            opus_bitrate: OpusBitrate::B96k,
            extract_audio_only: false,

            compression_preset: CompressionPreset::Balanced,
            compression_codec: VideoCodecChoice::H264,
            max_resolution: MaxResolution::Original,

            trim_start: "00:00:00".to_string(),
            trim_end: "00:00:10".to_string(),
            trim_mode: TrimMode::FastCopy,

            join_output_name: "merged_output.mp4".to_string(),

            tool_category: ToolSubCategory::Transform,
            transform_resize: MaxResolution::Original,
            transform_rotate: Rotation::None,
            transform_flip: Flip::None,
            transform_speed: PlaybackSpeed::Normal,

            audio_remove: false,
            audio_replace_file: None,

            gif_thumb_mode: GifThumbnailMode::VideoToGif,
            gif_fps: 15,
            gif_width: 480,
            thumb_time: "00:00:01".to_string(),

            subtitle_action: SubtitleAction::ExtractSrt,
            subtitle_file: None,

            volume_percent: 100,
            audio_normalize: false,
            audio_channels: AudioChannelMode::Original,
            audio_bitrate_kbps: 192,
        }
    }
}

/// Checks whether source video and audio codecs are compatible with the target container for `-c copy`
pub fn is_fast_remux_compatible(
    source_media: &MediaInfo,
    target_container: VideoContainer,
) -> bool {
    let v_codec = source_media.video_codec.as_deref().unwrap_or("").to_lowercase();
    let a_codec = source_media.audio_codec.as_deref().unwrap_or("").to_lowercase();

    match target_container {
        VideoContainer::Mp4 => {
            let v_ok = v_codec.is_empty() || v_codec == "h264" || v_codec == "hevc" || v_codec == "av1" || v_codec == "mpeg4";
            let a_ok = a_codec.is_empty() || a_codec == "aac" || a_codec == "mp3" || a_codec == "opus" || a_codec == "ac3" || a_codec == "eac3";
            v_ok && a_ok && (!v_codec.is_empty() || !a_codec.is_empty())
        }
        VideoContainer::Mkv => {
            // MKV supports almost all common video and audio codecs
            !v_codec.is_empty() || !a_codec.is_empty()
        }
        VideoContainer::Webm => {
            let v_ok = v_codec.is_empty() || v_codec == "vp8" || v_codec == "vp9" || v_codec == "av1";
            let a_ok = a_codec.is_empty() || a_codec == "opus" || a_codec == "vorbis";
            v_ok && a_ok && (!v_codec.is_empty() || !a_codec.is_empty())
        }
        VideoContainer::Avi => {
            let v_ok = v_codec.is_empty() || v_codec == "mpeg4" || v_codec == "mjpeg";
            let a_ok = a_codec.is_empty() || a_codec == "mp3" || a_codec == "ac3" || a_codec.starts_with("pcm_");
            v_ok && a_ok && (!v_codec.is_empty() || !a_codec.is_empty())
        }
        VideoContainer::Mov => {
            let v_ok = v_codec.is_empty() || v_codec == "h264" || v_codec == "hevc" || v_codec == "prores";
            let a_ok = a_codec.is_empty() || a_codec == "alac" || a_codec == "aac" || a_codec.starts_with("pcm_");
            v_ok && a_ok && (!v_codec.is_empty() || !a_codec.is_empty())
        }
        VideoContainer::Ts => {
            let v_ok = v_codec.is_empty() || v_codec == "h264" || v_codec == "hevc" || v_codec == "mpeg2video";
            let a_ok = a_codec.is_empty() || a_codec == "aac" || a_codec == "mp3" || a_codec == "ac3";
            v_ok && a_ok && (!v_codec.is_empty() || !a_codec.is_empty())
        }
    }
}

pub struct BuiltCommand {
    pub args: Vec<String>,
    pub output_path: PathBuf,
    pub was_fast_copy: bool,
    pub is_hardware: bool,
}

/// Generates the ffmpeg CLI arguments and target file path for a file job
pub fn build_job_command(
    file_path: &Path,
    media_info: &MediaInfo,
    settings: &AppSettings,
    hw: &HardwareSupport,
    dest_dir: &Path,
    force_software: bool,
) -> BuiltCommand {
    let mut args = Vec::new();
    let mut was_fast_copy = false;
    let mut is_hardware = false;

    // Standard progress flags: -hide_banner -y -progress pipe:1 -nostats
    args.push("-hide_banner".to_string());
    args.push("-y".to_string());
    args.push("-progress".to_string());
    args.push("pipe:1".to_string());
    args.push("-nostats".to_string());

    let (target_ext, built_args) = match settings.active_tab {
        TabCategory::Convert => {
            let container = settings.video_target_container;
            let ext = container.extension();

            let mut tab_args = Vec::new();

            // Check if fast remux mode applies
            let can_remux = settings.auto_remux && is_fast_remux_compatible(media_info, container);
            if can_remux {
                was_fast_copy = true;
                tab_args.push("-i".to_string());
                tab_args.push(file_path.to_string_lossy().to_string());
                tab_args.push("-c".to_string());
                tab_args.push("copy".to_string());
            } else {
                tab_args.push("-i".to_string());
                tab_args.push(file_path.to_string_lossy().to_string());

                // Choose video encoder
                let use_hw = settings.hardware_accel && !force_software;
                match container {
                    VideoContainer::Mp4 | VideoContainer::Mov | VideoContainer::Ts => {
                        if use_hw && hw.nvenc_h264 {
                            is_hardware = true;
                            tab_args.push("-c:v".to_string());
                            tab_args.push("h264_nvenc".to_string());
                        } else if use_hw && hw.qsv_h264 {
                            is_hardware = true;
                            tab_args.push("-c:v".to_string());
                            tab_args.push("h264_qsv".to_string());
                        } else if use_hw && hw.amf_h264 {
                            is_hardware = true;
                            tab_args.push("-c:v".to_string());
                            tab_args.push("h264_amf".to_string());
                        } else {
                            tab_args.push("-c:v".to_string());
                            tab_args.push("libx264".to_string());
                            tab_args.push("-preset".to_string());
                            tab_args.push("medium".to_string());
                            tab_args.push("-crf".to_string());
                            tab_args.push("23".to_string());
                        }
                        tab_args.push("-c:a".to_string());
                        tab_args.push("aac".to_string());
                        tab_args.push("-b:a".to_string());
                        tab_args.push("192k".to_string());
                    }
                    VideoContainer::Mkv => {
                        if use_hw && hw.nvenc_h264 {
                            is_hardware = true;
                            tab_args.push("-c:v".to_string());
                            tab_args.push("h264_nvenc".to_string());
                        } else {
                            tab_args.push("-c:v".to_string());
                            tab_args.push("libx264".to_string());
                            tab_args.push("-crf".to_string());
                            tab_args.push("23".to_string());
                        }
                        tab_args.push("-c:a".to_string());
                        tab_args.push("aac".to_string());
                    }
                    VideoContainer::Webm => {
                        tab_args.push("-c:v".to_string());
                        tab_args.push("libvpx-vp9".to_string());
                        tab_args.push("-crf".to_string());
                        tab_args.push("30".to_string());
                        tab_args.push("-b:v".to_string());
                        tab_args.push("0".to_string());
                        tab_args.push("-c:a".to_string());
                        tab_args.push("libopus".to_string());
                        tab_args.push("-b:a".to_string());
                        tab_args.push("128k".to_string());
                    }
                    VideoContainer::Avi => {
                        tab_args.push("-c:v".to_string());
                        tab_args.push("mpeg4".to_string());
                        tab_args.push("-qscale:v".to_string());
                        tab_args.push("3".to_string());
                        tab_args.push("-c:a".to_string());
                        tab_args.push("libmp3lame".to_string());
                        tab_args.push("-q:a".to_string());
                        tab_args.push("2".to_string());
                    }
                }
            }
            (ext, tab_args)
        }

        TabCategory::Audio => {
            let format = settings.audio_target_format;
            let ext = format.extension();
            let mut tab_args = Vec::new();

            tab_args.push("-i".to_string());
            tab_args.push(file_path.to_string_lossy().to_string());

            // Always strip video when doing audio tab conversions
            tab_args.push("-vn".to_string());

            let current_a_codec = media_info.audio_codec.as_deref().unwrap_or("").to_lowercase();
            let target_codec = format.codec_name();

            // If source audio is already target codec, use lossless copy!
            if current_a_codec == target_codec {
                was_fast_copy = true;
                tab_args.push("-c:a".to_string());
                tab_args.push("copy".to_string());
            } else {
                match format {
                    AudioFormat::Mp3 => {
                        tab_args.push("-c:a".to_string());
                        tab_args.push("libmp3lame".to_string());
                        tab_args.push("-q:a".to_string());
                        tab_args.push("2".to_string()); // High quality VBR ~190 kbps
                    }
                    AudioFormat::Opus => {
                        tab_args.push("-c:a".to_string());
                        tab_args.push("libopus".to_string());
                        tab_args.push("-b:a".to_string());
                        tab_args.push(settings.opus_bitrate.value().to_string());
                    }
                    AudioFormat::Aac => {
                        tab_args.push("-c:a".to_string());
                        tab_args.push("aac".to_string());
                        tab_args.push("-b:a".to_string());
                        tab_args.push("192k".to_string());
                    }
                    AudioFormat::Flac => {
                        tab_args.push("-c:a".to_string());
                        tab_args.push("flac".to_string());
                    }
                    AudioFormat::Wav => {
                        tab_args.push("-c:a".to_string());
                        tab_args.push("pcm_s16le".to_string());
                    }
                    AudioFormat::Ogg => {
                        tab_args.push("-c:a".to_string());
                        tab_args.push("libvorbis".to_string());
                        tab_args.push("-q:a".to_string());
                        tab_args.push("4".to_string());
                    }
                }
            }
            (ext, tab_args)
        }

        TabCategory::Compress => {
            let mut tab_args = Vec::new();
            tab_args.push("-i".to_string());
            tab_args.push(file_path.to_string_lossy().to_string());

            let crf_val = settings.compression_preset.crf();
            let use_hw = settings.hardware_accel && !force_software;

            match settings.compression_codec {
                VideoCodecChoice::H264 => {
                    if use_hw && hw.nvenc_h264 {
                        is_hardware = true;
                        tab_args.push("-c:v".to_string());
                        tab_args.push("h264_nvenc".to_string());
                        tab_args.push("-rc".to_string());
                        tab_args.push("vbr".to_string());
                        tab_args.push("-cq".to_string());
                        tab_args.push(crf_val.to_string());
                    } else if use_hw && hw.qsv_h264 {
                        is_hardware = true;
                        tab_args.push("-c:v".to_string());
                        tab_args.push("h264_qsv".to_string());
                        tab_args.push("-global_quality".to_string());
                        tab_args.push(crf_val.to_string());
                    } else if use_hw && hw.amf_h264 {
                        is_hardware = true;
                        tab_args.push("-c:v".to_string());
                        tab_args.push("h264_amf".to_string());
                        tab_args.push("-rc".to_string());
                        tab_args.push("cqp".to_string());
                        tab_args.push("-qp_i".to_string());
                        tab_args.push(crf_val.to_string());
                        tab_args.push("-qp_p".to_string());
                        tab_args.push(crf_val.to_string());
                    } else {
                        tab_args.push("-c:v".to_string());
                        tab_args.push("libx264".to_string());
                        tab_args.push("-crf".to_string());
                        tab_args.push(crf_val.to_string());
                        tab_args.push("-preset".to_string());
                        tab_args.push("medium".to_string());
                    }
                }
                VideoCodecChoice::H265 => {
                    if use_hw && hw.nvenc_hevc {
                        is_hardware = true;
                        tab_args.push("-c:v".to_string());
                        tab_args.push("hevc_nvenc".to_string());
                        tab_args.push("-rc".to_string());
                        tab_args.push("vbr".to_string());
                        tab_args.push("-cq".to_string());
                        tab_args.push(crf_val.to_string());
                    } else if use_hw && hw.qsv_hevc {
                        is_hardware = true;
                        tab_args.push("-c:v".to_string());
                        tab_args.push("hevc_qsv".to_string());
                        tab_args.push("-global_quality".to_string());
                        tab_args.push(crf_val.to_string());
                    } else if use_hw && hw.amf_hevc {
                        is_hardware = true;
                        tab_args.push("-c:v".to_string());
                        tab_args.push("hevc_amf".to_string());
                        tab_args.push("-rc".to_string());
                        tab_args.push("cqp".to_string());
                        tab_args.push("-qp_i".to_string());
                        tab_args.push(crf_val.to_string());
                        tab_args.push("-qp_p".to_string());
                        tab_args.push(crf_val.to_string());
                    } else {
                        tab_args.push("-c:v".to_string());
                        tab_args.push("libx265".to_string());
                        tab_args.push("-crf".to_string());
                        tab_args.push(crf_val.to_string());
                        tab_args.push("-preset".to_string());
                        tab_args.push("medium".to_string());
                    }
                }
            }

            // Optional max resolution filter
            if let Some(scale) = settings.max_resolution.scale_filter() {
                tab_args.push("-vf".to_string());
                tab_args.push(scale.to_string());
            }

            tab_args.push("-c:a".to_string());
            tab_args.push("aac".to_string());
            tab_args.push("-b:a".to_string());
            tab_args.push("128k".to_string());

            ("mp4", tab_args)
        }

        TabCategory::Trim => {
            let mut tab_args = Vec::new();
            let start = settings.trim_start.trim();
            let end = settings.trim_end.trim();

            if !start.is_empty() {
                tab_args.push("-ss".to_string());
                tab_args.push(start.to_string());
            }
            if !end.is_empty() {
                tab_args.push("-to".to_string());
                tab_args.push(end.to_string());
            }

            tab_args.push("-i".to_string());
            tab_args.push(file_path.to_string_lossy().to_string());

            match settings.trim_mode {
                TrimMode::FastCopy => {
                    was_fast_copy = true;
                    tab_args.push("-c".to_string());
                    tab_args.push("copy".to_string());
                }
                TrimMode::PreciseEncode => {
                    tab_args.push("-c:v".to_string());
                    tab_args.push("libx264".to_string());
                    tab_args.push("-crf".to_string());
                    tab_args.push("20".to_string());
                    tab_args.push("-c:a".to_string());
                    tab_args.push("aac".to_string());
                }
            }

            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
            (ext, tab_args)
        }

        TabCategory::Join => {
            // Join is handled separately by generating a concat demuxer file
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
            let mut tab_args = Vec::new();
            tab_args.push("-i".to_string());
            tab_args.push(file_path.to_string_lossy().to_string());
            tab_args.push("-c".to_string());
            tab_args.push("copy".to_string());
            (ext, tab_args)
        }

        TabCategory::Tools => {
            let mut tab_args = Vec::new();

            match settings.tool_category {
                ToolSubCategory::Transform => {
                    tab_args.push("-i".to_string());
                    tab_args.push(file_path.to_string_lossy().to_string());

                    let mut vfilters = Vec::new();

                    if let Some(scale) = settings.transform_resize.scale_filter() {
                        vfilters.push(scale.to_string());
                    }

                    match settings.transform_rotate {
                        Rotation::None => {}
                        Rotation::Rotate90Cw => vfilters.push("transpose=1".to_string()),
                        Rotation::Rotate180 => vfilters.push("transpose=2,transpose=2".to_string()),
                        Rotation::Rotate270Cw => vfilters.push("transpose=2".to_string()),
                    }

                    match settings.transform_flip {
                        Flip::None => {}
                        Flip::Horizontal => vfilters.push("hflip".to_string()),
                        Flip::Vertical => vfilters.push("vflip".to_string()),
                    }

                    let speed = settings.transform_speed.factor();
                    if (speed - 1.0).abs() > 0.01 {
                        let pts_mult = 1.0 / speed;
                        vfilters.push(format!("setpts={:.3}*PTS", pts_mult));

                        let mut afilters = Vec::new();
                        afilters.push(format!("atempo={:.3}", speed));
                        tab_args.push("-af".to_string());
                        tab_args.push(afilters.join(","));
                    }

                    if !vfilters.is_empty() {
                        tab_args.push("-vf".to_string());
                        tab_args.push(vfilters.join(","));
                    }

                    tab_args.push("-c:v".to_string());
                    tab_args.push("libx264".to_string());
                    tab_args.push("-crf".to_string());
                    tab_args.push("22".to_string());
                    tab_args.push("-c:a".to_string());
                    tab_args.push("aac".to_string());

                    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
                    (ext, tab_args)
                }

                ToolSubCategory::AudioTrack => {
                    tab_args.push("-i".to_string());
                    tab_args.push(file_path.to_string_lossy().to_string());

                    if settings.audio_remove {
                        tab_args.push("-an".to_string());
                        tab_args.push("-c:v".to_string());
                        tab_args.push("copy".to_string());
                    } else if let Some(audio_file) = &settings.audio_replace_file {
                        tab_args.push("-i".to_string());
                        tab_args.push(audio_file.to_string_lossy().to_string());
                        tab_args.push("-map".to_string());
                        tab_args.push("0:v:0".to_string());
                        tab_args.push("-map".to_string());
                        tab_args.push("1:a:0".to_string());
                        tab_args.push("-c:v".to_string());
                        tab_args.push("copy".to_string());
                        tab_args.push("-c:a".to_string());
                        tab_args.push("aac".to_string());
                    } else {
                        tab_args.push("-c".to_string());
                        tab_args.push("copy".to_string());
                    }

                    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
                    (ext, tab_args)
                }

                ToolSubCategory::GifThumbnail => {
                    match settings.gif_thumb_mode {
                        GifThumbnailMode::VideoToGif => {
                            tab_args.push("-i".to_string());
                            tab_args.push(file_path.to_string_lossy().to_string());

                            let gif_filter = format!(
                                "fps={},scale={}:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
                                settings.gif_fps, settings.gif_width
                            );
                            tab_args.push("-vf".to_string());
                            tab_args.push(gif_filter);

                            ("gif", tab_args)
                        }
                        GifThumbnailMode::ExtractThumbnail => {
                            let t = settings.thumb_time.trim();
                            if !t.is_empty() {
                                tab_args.push("-ss".to_string());
                                tab_args.push(t.to_string());
                            }
                            tab_args.push("-i".to_string());
                            tab_args.push(file_path.to_string_lossy().to_string());
                            tab_args.push("-vframes".to_string());
                            tab_args.push("1".to_string());
                            ("jpg", tab_args)
                        }
                    }
                }

                ToolSubCategory::Subtitles => {
                    tab_args.push("-i".to_string());
                    tab_args.push(file_path.to_string_lossy().to_string());

                    match settings.subtitle_action {
                        SubtitleAction::ExtractSrt => {
                            tab_args.push("-map".to_string());
                            tab_args.push("0:s:0".to_string());
                            ("srt", tab_args)
                        }
                        SubtitleAction::BurnIn => {
                            if let Some(sub) = &settings.subtitle_file {
                                let sub_escaped = sub.to_string_lossy().replace('\\', "/").replace(':', "\\:");
                                tab_args.push("-vf".to_string());
                                tab_args.push(format!("subtitles='{}'", sub_escaped));
                            }
                            tab_args.push("-c:v".to_string());
                            tab_args.push("libx264".to_string());
                            tab_args.push("-c:a".to_string());
                            tab_args.push("copy".to_string());
                            ("mp4", tab_args)
                        }
                    }
                }

                ToolSubCategory::AudioFilters => {
                    tab_args.push("-i".to_string());
                    tab_args.push(file_path.to_string_lossy().to_string());

                    let mut afilters = Vec::new();

                    if settings.volume_percent != 100 {
                        let mult = settings.volume_percent as f64 / 100.0;
                        afilters.push(format!("volume={:.2}", mult));
                    }

                    if settings.audio_normalize {
                        afilters.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
                    }

                    if !afilters.is_empty() {
                        tab_args.push("-af".to_string());
                        tab_args.push(afilters.join(","));
                    }

                    match settings.audio_channels {
                        AudioChannelMode::Original => {}
                        AudioChannelMode::Mono => {
                            tab_args.push("-ac".to_string());
                            tab_args.push("1".to_string());
                        }
                        AudioChannelMode::Stereo => {
                            tab_args.push("-ac".to_string());
                            tab_args.push("2".to_string());
                        }
                    }

                    tab_args.push("-b:a".to_string());
                    tab_args.push(format!("{}k", settings.audio_bitrate_kbps));

                    if media_info.video_codec.is_some() {
                        tab_args.push("-c:v".to_string());
                        tab_args.push("copy".to_string());
                        tab_args.push("-c:a".to_string());
                        tab_args.push("aac".to_string());
                        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
                        (ext, tab_args)
                    } else {
                        tab_args.push("-c:a".to_string());
                        tab_args.push("libmp3lame".to_string());
                        ("mp3", tab_args)
                    }
                }
            }
        }
    };

    args.extend(built_args);

    let original_filename = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("output");
    let safe_output_path = crate::ffmpeg::get_safe_destination_path(dest_dir, original_filename, target_ext);

    args.push(safe_output_path.to_string_lossy().to_string());

    BuiltCommand {
        args,
        output_path: safe_output_path,
        was_fast_copy,
        is_hardware,
    }
}

/// Builds concat join command for multiple files
pub fn build_join_command(
    concat_list_file: &Path,
    output_filename: &str,
    dest_dir: &Path,
) -> BuiltCommand {
    let mut args = Vec::new();
    args.push("-hide_banner".to_string());
    args.push("-y".to_string());
    args.push("-progress".to_string());
    args.push("pipe:1".to_string());
    args.push("-nostats".to_string());
    args.push("-f".to_string());
    args.push("concat".to_string());
    args.push("-safe".to_string());
    args.push("0".to_string());
    args.push("-i".to_string());
    args.push(concat_list_file.to_string_lossy().to_string());
    args.push("-c".to_string());
    args.push("copy".to_string());

    let ext = Path::new(output_filename).extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let safe_path = crate::ffmpeg::get_safe_destination_path(dest_dir, output_filename, ext);

    args.push(safe_path.to_string_lossy().to_string());

    BuiltCommand {
        args,
        output_path: safe_path,
        was_fast_copy: true,
        is_hardware: false,
    }
}

/// Formats a human-readable shell command string for the "Show command" preview box
pub fn format_command_display(ffmpeg_exe: &str, args: &[String]) -> String {
    let mut parts = Vec::new();
    parts.push(ffmpeg_exe.to_string());

    for arg in args {
        if arg.contains(' ') || arg.contains(';') || arg.contains('[') || arg.contains(']') {
            parts.push(format!("\"{}\"", arg));
        } else {
            parts.push(arg.clone());
        }
    }

    parts.join(" ")
}
