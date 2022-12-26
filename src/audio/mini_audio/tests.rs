use std::{thread::sleep, time::Duration, fs::OpenOptions, io::Write};

use anyhow::Result;

use crossbeam_channel::unbounded;
use lazy_static::__Deref;
use miniaudio::{Context, Device, Decoder, DeviceConfig, DeviceType, DecoderConfig, Format, Resampler, ResamplerConfig, ResampleAlgorithmType, FramesMut};
use serde::__private::de;

#[test]
pub fn audio_test() {
    let mut decoder = Decoder::from_file("examples/bonk-By-Tuna.mp3", None)
        .expect("failed to initialize decoder from file");

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

    let mut device = Device::new(None, &config).expect("failed to open playback device");

    // Unlike `SyncDecoder`, Decoder is not cloneable so we have to use a version of the data
    // callback that doesn't require everything that we pass into it to be cloneable. So here we
    // use a device specific data callback.
    device.set_data_callback(move |_device, output, _frames| {

        decoder.read_pcm_frames(output);
    });

    device.start().expect("failed to start device");

    println!("Device Backend: {:?}", device.context().backend());
    sleep(Duration::from_secs(1));
}

#[test]
pub fn capture_test() {

    let mut config = DeviceConfig::new(DeviceType::Capture);
    config.capture_mut();

    let mut device = Device::new(None, &config).expect("failed to open capture device");

    device.capture();

    // let (tx, rx) = unbounded();

    device.set_data_callback(move |raw_device, frame_mut, frame | {

        // let bytes = frame.as_bytes();        

        let conf = ResamplerConfig::new(
            Format::U8,
            1,
            raw_device.sample_rate(),
            raw_device.sample_rate(),
            ResampleAlgorithmType::Linear);

        let mut samp = Resampler::new(&conf).unwrap();

        let data = samp.process_pcm_frames(frame_mut, frame).unwrap();

        // tx.clone().send(frame_mut.as_bytes());

        // play(bytes, raw_device.sample_rate());

        // println!("{:?}",bytes);

        // let mut f = OpenOptions::new().create(true).append(true).open("test.mp3").unwrap();
        // f.write_all(bytes);

    });

    device.start();


    sleep(Duration::from_secs(10));
}

pub fn play(bytes: &[u8], output_sample_rate: u32) -> Result<()> {

    let decoder_conf = DecoderConfig::new(Format::Unknown, 1, output_sample_rate);

    let mut decoder = Decoder::from_memory(bytes, Some(&decoder_conf))
        .expect("failed to initialize decoder from file");

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

    let mut device = Device::new(None, &config).expect("failed to open playback device");

    // Unlike `SyncDecoder`, Decoder is not cloneable so we have to use a version of the data
    // callback that doesn't require everything that we pass into it to be cloneable. So here we
    // use a device specific data callback.
    device.set_data_callback(move |_device, output, _frames| {
        decoder.read_pcm_frames(output);
    });

    device.start().expect("failed to start device");

    Ok(())
}