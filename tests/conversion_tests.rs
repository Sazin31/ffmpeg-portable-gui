// Test verification of codec probing, command generation, and execution
// Checklist:
// 1. TS to MP4 (copy)
// 2. MP4 to MP3
// 3. MP4 to Opus
// 4. Video Compression (CRF)
// 5. Trim/cut (fast copy and precise)
// 6. Join (concat demuxer)
// 7. Video to GIF (palettegen + paletteuse)

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    fn get_ffmpeg_bin() -> PathBuf {
        if PathBuf::from("bin/ffmpeg.exe").is_file() {
            PathBuf::from("bin/ffmpeg.exe")
        } else {
            PathBuf::from("ffmpeg")
        }
    }

    fn ensure_test_assets(ffmpeg: &PathBuf) {
        let _ = fs::create_dir_all("test_assets");
        let mp4 = PathBuf::from("test_assets/sample.mp4");
        if !mp4.exists() {
            let _ = Command::new(ffmpeg)
                .args(&[
                    "-y",
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc=size=640x360:rate=30",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=frequency=1000:sample_rate=44100",
                    "-t",
                    "2",
                    "-c:v",
                    "libx264",
                    "-c:a",
                    "aac",
                    "test_assets/sample.mp4",
                ])
                .status();
        }
        let ts = PathBuf::from("test_assets/sample.ts");
        if !ts.exists() {
            let _ = Command::new(ffmpeg)
                .args(&[
                    "-y",
                    "-i",
                    "test_assets/sample.mp4",
                    "-c",
                    "copy",
                    "test_assets/sample.ts",
                ])
                .status();
        }
    }

    #[test]
    fn test_ffmpeg_probe_and_remux_ts_to_mp4() {
        let ffmpeg_exe = get_ffmpeg_bin();
        ensure_test_assets(&ffmpeg_exe);
        let ts_file = PathBuf::from("test_assets/sample.ts");
        let out_mp4 = PathBuf::from("test_assets/output_remux.mp4");

        if out_mp4.exists() {
            let _ = fs::remove_file(&out_mp4);
        }

        // 1. Probe TS file
        let output = Command::new(&ffmpeg_exe)
            .arg("-hide_banner")
            .arg("-i")
            .arg(&ts_file)
            .output()
            .expect("probe TS file");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("h264"), "Should detect h264 in TS: {}", stderr);
        assert!(stderr.contains("aac"), "Should detect aac in TS: {}", stderr);

        // 2. Perform fast remux TS -> MP4 (-c copy)
        let status = Command::new(&ffmpeg_exe)
            .args(&[
                "-hide_banner",
                "-y",
                "-i",
                ts_file.to_str().unwrap(),
                "-c",
                "copy",
                out_mp4.to_str().unwrap(),
            ])
            .status()
            .expect("execute remux");

        assert!(status.success(), "Remux should succeed");
        assert!(out_mp4.exists(), "Output MP4 should exist");
        assert!(fs::metadata(&out_mp4).unwrap().len() > 1000, "Output MP4 should be non-empty");

        let _ = fs::remove_file(&out_mp4);
    }

    #[test]
    fn test_mp4_to_mp3_extraction() {
        let ffmpeg_exe = get_ffmpeg_bin();
        ensure_test_assets(&ffmpeg_exe);
        let mp4_file = PathBuf::from("test_assets/sample.mp4");
        let out_mp3 = PathBuf::from("test_assets/output_extracted.mp3");

        if out_mp3.exists() {
            let _ = fs::remove_file(&out_mp3);
        }

        // MP4 -> MP3 (-vn -c:a libmp3lame -q:a 2)
        let status = Command::new(&ffmpeg_exe)
            .args(&[
                "-hide_banner",
                "-y",
                "-i",
                mp4_file.to_str().unwrap(),
                "-vn",
                "-c:a",
                "libmp3lame",
                "-q:a",
                "2",
                out_mp3.to_str().unwrap(),
            ])
            .status()
            .expect("execute audio extraction");

        assert!(status.success(), "MP3 extraction should succeed");
        assert!(out_mp3.exists(), "Output MP3 should exist");
        assert!(fs::metadata(&out_mp3).unwrap().len() > 1000, "Output MP3 should be non-empty");

        let _ = fs::remove_file(&out_mp3);
    }

    #[test]
    fn test_mp4_to_opus_extraction() {
        let ffmpeg_exe = get_ffmpeg_bin();
        ensure_test_assets(&ffmpeg_exe);
        let mp4_file = PathBuf::from("test_assets/sample.mp4");
        let out_opus = PathBuf::from("test_assets/output_extracted.opus");

        if out_opus.exists() {
            let _ = fs::remove_file(&out_opus);
        }

        // MP4 -> Opus (-vn -c:a libopus -b:a 128k)
        let status = Command::new(&ffmpeg_exe)
            .args(&[
                "-hide_banner",
                "-y",
                "-i",
                mp4_file.to_str().unwrap(),
                "-vn",
                "-c:a",
                "libopus",
                "-b:a",
                "128k",
                out_opus.to_str().unwrap(),
            ])
            .status()
            .expect("execute opus conversion");

        assert!(status.success(), "Opus extraction should succeed");
        assert!(out_opus.exists(), "Output Opus should exist");
        assert!(fs::metadata(&out_opus).unwrap().len() > 500, "Output Opus should be non-empty");

        let _ = fs::remove_file(&out_opus);
    }

    #[test]
    fn test_video_compression() {
        let ffmpeg_exe = get_ffmpeg_bin();
        ensure_test_assets(&ffmpeg_exe);
        let mp4_file = PathBuf::from("test_assets/sample.mp4");
        let out_compressed = PathBuf::from("test_assets/output_compressed.mp4");

        if out_compressed.exists() {
            let _ = fs::remove_file(&out_compressed);
        }

        // Compress video with CRF 28 and preset medium
        let status = Command::new(&ffmpeg_exe)
            .args(&[
                "-hide_banner",
                "-y",
                "-i",
                mp4_file.to_str().unwrap(),
                "-c:v",
                "libx264",
                "-crf",
                "28",
                "-preset",
                "medium",
                "-c:a",
                "copy",
                out_compressed.to_str().unwrap(),
            ])
            .status()
            .expect("execute video compression");

        assert!(status.success(), "Video compression should succeed");
        assert!(out_compressed.exists(), "Compressed MP4 should exist");

        let _ = fs::remove_file(&out_compressed);
    }

    #[test]
    fn test_video_trim() {
        let ffmpeg_exe = get_ffmpeg_bin();
        ensure_test_assets(&ffmpeg_exe);
        let mp4_file = PathBuf::from("test_assets/sample.mp4");
        let out_trim = PathBuf::from("test_assets/output_trimmed.mp4");

        if out_trim.exists() {
            let _ = fs::remove_file(&out_trim);
        }

        // Fast trim with copy: -ss 00:00:00 -to 00:00:01 -c copy
        let status = Command::new(&ffmpeg_exe)
            .args(&[
                "-hide_banner",
                "-y",
                "-ss",
                "00:00:00",
                "-to",
                "00:00:01",
                "-i",
                mp4_file.to_str().unwrap(),
                "-c",
                "copy",
                out_trim.to_str().unwrap(),
            ])
            .status()
            .expect("execute trim");

        assert!(status.success(), "Trim should succeed");
        assert!(out_trim.exists(), "Trimmed MP4 should exist");

        let _ = fs::remove_file(&out_trim);
    }

    #[test]
    fn test_concat_join() {
        let ffmpeg_exe = get_ffmpeg_bin();
        ensure_test_assets(&ffmpeg_exe);
        let mp4_file = PathBuf::from("test_assets/sample.mp4");
        let list_file = PathBuf::from("test_assets/join_list.txt");
        let out_joined = PathBuf::from("test_assets/output_joined.mp4");

        let abs_path = fs::canonicalize(&mp4_file).unwrap();
        let path_str = abs_path.to_str().unwrap().replace('\\', "/");
        fs::write(&list_file, format!("file '{}'\nfile '{}'\n", path_str, path_str)).expect("write join list");

        if out_joined.exists() {
            let _ = fs::remove_file(&out_joined);
        }

        // Concat demuxer: -f concat -safe 0 -i list.txt -c copy output.mp4
        let status = Command::new(&ffmpeg_exe)
            .args(&[
                "-hide_banner",
                "-y",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                list_file.to_str().unwrap(),
                "-c",
                "copy",
                out_joined.to_str().unwrap(),
            ])
            .status()
            .expect("execute join");

        assert!(status.success(), "Concat join should succeed");
        assert!(out_joined.exists(), "Joined MP4 should exist");

        let _ = fs::remove_file(&list_file);
        let _ = fs::remove_file(&out_joined);
    }

    #[test]
    fn test_video_to_gif_high_quality() {
        let ffmpeg_exe = get_ffmpeg_bin();
        ensure_test_assets(&ffmpeg_exe);
        let mp4_file = PathBuf::from("test_assets/sample.mp4");
        let out_gif = PathBuf::from("test_assets/output.gif");

        if out_gif.exists() {
            let _ = fs::remove_file(&out_gif);
        }

        // Palettegen + paletteuse filter
        let filter = "fps=15,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse";
        let status = Command::new(&ffmpeg_exe)
            .args(&[
                "-hide_banner",
                "-y",
                "-i",
                mp4_file.to_str().unwrap(),
                "-vf",
                filter,
                out_gif.to_str().unwrap(),
            ])
            .status()
            .expect("execute gif conversion");

        assert!(status.success(), "GIF conversion should succeed");
        assert!(out_gif.exists(), "GIF should exist");
        assert!(fs::metadata(&out_gif).unwrap().len() > 1000, "GIF should be non-empty");

        let _ = fs::remove_file(&out_gif);
    }
}
