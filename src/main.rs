use anyhow::{Context, Result};
use log::{error, info, warn};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;

#[derive(Debug, Clone)]
struct ServiceConfig {
    watch_folder: PathBuf,
    video_extensions: Vec<String>,
    ffmpeg_path: String,
    segment_duration: u32,
    ffmpeg_preset: String,
    ffmpeg_crf: u32,
    audio_bitrate: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            watch_folder: PathBuf::from("/var/watch/videos"),
            video_extensions: vec![
                "mp4".to_string(),
                "avi".to_string(),
                "mkv".to_string(),
                "mov".to_string(),
                "wmv".to_string(),
                "flv".to_string(),
            ],
            ffmpeg_path: "ffmpeg".to_string(),
            segment_duration: 4,
            ffmpeg_preset: "medium".to_string(),
            ffmpeg_crf: 23,
            audio_bitrate: "128k".to_string(),
        }
    }
}

fn load_config_from_env() -> ServiceConfig {
    let watch_folder = std::env::var("WATCH_FOLDER")
        .unwrap_or_else(|_| "/var/watch/videos".to_string());
    
    let video_extensions = std::env::var("VIDEO_EXTENSIONS")
        .unwrap_or_else(|_| "mp4,avi,mkv,mov,wmv,flv".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    
    let ffmpeg_path = std::env::var("FFMPEG_PATH")
        .unwrap_or_else(|_| "ffmpeg".to_string());
    
    let segment_duration = std::env::var("SEGMENT_DURATION")
        .unwrap_or_else(|_| "4".to_string())
        .parse()
        .unwrap_or(4);
    
    let ffmpeg_preset = std::env::var("FFMPEG_PRESET")
        .unwrap_or_else(|_| "medium".to_string());
    
    let ffmpeg_crf = std::env::var("FFMPEG_CRF")
        .unwrap_or_else(|_| "23".to_string())
        .parse()
        .unwrap_or(23);
    
    let audio_bitrate = std::env::var("AUDIO_BITRATE")
        .unwrap_or_else(|_| "128k".to_string());
    
    ServiceConfig {
        watch_folder: PathBuf::from(watch_folder),
        video_extensions,
        ffmpeg_path,
        segment_duration,
        ffmpeg_preset,
        ffmpeg_crf,
        audio_bitrate,
    }
}

fn is_video_file(path: &Path, extensions: &[String]) -> bool {
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            return extensions.iter().any(|e| e.eq_ignore_ascii_case(ext_str));
        }
    }
    false
}

fn convert_to_dash(video_path: &Path, config: &ServiceConfig) -> Result<()> {
    let file_stem = video_path
        .file_stem()
        .context("Invalid file name")?
        .to_str()
        .context("Invalid UTF-8 in file name")?;
    
    let parent_dir = video_path.parent().context("No parent directory")?;
    let output_dir = parent_dir.join(file_stem);
    
    info!("Converting {} to DASH format", video_path.display());
    info!("Output directory: {}", output_dir.display());
    
    // Create output directory
    std::fs::create_dir_all(&output_dir)
        .context(format!("Failed to create output directory: {}", output_dir.display()))?;
    
    let manifest_path = output_dir.join("manifest.mpd");
    let init_seg = output_dir.join("init-stream$RepresentationID$.m4s");
    let media_seg = output_dir.join("chunk-stream$RepresentationID$-$Number%05d$.m4s");
    
    info!("Creating DASH segments with FFmpeg...");
    
    // Use FFmpeg's built-in DASH segmenter
    let ffmpeg_output = Command::new(&config.ffmpeg_path)
        .args([
            "-i",
            video_path.to_str().unwrap(),
            "-c:v",
            "libx264",
            "-preset",
            &config.ffmpeg_preset,
            "-crf",
            &config.ffmpeg_crf.to_string(),
            "-c:a",
            "aac",
            "-b:a",
            &config.audio_bitrate,
            "-f",
            "dash",
            "-seg_duration",
            &config.segment_duration.to_string(),
            "-use_template",
            "1",
            "-use_timeline",
            "1",
            "-init_seg_name",
            init_seg.file_name().unwrap().to_str().unwrap(),
            "-media_seg_name",
            media_seg.file_name().unwrap().to_str().unwrap(),
            manifest_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to execute FFmpeg")?;
    
    if !ffmpeg_output.status.success() {
        let stderr = String::from_utf8_lossy(&ffmpeg_output.stderr);
        anyhow::bail!("FFmpeg DASH conversion failed: {}", stderr);
    }
    
    info!("DASH segmentation completed successfully");
    let stdout = String::from_utf8_lossy(&ffmpeg_output.stdout);
    if !stdout.is_empty() {
        info!("FFmpeg output: {}", stdout);
    }
    
    info!("Successfully converted {} to DASH format", video_path.display());
    info!("Manifest location: {}", manifest_path.display());
    
    Ok(())
}

fn process_video_file(path: PathBuf, config: &ServiceConfig) {
    info!("New video file detected: {}", path.display());
    
    // Wait a bit to ensure file is completely written
    std::thread::sleep(Duration::from_secs(2));
    
    match convert_to_dash(&path, config) {
        Ok(_) => info!("Successfully processed {}", path.display()),
        Err(e) => error!("Error processing {}: {}", path.display(), e),
    }
}

fn watch_folder(config: ServiceConfig) -> Result<()> {
    let watch_path = &config.watch_folder;
    
    if !watch_path.exists() {
        warn!("Watch folder doesn't exist, creating: {}", watch_path.display());
        std::fs::create_dir_all(watch_path)
            .context("Failed to create watch folder")?;
    }
    
    info!("Starting to watch folder: {}", watch_path.display());
    info!("Watching for extensions: {:?}", config.video_extensions);
    info!("FFmpeg preset: {}, CRF: {}", config.ffmpeg_preset, config.ffmpeg_crf);
    info!("Audio bitrate: {}", config.audio_bitrate);
    info!("Segment duration: {}s", config.segment_duration);
    
    let (tx, rx) = channel();
    
    let mut watcher: RecommendedWatcher = Watcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default(),
    )?;
    
    watcher.watch(watch_path, RecursiveMode::NonRecursive)?;
    
    info!("Folder watcher started successfully");
    
    for event in rx {
        if let EventKind::Create(_) | EventKind::Modify(_) = event.kind {
            for path in event.paths {
                if is_video_file(&path, &config.video_extensions) {
                    let config_clone = config.clone();
                    
                    // Process in a separate thread to avoid blocking the watcher
                    std::thread::spawn(move || {
                        process_video_file(path, &config_clone);
                    });
                }
            }
        }
    }
    
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();
    
    info!("DASH Transcoding Service starting...");
    info!("Loading configuration from environment variables");
    
    let config = load_config_from_env();
    
    info!("Configuration loaded successfully");
    
    watch_folder(config)?;
    
    Ok(())
}
