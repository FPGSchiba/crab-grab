use std::io::Cursor;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

pub struct SoundEngine {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,

    // Store two sounds now
    shutter_data: Vec<u8>,
    activate_data: Vec<u8>,
}

impl SoundEngine {
    pub fn new() -> Self {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();

        // Load BOTH sounds at compile time
        // Make sure you have 'assets/activate.wav'
        let shutter_data = include_bytes!("assets/shutter.wav").to_vec();
        // Use a dummy empty vec if you don't have the file yet to prevent compile error:
        // let activate_data = vec![];
        let activate_data = include_bytes!("assets/activate.wav").to_vec();

        Self {
            _stream,
            stream_handle,
            shutter_data,
            activate_data,
        }
    }

    /// Helper to play raw data
    fn play(&self, data: &[u8]) {
        if let Ok(sink) = Sink::try_new(&self.stream_handle) {
            let cursor = Cursor::new(data.to_vec()); // Clone the data for playback
            if let Ok(source) = Decoder::new(cursor) {
                sink.append(source);
                sink.detach();
            }
        }
    }

    pub fn play_shutter(&self) {
        self.play(&self.shutter_data);
    }

    pub fn play_activation(&self) {
        self.play(&self.activate_data);
    }
}