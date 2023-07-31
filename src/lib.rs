use reqwest::blocking::Client;
use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use rand::{ thread_rng, Rng };
use sha2::{ Sha256, Digest };
use base64::{engine::general_purpose, Engine as _};

// reads from the file and holds a client id and a secret
#[derive(Deserialize, Debug)]
pub struct ApiAuth {
    client_id: String,
    client_secret: String
}

impl ApiAuth {
    pub fn get_from_file() -> ApiAuth {
        let mut file = File::open("C:\\auth\\ledify_auth.json").expect("Unable to open a file");
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).expect("failed to read from file");
        serde_json::from_str(&buffer).expect("convertion from auth file to struct failed")
    }
}


// holds a token
#[derive(Deserialize, Debug)]
pub struct TokenRes {
    access_token: String,
    token_type: String,
}

impl TokenRes {
    pub fn get_token(&self) -> String {
        self.token_type.clone() + " " + self.access_token.as_str()
    }
}

// holds a track analysis data from the API
#[derive(Deserialize, Debug)]
pub struct TrackAnalysis {
    pub track: TrackSection,
    pub bars: Vec<BarSection>
}

// a part of the TrackAnalysis that holds track info
#[derive(Deserialize, Debug)]
pub struct TrackSection {
    pub tempo: f64
}

// a part of the TrackAnalysis that holds bar info
#[derive(Deserialize, Debug)]
pub struct BarSection {
    pub start: f64,
    pub duration: f64,
    pub confidence: f64
}

// holds a playback state from an API
#[derive(Deserialize, Debug)]
pub struct PlaybackState {
    pub item: ItemSection
}

#[derive(Deserialize, Debug)]
pub struct ItemSection {
    pub name: String
}

fn gen_rand_string(len: i32) -> String {
    let mut rng = thread_rng();
    let mut text = String::new();
    let possible = String::from("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789");
    for _i in 0..len {
        let index = rng.gen_range(0..possible.len());
        let character = possible.get(index..index+1).expect("character choosing failed");
        text.push_str(character);
        
    }
    text
}

fn string_sha256(str: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(str.as_bytes());
    format!("{:x}", hasher.finalize())    
}

pub fn get_encoded_hash() -> String {
    let hash = string_sha256(gen_rand_string(128));
    general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

pub fn req_user_auth() {
    let auth = ApiAuth::get_from_file();
    let state = gen_rand_string(16);
    let code_challenge = get_encoded_hash();
    let params = vec![("response_type", "code"), ("client_id", &auth.client_id), ("scope", "user-read-playback-state user-read-currently-playing"), ("redirect_uri", "http://localhost:8080"), ("state", &state), ("code_challenge_method", "S256"), ("code_challenge", &code_challenge)];
    
    open::that("https://accounts.spotify.com/authorize?".to_owned() + &querystring::stringify(params)).unwrap();
}

// requests a token from an API
pub fn req_token(client: &Client) -> TokenRes {
    let auth = ApiAuth::get_from_file();
    let params = [("grant_type", "client_credentials"), ("client_id", &auth.client_id), ("client_secret", &auth.client_secret)];
    let res = client.post("https://accounts.spotify.com/api/token")
        .header("Content-Type", "application/x-www-from-urlencoded")
        .form(&params)
        .send().expect("sending failed");
    res.json::<TokenRes>().expect("failed to convert to a struct")
}

// requests a track analysis from an API
pub fn req_track_analysis(client: &Client, token: &TokenRes, track_id: &str) -> TrackAnalysis {
    let res = client.get("https://api.spotify.com/v1/audio-analysis/".to_owned() + track_id)
        .header("Authorization", token.get_token())
        .send().expect("getting track audio analysis failed");
    res.json::<TrackAnalysis>().expect("failed to convert track analysis to structs")
}

// requests a playback state from an API
pub fn req_playback_state(client: &Client, token: &TokenRes) -> PlaybackState {
    let res = client.get("https://api.spotify.com/v1/me/player")
        .header("Authorization", token.get_token())
        .send().expect("getting track audio analysis failed");
    println!("{:#?}", res);
    res.json::<PlaybackState>().expect("failed to convert playback state to structs")
}