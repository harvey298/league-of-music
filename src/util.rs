use inputbot::KeybdKey;
use serde::{Deserialize, Serialize};

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::{ptr, fs, io};
use winapi::ctypes::c_int;
use winapi::um::winuser::{MapVirtualKeyExW, MAPVK_VK_TO_VSC, ToUnicodeEx};
use anyhow::{Result, bail};

use crate::{FFMPEG_DOWNLOAD_URL, DATA_DIR};


pub fn char_to_keyboard_id(c: char) -> u32 {
    let mut vk_code = 0;
    let mut scan_code = 0;
    let mut key_state: [u8; 256] = [0; 256];

    let mut buffer = [0; 10];

    let string = OsStr::new(&format!("{}", c)).encode_wide().collect::<Vec<_>>();
    let mut result = unsafe {
        ToUnicodeEx(
            string[0] as u32,
            vk_code,
            &key_state as *const u8,
            buffer.as_mut_ptr(),
            buffer.len() as c_int,
            0,
            ptr::null_mut(),
        )
    };

    // println!("{result}");

    if result > 0 {
        vk_code = buffer[0] as u32;
        scan_code = unsafe { MapVirtualKeyExW(vk_code, MAPVK_VK_TO_VSC, ptr::null_mut()) };
    }

    // println!("{scan_code}");

    scan_code
}


pub fn minutes_to_seconds(timestamp: &str) -> u32 {

    let splitted: Vec<&str> = timestamp.split(":").collect();

    let min: u32 = splitted.first().unwrap().parse().unwrap();
    
    let secs: u32 = splitted.last().unwrap().parse().unwrap();

    (min * 60) + secs
}

#[derive(Debug, Clone)]
pub struct GameDetail {
    pub players: Vec<String>,
    pub champ: String,
}

#[derive(Debug, Clone)]
pub struct Ability {
    pub level: i64,
    pub ah: f64,
    pub id: String,
}

// "assists": 0,
// "creepScore": 0,
// "deaths": 0,
// "kills": 0,
// "wardScore": 0.0

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct PlayerScores {
    pub assists: usize,
    pub creepScore: usize,
    pub deaths: usize,
    pub kills: usize,
    pub wardScore: f64,
}

#[derive(Debug, Clone)]
pub struct PlayerData {
    pub scores: PlayerScores,
    pub name: String,
}

pub(crate) fn get_keybd_key(c: char) -> Option<KeybdKey> {
    let key = match c {
        ' ' => Some(KeybdKey::SpaceKey),
        'A' | 'a' => Some(KeybdKey::AKey),
        'B' | 'b' => Some(KeybdKey::BKey),
        'C' | 'c' => Some(KeybdKey::CKey),
        'D' | 'd' => Some(KeybdKey::DKey),
        'E' | 'e' => Some(KeybdKey::EKey),
        'F' | 'f' => Some(KeybdKey::FKey),
        'G' | 'g' => Some(KeybdKey::GKey),
        'H' | 'h' => Some(KeybdKey::HKey),
        'I' | 'i' => Some(KeybdKey::IKey),
        'J' | 'j' => Some(KeybdKey::JKey),
        'K' | 'k' => Some(KeybdKey::KKey),
        'L' | 'l' => Some(KeybdKey::LKey),
        'M' | 'm' => Some(KeybdKey::MKey),
        'N' | 'n' => Some(KeybdKey::NKey),
        'O' | 'o' => Some(KeybdKey::OKey),
        'P' | 'p' => Some(KeybdKey::PKey),
        'Q' | 'q' => Some(KeybdKey::QKey),
        'R' | 'r' => Some(KeybdKey::RKey),
        'S' | 's' => Some(KeybdKey::SKey),
        'T' | 't' => Some(KeybdKey::TKey),
        'U' | 'u' => Some(KeybdKey::UKey),
        'V' | 'v' => Some(KeybdKey::VKey),
        'W' | 'w' => Some(KeybdKey::WKey),
        'X' | 'x' => Some(KeybdKey::XKey),
        'Y' | 'y' => Some(KeybdKey::YKey),
        'Z' | 'z' => Some(KeybdKey::ZKey),
        '0' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad0Key)},
        '1' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad1Key)},
        '2' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad2Key)},
        '3' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad3Key)},
        '4' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad4Key)},
        '5' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad5Key)},
        '6' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad6Key)},
        '7' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad7Key)},
        '8' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad8Key)},
        '9' => {println!("Number Bindings are binded to the numpad!");Some(KeybdKey::Numpad9Key)},
        // '[' => Some(KeybdKey::),
        _ => None,
    };

    if key.is_none() {
        Some(KeybdKey::OtherKey(char_to_keyboard_id(c) as u64))
    } else {
        key
    }
}

pub fn is_number(c: char) -> bool {
    match c {
        '0' => true,
        '1' => true,
        '2' => true,
        '3' => true,
        '4' => true,
        '5' => true,
        '6' => true,
        '7' => true,
        '8' => true,
        '9' => true,
        _ => false
    }
}

/// Mutilated version of this example https://github.com/zip-rs/zip/blob/master/examples/extract.rs
pub async fn download_ffmpeg() -> Result<String> {
    let url = FFMPEG_DOWNLOAD_URL;

    if !Path::new(DATA_DIR).exists() { fs::create_dir(DATA_DIR)? }

    let ffmpeg_zip_path = format!("{DATA_DIR}ffmpeg.zip");

    if !Path::new(&ffmpeg_zip_path).exists() {
        let res = reqwest::get(url).await?;
        let res = res.bytes().await?;
    
        if !Path::new(DATA_DIR).exists() { fs::create_dir(DATA_DIR)?; }
    
        fs::write(&ffmpeg_zip_path, res)?;
    }

    let ffmpeg_file_zip = OpenOptions::new().read(true).open(ffmpeg_zip_path)?;

    let mut archive = zip::ZipArchive::new(ffmpeg_file_zip)?;

    let fin_path = format!("{DATA_DIR}ffmpeg.exe");

    if Path::new(&fin_path).exists() { return Ok(fin_path) }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => {
                path.to_owned()
            },
            None => continue,
        };

        let path = outpath.to_str().unwrap();
        let new_path = format!("{DATA_DIR}{path}");
        let outpath = Path::new(&new_path);

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }


        if !outpath.display().to_string().contains("bin/ffmpeg.exe") {
            continue;
        }

        if (*file.name()).ends_with('/') {
            // println!("File {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            // println!(
            //     "File {} extracted to \"{}\" ({} bytes)",
            //     i,
            //     outpath.display(),
            //     file.size()
            // );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();

            fs::copy(outpath, fin_path.clone())?;
        }

    }

    Ok(fin_path)
}