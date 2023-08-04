use reqwest::blocking::Client;
use serde::Deserialize;
use std::{fs::{ self, File }, net::TcpStream };
use std::io::{prelude::*, BufReader};
use rand::{ thread_rng, Rng };
use sha2::{ Sha256, Digest };
use base64::{engine::general_purpose, Engine as _};
use std::net::TcpListener;

pub mod process;

const REDIRECT_URI: &str = "http://localhost:8080";

// reads from the file and holds a client id and a secret
#[derive(Deserialize, Debug)]
pub struct ClientID {
    client_id: String,
}

impl ClientID {
    pub fn get_from_file() -> ClientID {
        let mut file = File::open("client_id.json").expect("Unable to open a file");
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).expect("failed to read from file");
        serde_json::from_str(&buffer).expect("convertion from auth file to struct failed")
    }

    pub fn get(&self) -> &str {
        &self.client_id
    }
}


// holds a token
#[derive(Deserialize, Debug)]
pub struct TokenRes {
    access_token: String,
    token_type: String,
    expires_in: i32,
    refresh_token: String
}

impl TokenRes {
    pub fn new(client: &Client, client_id: &ClientID) -> TokenRes {
        match read_refresh_token_from_file() {
            Ok(refresh_token) => {
                TokenRes{access_token: String::new(), token_type: String::new(), expires_in: 0, refresh_token}.refresh_token(client, client_id)
            }
            Err(e) => {     
                println!("Error occured wher reading a file: {e}. Requesting a user auth");
                req_token(client, req_user_auth(client_id), client_id)
            }
        }
    }

    pub fn get_token(&self) -> String {
        self.token_type.clone() + " " + self.access_token.as_str()
    }

    fn refresh_token(self, client: &Client, client_id: &ClientID) -> TokenRes {
        let params = [("grant_type", "refresh_token"), ("refresh_token", self.refresh_token.as_str()), ("client_id", client_id.get())];
        let res = client.post("https://accounts.spotify.com/api/token")
            .header("Content-Type", "application/x-www-from-urlencoded")
            .form(&params)
            .send().expect("requesting refreshing token failed");
        println!("refresh token requested");
        
        let token_res = res.json::<TokenRes>().expect("failed to convert refresh_token response to struct");
        write_refresh_token_to_file(&token_res);
        token_res
        //TokenRes{access_token: String::new(), token_type: String::new(), expires_in: 0, refresh_token: String::new()}

    }
}

// holds a track analysis data from the API
#[derive(Deserialize, Debug, Default)]
pub struct TrackAnalysis {
    pub track: TrackSection,
    pub bars: Vec<BBTSection>,
    pub beats: Vec<BBTSection>,
    pub tatums: Vec<BBTSection>
}

// a part of the TrackAnalysis that holds track info
#[derive(Deserialize, Debug, Default)]
pub struct TrackSection {
    pub tempo: f64
}

// a part of the TrackAnalysis that holds bar info
#[derive(Deserialize, Debug, Default, Clone)]
pub struct BBTSection {
    pub start: f64,
    pub duration: f64,
    pub confidence: f64
}

// holds a playback state from an API
#[derive(Deserialize, Debug, Default)]
pub struct PlaybackState {
    pub item: ItemSection,
    pub progress_ms: u128,
    pub is_playing: bool
}

#[derive(Deserialize, Debug, Default)]
pub struct ItemSection {
    pub name: String,
    pub artists: Vec<ArtistSection>,
    pub id: String
}

#[derive(Deserialize, Debug, Default)]
pub struct ArtistSection {
    pub name: String
}

#[derive(Deserialize, Debug)]
pub struct UserAuth {
    pub code: String,
    pub state: String,
    pub code_verifier: String,
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

fn get_encoded_hash(str: &String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(str.as_bytes());
    general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize())
}

pub fn req_user_auth(client_id: &ClientID) -> UserAuth {
    let state = gen_rand_string(16);
    let code_verifier = gen_rand_string(128);
    let code_challenge = get_encoded_hash(&code_verifier);
    let params = vec![("response_type", "code"), ("client_id", client_id.get()), ("scope", "user-read-playback-state user-read-currently-playing"), ("redirect_uri", REDIRECT_URI), ("state", &state), ("code_challenge_method", "S256"), ("code_challenge", &code_challenge)];
    
    open::that("https://accounts.spotify.com/authorize?".to_owned() + &querystring::stringify(params)).unwrap();

    let listener = TcpListener::bind("127.0.0.1:8080").expect("TcpListener cannot be set");

    match listener.accept() {
        Ok((socket, _addr)) => {
            let user_auth = handle_connection(socket);
            println!("user auth requested");

            if user_auth.state != state {
                panic!("States are not equal, aborting");
            }
            UserAuth{code_verifier, ..user_auth}

        }
        Err(e) => panic!("couldn't get client: {e:?}")
    }
}

fn handle_connection(mut stream: TcpStream) -> UserAuth {
    let buf_reader = BufReader::new(&mut stream);
    let http_req: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    

    
    let mut get_line: Vec<_> = http_req[0].split('?').collect();
    get_line = get_line[1].split(' ').collect();
    get_line = get_line[0].split('&').collect();
    let code: Vec<_> = get_line[0].split('=').collect();
    let code = code[1].to_string();
    let state: Vec<_> = get_line[1].split('=').collect();
    let state = state[1].to_string();
    

    let status = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("ok.html").expect("reading from html file failed");
    let len = contents.len();

    let res = format!("{status}\r\nContent-Length: {len}\r\n\r\n{contents}");

    stream.write_all(res.as_bytes()).expect("sending html failed");

    UserAuth{code, state, code_verifier: String::new()}

}

// requests a token from an API
pub fn req_token(client: &Client, user_auth: UserAuth, client_id: &ClientID) -> TokenRes {
    let params = [("grant_type", "authorization_code"), ("code", &user_auth.code), ("redirect_uri", REDIRECT_URI), ("client_id", client_id.get()), ("code_verifier", &user_auth.code_verifier)];
    let res = client.post("https://accounts.spotify.com/api/token")
        .header("Content-Type", "application/x-www-from-urlencoded")
        .form(&params)
        .send().expect("sending failed");
    // println!("{:#?}", res.text());
    println!("token requested");

    let token_res = res.json::<TokenRes>().expect("failed to convert to a struct");
    write_refresh_token_to_file(&token_res);

    token_res
}

fn write_refresh_token_to_file(token_res: &TokenRes) {
    let mut file = File::create("refresh_token.txt").expect("failed to create a file");
    file.write_all(token_res.refresh_token.as_bytes()).expect("writing to file failed");
}

fn read_refresh_token_from_file() -> std::io::Result<String> {
    let mut file = File::open("refresh_token.txt")?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let buf = String::from_utf8(buf).expect("failed to convert token from file to string");
    Ok(buf)
}

// requests a track analysis from an API
pub fn req_track_analysis(client: &Client, token: &TokenRes, track_id: &str) -> reqwest::Result<TrackAnalysis> {
    let res = client.get("https://api.spotify.com/v1/audio-analysis/".to_owned() + track_id)
        .header("Authorization", token.get_token())
        .send().expect("getting track audio analysis failed");
    println!("track analysis requested");

    res.json::<TrackAnalysis>()
}

// requests a playback state from an API
pub fn req_playback_state(client: &Client, token: &TokenRes) -> reqwest::Result<PlaybackState> {
    let res = client.get("https://api.spotify.com/v1/me/player")
        .header("Authorization", token.get_token())
        .send().expect("getting track audio analysis failed");
    println!("playback state requested");

    // println!("{:#?}", res.text());
    res.json::<PlaybackState>()
}