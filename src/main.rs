use std::{fs::{self, OpenOptions}, process::Command, path::Path, thread::spawn, time::{Duration, Instant}, sync::Arc, io::Write};

use crossbeam_channel::{unbounded, Receiver, TryRecvError};

use anyhow::{Result, bail};

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate toml;

use reqwest::{Certificate, Client};
use sounds::play;
use tokio::sync::RwLock;
use toml::Value;
use serde_json::Value as SValue;

const MUSIC_FILE_FORMAT: &str = ".mp3";
const ROOT_CERT: &str = "https://static.developer.riotgames.com/docs/lol/riotgames.pem";
const FFMPEG_DOWNLOAD_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
const CONFIG_FILE: &str = "./config.toml";
const DATA_DIR: &str = "./data/";

const ABILITES: [&str; 6] = ["q","w","e","r","death","kill"];

const FULL_ALPHABET: [&str; 26] = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z"];
const KEYS: [&str; 22] = ["a", "b", "c", "d", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "s", "t", "u", "v", "x", "y", "z"];

const LOAD_STEPS: usize = 3;

use std::env;
lazy_static! {

    static ref ARGS: Vec<String> = {
        env::args().map(|i| { i.to_lowercase() }).collect()
    };

    static ref VERBOSE: bool = {
        ARGS.contains(&"--verbose".to_string()) || ARGS.contains(&"-v".to_string())
    };

    static ref STATE: Arc<RwLock<InternalState>> = {
        Arc::new(RwLock::new(InternalState::InactiveGame))
    };
}

extern crate rodio;

use inputbot::{KeybdKey::{*, self}};
use util::{GameDetail, Ability};
use std::{thread::sleep};

use crate::util::{PlayerScores, PlayerData, char_to_keyboard_id, download_ffmpeg};

mod sounds;
mod audio;
mod util;

#[tokio::main]
async fn main() -> Result<()> {

    println!("[(Loading 0/{LOAD_STEPS}) Game - Program Security] Fetching Game Client Certificate");
    let pem = reqwest::get(ROOT_CERT).await.unwrap().bytes().await.unwrap();
    let pem = Certificate::from_pem(&pem)?;

    let client = Arc::new(reqwest::Client::builder().add_root_certificate(pem).build()?);

    println!("[(Loading 0/{LOAD_STEPS}) Game - Program Security] OK");

    println!("[(Loading 1/{LOAD_STEPS}) Sound Validation] Validating & Cleaning sounds");

    let cmd = Command::new("ffmpeg").args(["-loglevel","quiet"]).status();

    let ffmpeg_path = match cmd {
        Ok(_) => {
            Box::new("ffmpeg".to_string())
        },
        Err(_) => {
            println!("[(Loading 1/{LOAD_STEPS}) Sound Validation] Cannot find FFMPEG!");

            let path = download_ffmpeg().await?;

            Box::new(path)
        }
    };


    if !Path::new(CONFIG_FILE).exists() {
        let data = toml::toml!(
            tip = "Check example_config.toml for more details!"

            [sounds]
            tip = "
            The sounds key is for adding new sounds for events in league of legends!
            "

            [keys]
            tip = 
            "
            The keys key is for creating bindings to a sound (like a soundboard)!
            To note adding a new key won't add a new key until the program has been restarted!
            but once a binding has been created the sound can be changed at any point!
            "
        );

        let mut file = OpenOptions::new().create_new(true).write(true).open(CONFIG_FILE)?;
        file.write_all( toml::to_string_pretty(&data)?.as_bytes() )?;
    }

    let music_file: SValue = toml::from_str(&fs::read_to_string(CONFIG_FILE).unwrap()).unwrap();

    if music_file["sounds"]["test"].is_object() {
        for path in music_file["sounds"]["test"]["path"].as_array().unwrap() {
            // let path = path;
            let path = path.as_str().unwrap();
    
            if !path.contains(MUSIC_FILE_FORMAT) {
    
                let current_format = path.split(".").last().unwrap();
                let new_path = path.replace(&format!(".{}",current_format), MUSIC_FILE_FORMAT);
    
                if !Path::new(&new_path).exists() {
                    Command::new(&*ffmpeg_path.clone()).args(["-loglevel","quiet","-nostats","-i", path, &new_path]).status().unwrap();
                }
            }
        }
    }


    for (o, champ) in music_file["sounds"].as_object().unwrap().into_iter() {
        if o == "tip" { continue; }

        if champ["enable"].as_bool().unwrap() {
            
            for ability in ABILITES {
                if champ[ability].is_object() {
                    for path in champ[ability]["path"].as_array().unwrap() {
                        let path = path.as_str().unwrap();

                        if !path.contains(MUSIC_FILE_FORMAT) {
        
                            let current_format = path.split(".").last().unwrap();
                            let new_path = path.replace(&format!(".{}",current_format), MUSIC_FILE_FORMAT);
            
                            if !Path::new(&new_path).exists() {
                                Command::new(&*ffmpeg_path.clone()).args(["-loglevel","quiet","-nostats","-i", path, &new_path]).status().unwrap();
                            }
                        }
                    }
                }
            }

        }

    }

    let _added_keys = {
        let mut tmp: Vec<String> = KEYS.into_iter().map(|i| { i.to_string() }).collect();
        let mut new_keys = Vec::new();
        for i in 0..10 {
            // println!("{i}");
            new_keys.push(format!("{i}"));
        }
        tmp.append(&mut new_keys);

        tmp
    };

    println!("[(Loading 1/{LOAD_STEPS}) Sound Validation] OK");

    println!("[(Loading 2/{LOAD_STEPS}) Game Knowledge] Loading Data Dragon");
    let dd_versions: Value = serde_json::from_str(&reqwest::get("https://ddragon.leagueoflegends.com/api/versions.json").await.unwrap().text().await.unwrap())?;
    let latest_version = dd_versions.as_array().unwrap().first().unwrap().as_str().unwrap();
    let dd_url = format!("http://ddragon.leagueoflegends.com/cdn/{latest_version}/data/en_GB/");

    println!("[(Loading 2/{LOAD_STEPS}) Game Knowledge] OK");

    println!("[(Loading 3/{LOAD_STEPS}) Soundboard] Creating Key Bindings");

    let (tx_key, rx_key) = unbounded();

    let tx_player1 = tx_key.clone();
    EKey.bind(move || { tx_player1.send("e").unwrap(); });

    let tx_player1 = tx_key.clone();
    QKey.bind(move || { tx_player1.clone().send("q").unwrap(); });

    let tx_player1 = tx_key.clone();
    RKey.bind(move || { tx_player1.clone().send("r").unwrap(); });

    let tx_player1 = tx_key.clone();
    WKey.bind(move || { tx_player1.clone().send("w").unwrap(); });

    let keys = music_file["keys"].as_object().unwrap();

    // println!("{:?}",keys);

    for (letter, k) in keys {
        if letter == "tip" { continue; }

        if music_file["keys"][&letter]["enable"].as_bool().unwrap() {
            // println!("{letter}");

            let paths = music_file["keys"][&letter]["path"].as_array().unwrap();

            for path in paths {
                // println!("{letter}-1");
                let path = path.as_str().unwrap();

                // if !path.contains(MUSIC_FILE_FORMAT) {

                    let current_format = path.split(".").last().unwrap();
                    let new_path = path.replace(&format!(".{}",current_format), MUSIC_FILE_FORMAT);
                    // println!("Binding created! {letter}");
    
                    if !Path::new(&new_path).exists() {
                        Command::new(&*ffmpeg_path.clone()).args(["-loglevel","quiet","-nostats","-i", path, &new_path]).status().unwrap();
                    }

                    if *VERBOSE {
                        println!("[(Loading 3/{LOAD_STEPS}) Soundboard] Creating binding for key {letter}");
                    }

                    // println!("[(Loading 3/{LOAD_STEPS}) Soundboard] Creating binding for key {letter}");
    
                    let key2 = letter.to_uppercase();
                    let mut chars = key2.chars();

                    // println!("Creating binding for: {key2}");B
    
                    let path = format!("keys.{letter}");
                    let c = chars.next().unwrap();

                    let (tx_key, rx_key) = unbounded();
    
                    KeybdKey::from((c as u8) as u64).bind( move || {    

                        if LShiftKey.is_pressed() {
                            play(false, rx_key.clone(), &path);
                        }

                        if LAltKey.is_pressed() {
                            // println!("Sedning death");
                            tx_key.send(()).unwrap()
                        }
        
                    });
                // }
            }


        }
    }

    spawn(move || {
        inputbot::handle_input_events();
    });

    println!("[(Loading 3/{LOAD_STEPS}) Soundboard] OK");

    println!("I am ready!");

    loop {
        
        match main_event_loop(client.clone(), &dd_url, rx_key.clone()).await {
            Ok(_) => { break },
            Err(_) => {
                sleep(Duration::from_secs(60));
            },
        };

    }

    Ok(())
}


async fn main_event_loop(client: Arc<Client>, dd_url: &str, rx_key: Receiver<&str>) -> Result<()> {

    let game = game_check(client.clone()).await?;

    let champ = game.clone().champ;
    let dd_url = format!("{dd_url}champion/{champ}.json");

    let dd_data: SValue = serde_json::from_str(&client.get(dd_url).send().await?.text().await?)?;

    let config: SValue = toml::from_str(&fs::read_to_string(CONFIG_FILE).unwrap()).unwrap();

    let champ2 = champ.to_lowercase();
    let sounds = format!("sounds.{champ2}");

    let enable = format!("sounds.{champ2}.enable");

    let mut path = convert_dot_path_to_vec(&enable);
    let mut enable = &config[path.first().unwrap()];
    path.remove(0);
    for i in path { enable = &enable[i]; }
    let enabled = enable.as_bool().unwrap_or(false);

    if !enabled {
        println!("I am not going to play sounds for this champ!");
    }

    let mut cooldown = 0.0;
    let mut last_game_time = get_game_time(client.clone()).await?;
    let mut last_internel_time = Instant::now();

    let mut player_data = Vec::new();
    let mut first_loop = true;

    let mut evennt_sound_playing = false;

    loop {
        match rx_key.try_recv() {
            Ok(key) => {
                // Do ability related stuff here
                // This could cause some issues?
                // It did
                if cooldown > 0.0 { continue; }
                if !enabled { continue; }

                let txt_path = format!("{sounds}.{key}");
                let mut path = convert_dot_path_to_vec(&txt_path);
                let mut buffer = &config[path.first().unwrap()];
                path.remove(0);

                for i in path { buffer = &buffer[i]; }

                if buffer.is_object() {
                    let (tx_key, rx_key) = unbounded();
                    play(false, rx_key, &txt_path);
                    cooldown = calculate_cooldown(client.clone(),&champ, &key.to_uppercase(), &dd_data).await.unwrap();

                }            

            },

            Err(TryRecvError::Disconnected) => {
                println!("I am unable to understand your key inputs!\nI need restarting!");
                break
            }

            Err(TryRecvError::Empty) => {
                // Do game related stuff here

                let enable = true;

                if last_internel_time.elapsed() > Duration::from_secs_f32(1.0) && enable {

                    // println!("{:?}",game.clone().players);
                    let mut filling = false;
                    let mut needs_emptying = false;

                    for (_id, player) in game.clone().players.into_iter().enumerate() {
                        let url = format!("https://127.0.0.1:2999/liveclientdata/playerscores?summonerName={player}");
                        let player_score: PlayerScores = serde_json::from_str(&client.get(url).send().await?.text().await?)?;

                        let player_detail = PlayerData { scores: player_score, name: player.to_string() };

                        // if first_loop { player_data.push(player_detail); println!("Resetting 0"); first_loop = false; continue; }

                        // If the list is empty, fill it
                        if player_data.clone().is_empty() || filling {

                            player_data.push(player_detail);
                            filling = true;
                            continue;
                        }

                        // Find the player
                        for old_data in &player_data {
                            // println!("Goind through players");

                            // println!("{} | {}",old_data.name, player_detail.name);

                            if old_data.name != player_detail.name { continue; }

                            let old_scores = &old_data.scores;

                            if old_scores == &player_score { continue; }

                            // println!("Detected a score change");

                            let player = &old_data.name;
                            
                            let sound_path = 

                            // Death Check
                            if old_scores.deaths != player_score.deaths {
                                // println!("Someone has died!");
                                let path = format!("sounds.{player}.death");
    
                                if config["sounds"][player]["death"].is_object() {
    
                                    Some( path )
    
                                } else {
                                    None
                                }                            
                                
                            } 
                            // Kill Check
                            else if old_scores.kills != player_score.kills {
                                // println!("Someone has gotten a kill!");
                                let path = format!("sounds.{player}.kill");
    
                                if config["sounds"][player]["kill"].is_object() {
    
                                    Some( path )
    
                                } else {
                                    None
                                }
    
                            } else { None };
    
                            // Play sound
                            match sound_path.clone() {
                                Some(o) => {
                                    // println!("Playing a sound! {:?}", sound_path);
    
                                    if !evennt_sound_playing {
                                        evennt_sound_playing = true;
                                        let (tx_key, rx_key) = unbounded();
                                        play(false, rx_key, &o);
                                        player_data.clone().clear();
                                        filling = true;
                                        needs_emptying = true;

                                    }
    
                                },
                                None => {},
                            }

                        }

                    }

                    if filling && needs_emptying {
                        needs_emptying = false;
                        player_data.clear();
                    }

                    filling = false;
                    evennt_sound_playing = false;

                }


            }
            
        }

        let interal_time = Instant::now();

        if last_internel_time.elapsed() > Duration::from_secs_f32(0.9) && cooldown > 0.0 {
            // println!("{cooldown} | {last_game_time}");

            let current_time = get_game_time(client.clone()).await?;

            // if !(cooldown <= 0.0) {
            cooldown -= current_time - last_game_time;
            // }

            // println!("{}",cooldown);

            last_game_time = current_time;

            last_internel_time = interal_time;
        }        

        first_loop = false;
    }


    Ok(())
}

fn convert_dot_path_to_vec(path: &str) -> Vec<String> {
    path.split(".").into_iter().map(| i | { i.to_string() }).collect()
}

async fn get_game_time(client: Arc<Client>) -> Result<f64> {
    let game: SValue = serde_json::from_str(&client.get("https://127.0.0.1:2999/liveclientdata/gamestats").send().await?.text().await?)?;
    let time = game["gameTime"].as_f64().unwrap();

    Ok(time)
}

async fn calculate_cooldown(client: Arc<Client>, champ: &str, ability: &str, dd_data: &SValue) -> Result<f64> {

    let data = get_cooldown_reduction_and_id(client, ability).await?;

    let spells = dd_data["data"][champ]["spells"].as_array().unwrap();
    
    let mut cooldown = 0.0;

    for a in spells {

        let id = a["id"].as_str().unwrap().replace("\"", "");

        if data.id == id {

            let cdr = ah_to_cdr(data.ah);

            let cooldowns: Vec<f64> = a["cooldown"].as_array().unwrap().into_iter().map( | i | { i.as_f64().unwrap() }).collect();
            
            let current_cooldown = cooldowns.get((data.level-1) as usize).unwrap();

            let removed = current_cooldown*(cdr/100.0);

            cooldown = current_cooldown-removed;
            break
        }
    }

    Ok(cooldown)
}

fn ah_to_cdr(ah: f64) -> f64 {
    100.0 - ( 10000.0 / ( ah + 100.0 ) )
}

async fn get_cooldown_reduction_and_id(client: Arc<Client>, ability: &str) -> Result<Ability> {

    let res = client.get("https://127.0.0.1:2999/liveclientdata/activeplayer").send().await?;

    if res.status() != 200 { bail!("Game not Ready!") }

    let player: SValue = serde_json::from_str(&res.text().await?)?;

    let ability = ability.to_uppercase();
    
    let ability_level = player["abilities"][&ability]["abilityLevel"].as_i64().unwrap();
    let ability_id = player["abilities"][&ability]["id"].as_str().unwrap().replace("\"", "");

    let ah = player["championStats"]["abilityHaste"].as_f64().unwrap();

    let data = Ability { level: ability_level, id: ability_id, ah };

    Ok(data)
}

/// Gets the current champ and all player names
async fn game_check(client: Arc<Client>) -> Result<GameDetail> {

    let my_username = client.get("https://127.0.0.1:2999/liveclientdata/activeplayername").send().await?.text().await?;
    let my_username = my_username.replace("\"", "");
    println!("Hey {my_username}! I'm attempting to understand your game");

    let all_players: Value = serde_json::from_str(&client.get("https://127.0.0.1:2999/liveclientdata/playerlist").send().await?.text().await?)?;
    let all_players = all_players.as_array().unwrap();

    let mut all_player_names = Vec::new();

    let mut current_champ = String::new();

    for player in all_players {
        if let Some(username) = player["summonerName"].as_str() {
            let username = username.replace("\"", ""); // .unwrap().replace("\"", "")

            all_player_names.push(username.clone().to_string());

            if my_username == username {
                current_champ = player["championName"].as_str().unwrap().to_owned().replace("\"", "");
                println!("Playing {current_champ}? I see, have fun!");
            }
        };
    }

    if current_champ == String::new() { bail!("Game not loaded!"); }

    let detail = GameDetail{ players: all_player_names, champ: current_champ };

    println!("I have understood your game!");

    Ok(detail)
}

pub enum InternalState {
    ActiveGame,
    InactiveGame,
}