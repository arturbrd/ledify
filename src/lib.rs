use reqwest::blocking::Client;
use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;


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
    pub track: TrackSection
}

// a part of the TrackAnalysis
#[derive(Deserialize, Debug)]
pub struct TrackSection {
    pub tempo: f64
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
pub fn req_track_analysis(client: &Client, token: TokenRes, track_id: &str) -> TrackAnalysis {
    let track_analysis = client.get("https://api.spotify.com/v1/audio-analysis/".to_owned() + track_id)
        .header("Authorization", token.get_token())
        .send().expect("getting track audio analysis failed");
    track_analysis.json::<TrackAnalysis>().expect("failed to convert track analysis to structs")
}