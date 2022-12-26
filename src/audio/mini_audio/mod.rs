
use miniaudio::{Context, Device, Decoder, DeviceConfig, DeviceType, DecoderConfig, Format, Resampler, ResamplerConfig, ResampleAlgorithmType, FramesMut, SyncDecoder, ContextConfig};

use std::{thread::sleep, time::Duration, fs::OpenOptions, io::Write};

use anyhow::{Result, bail};


mod tests;

#[derive(Clone)]
pub struct Sound {
    name: String,
    decoder: SyncDecoder,
}

impl Sound {
    pub fn new(path: &str) -> Result<Self> {

        let mut decoder = SyncDecoder::from_file(path, None).unwrap();

        Ok(Self { name: path.to_owned(), decoder: decoder })
    }
}

#[derive(Clone)]
pub struct Player {
    sounds: Option<Vec<Sound>>
}

impl Player {

    pub fn new() -> Self {

        Self { sounds: None  }
    }

    pub fn play() {

    }
    
    fn _internal_player(decoder: &mut SyncDecoder) {
        // let mut decoder = Decoder::from_file("examples/bonk-By-Tuna.mp3", None)
        // .expect("failed to initialize decoder from file");
    
        let mut config = DeviceConfig::new(DeviceType::Playback);
        config.playback_mut().set_format(decoder.output_format());
        config
            .playback_mut()
            .set_channels(decoder.output_channels());
        config.set_sample_rate(decoder.output_sample_rate());
    
        // config.set_data_callback(move |_device, _output, _frames| {
        //     println!("ignored");
        // });
    
        // This stop callback can go on the config because it is cloneable.
        config.set_stop_callback(|_device| {
            println!("Device Stopped.");
        });


        let context_context = ContextConfig::default();
        let context = Context::new(&[], Some(&context_context)).unwrap();

        for device in context.playback_devices() {

            // devic

        }
    
        let mut device = Device::new(None, &config).expect("failed to open playback device");
    
        // Unlike `SyncDecoder`, Decoder is not cloneable so we have to use a version of the data
        // callback that doesn't require everything that we pass into it to be cloneable. So here we
        // use a device specific data callback.
        let decoder2 = decoder.clone();
        device.set_data_callback(move |_device, output, _frames| {
            decoder2.clone().read_pcm_frames(output);
        });
    
        device.start().expect("failed to start device");
    
    }

}
