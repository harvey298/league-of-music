use std::{fs, process::Command, path::Path, thread::{self, spawn}, time::Duration, sync::Arc};

use crossbeam_channel::unbounded;

use anyhow::Result;

use reqwest::Certificate;
use sounds::play;
use toml::Value;
use serde_json::Value as SValue;

const MUSIC_FILE_FORMAT: &str = ".mp3";
const ROOT_CERT: &str = "https://static.developer.riotgames.com/docs/lol/riotgames.pem";

extern crate rodio;

use inputbot::{KeySequence, KeybdKey::*, MouseButton::*};
use std::{thread::sleep};

mod sounds;
mod util;

#[tokio::main]
async fn main() -> Result<()> {

    // let data: Value = toml::from_str(&fs::read_to_string("music.toml").unwrap()).unwrap();

    println!("Analyzing Songs!");
    let music_file: Value = toml::from_str(&fs::read_to_string("music.toml").unwrap()).unwrap();

    for file in music_file["sounds"].as_table() {
        for key in file.keys() {
            let paths = file[key]["path"].as_array().unwrap();
            for path in paths {
                let path = path.as_str().unwrap();

                if !path.contains(MUSIC_FILE_FORMAT) {

                    let current_format = path.split(".").last().unwrap();
                    let new_path = path.replace(&format!(".{}",current_format), MUSIC_FILE_FORMAT);
    
                    if !Path::new(&new_path).exists() {
                        Command::new("ffmpeg").args(["-i", path, &new_path]).status().unwrap();
                    }
                }
            }

        }
    }
    println!("Songs ready!");

    let (tx_music_stopper, rx_music_stopper) = unbounded();

    #[cfg(debug_assertions)]
    play("debug", false, rx_music_stopper.clone());


    let username = music_file["game"]["username"].to_string().to_lowercase();

    let pem = reqwest::get(ROOT_CERT).await.unwrap().bytes().await.unwrap();

    let client = reqwest::Client::builder().add_root_certificate(Certificate::from_pem(&pem)?).build().unwrap();

    // Bind the number 1 key your keyboard to a function that types 
    // "Hello, world!" when pressed.
    // Numrow1Key.bind(|| KeySequence("Hello, world!").send());

    // Bind your caps lock key to a function that starts an autoclicker.
    let (tx_player, rx_player) = unbounded();

    let tx_player1 = tx_player.clone();
    EKey.bind(move || {
        tx_player1.send("e").unwrap();
    });

    let tx_player1 = tx_player.clone();
    QKey.bind(move || {
        tx_player1.clone().send("q").unwrap();
    });

    let tx_player1 = tx_player.clone();
    RKey.bind(move || {
        tx_player1.clone().send("r").unwrap();
    });

    let tx_player1 = tx_player.clone();
    WKey.bind(move || {
        tx_player1.clone().send("w").unwrap();
    });

    let tx_music_stopper2 = tx_music_stopper.clone();
    ZKey.bind(move || {
        tx_music_stopper2.send(()).unwrap();
    });

    // Call this to start listening for bound inputs.
    println!("Starting event listening");
    spawn(move || {
        inputbot::handle_input_events();
    });

    let mut champ = String::new();
    let mut loaded = false;
    let mut alive = false;

    let pm2 = Arc::new(pem);

    println!("Waiting for event!");
    loop {

        if !loaded {
            champ = String::new();
            match client.get("https://127.0.0.1:2999/liveclientdata/activeplayername").send().await {
                Ok(r) => {
                    // println!("Req made");
                    if r.status() == 200 {
                        // println!("200");
                        let riot_username = r.text().await.unwrap();
                        // println!("{} | {}",riot_username, username);
                        if riot_username.to_lowercase() == username {
                            loaded = true;
                            println!("Game has loaded!");
                        }
                        
                    } else {
                        thread::sleep(Duration::from_secs(5));
                    }
                },
                Err(_) => {
                    loaded = false;
                    thread::sleep(Duration::from_secs(15));
                },
            }

            if !rx_player.is_empty() {
                match rx_player.try_recv() {
                    Ok(_) => {},
                    Err(_) => {},
                }
            }

        } else {

            // https://127.0.0.1:2999/liveclientdata/playerlist
            if champ.clone() == String::new() {
                match client.get("https://127.0.0.1:2999/liveclientdata/playerlist").send().await {
                    Ok(r) => {
                        if r.status() == 200 {
                            println!("Game is loaded!");

                            println!("Finding active champion!");

                            let data: SValue = serde_json::from_str(&r.text().await?)?;
                            for player in data.as_array().unwrap() {
                                if player["summonerName"].to_string().to_lowercase() == username {
                                    champ = player["championName"].to_string().to_lowercase();
                                    println!("Found Champ! ({})", champ);
                                }
                            }
        
                        } else{
                            loaded = false;
                            println!("Game Not Loaded");
                        }
                    },
                    Err(_) => {
                        loaded = false;
                        println!("Game Not Loaded");
                    },
                }
            } else {
                // All checks - Start playing music

                let pm3 = pm2.clone();

                spawn(move || {

                    let client = reqwest::blocking::Client::builder().add_root_certificate(Certificate::from_pem(&pm3.clone()).unwrap()).build().unwrap();

                    match client.get("https://127.0.0.1:2999/liveclientdata/playerlist").send() {
                        Ok(_) => {
                            
                        },
                        Err(_) => todo!(),
                    }

                });

                champ = champ.to_lowercase().replace("\"", "");
                if music_file["sounds"].as_table().unwrap().contains_key(&champ) && !rx_player.is_empty() {

                    match rx_player.try_recv() {
                        Ok(o) => {
                            
                            #[cfg(debug_assertions)]
                            println!("Playing {} more times!",rx_player.len());

                            // println!("Key Input Detected!");

                            let abilites: Vec<String> = music_file["sounds"][champ.clone()]["ability"].as_array().unwrap().into_iter().map(| i | { i.to_string() }).collect();

                            let mut played = false;
                            for a in abilites {
                                let b = a.replace("\"","");
                                if b == o {

                                    match client.get("https://127.0.0.1:2999/liveclientdata/activeplayerabilities").send().await {
                                        Ok(r) => {

                                            // Alive Check
                                            match client.get("https://127.0.0.1:2999/liveclientdata/playerlist").send().await {
                                                Ok(r2) => {
                                                    if r.status() == 200 {
                                                        let data: SValue = serde_json::from_str(&r2.text().await?)?;
                                                        for player in data.as_array().unwrap() {
                                                            if player["summonerName"].to_string().to_lowercase() == username {

                                                                if champ == player["championName"].to_string().to_lowercase() {
                                                                    println!("New Game Detected!");
                                                                    alive = false;
                                                                    loaded = false;
                                                                }

                                                                alive = !player["isDead"].as_bool().unwrap();
                                                                #[cfg(debug_assertions)]
                                                                println!("Alive: {}", alive);
                                                            }
                                                        }
                                    
                                                    }
                                                },
                                                Err(_) => {
                                                    loaded = false;
                                                    println!("Game Not Loaded");
                                                },
                                            }
                                            
                                            let data: SValue = serde_json::from_str(&r.text().await?)?;

                                            let known_ability = data[o.to_uppercase()]["abilityLevel"].as_u64().unwrap();

                                            if rx_player.len() > 2 {
                                                let mut empty = false;
                                                println!("Ignoring Key input");
                                                while !empty {
                                                    match rx_player.try_recv() {
                                                        Ok(_) => {  },
                                                        Err(_) => { empty = true; },
                                                    }
                                                }
                
                                            } else

                                            if known_ability != 0 && alive && !LControlKey.is_pressed() {
                                                println!("Playing music for {}, Condition: {}", champ, o);
                                                let delay = music_file["sounds"][champ.clone()]["time_delay"].as_integer().unwrap();
                                                let cooldown = music_file["sounds"][champ.clone()]["cooldown"].as_integer().unwrap();
                                                let ignorance = music_file["sounds"][champ.clone()]["ignore"].as_integer().unwrap();

                                                if delay != 0 {
                                                    sleep(Duration::from_secs(delay as u64));
                                                }

                                                played = true;
                                                play(&champ, true, rx_music_stopper.clone());

                                                for _ in 0..ignorance {
                                                    println!("Sending Ignore packets!");
                                                    tx_music_stopper.clone().send(()).unwrap();
                                                }

                                                if cooldown != 0 {
                                                    sleep(Duration::from_secs(cooldown as u64));
                                                }
                                            }

                                        },
                                        Err(_) => {
                                            loaded = false;
                                            println!("Game Not Loaded");
                                        },
                                    }


                                }
                            }

                            if !played {
                                // loaded = false;
                            }

                        },
                        Err(_) => {},
                    }
                    

                } else {

                    if !rx_player.is_empty() {
                        loaded = false;
                        println!("Cannot understand if game is active or not!");
                        sleep(Duration::from_secs(60));
                    }

                }

            }

        }

    }

    Ok(())
}