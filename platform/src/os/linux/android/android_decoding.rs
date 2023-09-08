use {
    crate::{
        cx::Cx,
        decoding_api::{CxDecodingApi, VideoDecodingInputFn},
        event::Event,
        makepad_live_id::LiveId,
        thread::Signal,
    },
    std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    },
};

#[derive(Default)]
pub struct CxAndroidDecoding {
    pub video_decoding_input_cb: HashMap<LiveId, Arc<Mutex<Option<VideoDecodingInputFn>>>>,
}

impl Cx {}

impl CxDecodingApi for Cx {
    fn video_decoding_input_box(&mut self, video_id: LiveId, f: VideoDecodingInputFn) {
        let callback = Arc::new(Mutex::new(Some(f)));
        self.os
            .decoding
            .video_decoding_input_cb
            .insert(video_id, callback);
    }
}
