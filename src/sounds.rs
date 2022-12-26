
use std::fs::{File, self};
use std::io::BufReader;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use rodio::Device;
use rodio::{Sink, cpal, cpal::traits::HostTrait };
use rodio::{Decoder, OutputStream, source::Source};
use toml::Value;
use anyhow::Result;

use crossbeam_channel::unbounded;
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;

use crate::{MUSIC_FILE_FORMAT, CONFIG_FILE, convert_dot_path_to_vec};
use crate::util::minutes_to_seconds;

pub fn play(hold: bool, killer: Receiver<()>, config_path: &str) {

    let host = cpal::default_host();

    println!("Playing: {config_path}");

    // let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let music_file: Value = toml::from_str(&fs::read_to_string(CONFIG_FILE).unwrap()).unwrap();

    let mut path = convert_dot_path_to_vec(&config_path);
    let mut buffer = &music_file[path.first().unwrap()];
    path.remove(0);

    for i in path { buffer = &buffer[i]; }

    let paths = buffer["path"].as_array().unwrap();

    let mut handles = Vec::new();

    for (i, path) in paths.into_iter().enumerate() {

        let path = path.as_str().unwrap();

        let current_format = path.split(".").last().unwrap();
        let binding = path.replace(&format!(".{}",current_format), MUSIC_FILE_FORMAT);
        let new_path = binding.to_string();
    
        let fin_path = if !path.contains(MUSIC_FILE_FORMAT) {
            new_path    
        } else { path.to_string() };
    
        let start = minutes_to_seconds( buffer["start"].as_str().unwrap() );
        let play_for = buffer["play_for"].as_integer().unwrap() as u32;
        let volume = buffer["volume"].as_float().unwrap();
        // let reverb = data["reverb"].as_bool().unwrap();
        let reverb = false;

        for device in host.output_devices().unwrap() {

            let bdevice = Arc::new(device);

            let start = if i != 0 {
                0
            } else { start };

            let volume = if i != 0 {
                -0.10
            } else { volume };

            handles.push(play_with_device(fin_path.clone().to_string(), start as u64, play_for as i32, bdevice.clone(), killer.clone(), volume, reverb).unwrap());

        }

    }

    for handle in handles {
        if !handle.is_finished() && hold { handle.join().unwrap(); }
    }

}

fn play_with_device(path: String, start: u64, play_for: i32 , device: Arc<Device>, killer: Receiver<()>, volume: f64, _reverb: bool ) -> Result<JoinHandle<()>> {  
    
    let p2 = Arc::new(path.clone());

    let k2 = killer.clone();
    let handle = thread::spawn(move || {

        // Get a output stream handle to the default physical sound device
        let (_stream, stream_handle) = OutputStream::try_from_device(&device).unwrap();

        // println!("Playing: {}",path);

        // Load a sound from a file, using a path relative to Cargo.toml
        let file = BufReader::new(File::open(p2.clone().to_string()).unwrap());

        // let reverb_duration = if reverb { Duration::from_secs( play_for as u64 )  } else { Duration::from_secs( 0 ) };

        // Decode that sound file into a source
        let source = Decoder::new(file).unwrap()
            .skip_duration(Duration::from_secs( start as u64 ))
            .amplify(volume as f32)
            .buffered()
            // .reverb(reverb_duration, 5.0)
            
            // .take_duration(Duration::from_secs( play_for.into() ))
            ;

        let sink = Sink::try_new(&stream_handle).unwrap();

        sink.append(source);
        
        for _ in 0..play_for {

            match k2.try_recv() {
                Ok(_) => {
                    sink.stop();
                },
                Err(_) => {
                    thread::sleep(Duration::from_secs(1));
                },
            }


        }
        // thread::sleep(Duration::from_secs(play_for.into()));

        sink.stop();
    });

    Ok(handle)
}
