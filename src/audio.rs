use std::io::Cursor;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};

pub struct SoundEngine {
    _stream: OutputStream,

    // Store two sounds now
    shutter_data: Vec<u8>,
    activate_data: Vec<u8>,
}

impl SoundEngine {
    pub fn new() -> Self {
        // Open the default output stream using the builder API
        let stream = OutputStreamBuilder::open_default_stream().unwrap();

        // Load BOTH sounds at compile time
        // Make sure you have 'assets/activate.wav'
        let shutter_data = include_bytes!("assets/shutter.wav").to_vec();
        // Use a dummy empty vec if you don't have the file yet to prevent compile error:
        // let activate_data = vec![];
        let activate_data = include_bytes!("assets/activate.wav").to_vec();

        Self {
            _stream: stream,
            shutter_data,
            activate_data,
        }
    }

    /// Helper to play raw data
    fn play(&self, data: &[u8]) {
        // Create a Sink connected to the stream's mixer
        let sink = Sink::connect_new(&self._stream.mixer());
        let cursor = Cursor::new(data.to_vec()); // Clone the data for playback
        if let Ok(source) = Decoder::try_from(cursor) {
            sink.append(source);
            sink.detach();
        }
    }

    pub fn play_shutter(&self) {
        self.play(&self.shutter_data);
    }

    pub fn play_activation(&self) {
        self.play(&self.activate_data);
    }
}