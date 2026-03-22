use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct MusicAudioEngine {
    stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    active_path: Option<PathBuf>,
}

impl MusicAudioEngine {
    pub fn new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|err| format!("Audio output unavailable: {err}"))?;
        Ok(Self {
            stream,
            handle,
            sink: None,
            active_path: None,
        })
    }

    pub fn active_path(&self) -> Option<&Path> {
        self.active_path.as_deref()
    }

    pub fn play_file(&mut self, path: &Path, start_seconds: f32) -> Result<(), String> {
        let file = File::open(path).map_err(|err| format!("Failed to open audio file: {err}"))?;
        let reader = BufReader::new(file);
        let decoder =
            Decoder::new(reader).map_err(|err| format!("Failed to decode audio file: {err}"))?;
        let source = decoder.skip_duration(Duration::from_secs_f32(start_seconds.max(0.0)));
        let sink = Sink::try_new(&self.handle)
            .map_err(|err| format!("Failed to create audio sink: {err}"))?;
        sink.append(source);
        sink.play();
        if let Some(existing) = self.sink.take() {
            existing.stop();
        }
        self.sink = Some(sink);
        self.active_path = Some(path.to_path_buf());
        let _ = &self.stream;
        Ok(())
    }

    pub fn pause(&self) {
        if let Some(sink) = &self.sink {
            sink.pause();
        }
    }

    pub fn resume(&self) {
        if let Some(sink) = &self.sink {
            sink.play();
        }
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.active_path = None;
    }
}
