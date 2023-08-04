use std::{time::{Duration, Instant}, thread::{self, JoinHandle}};

use ledify::TokenRes;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

const SLEEP_TIME: Duration = Duration::from_millis(3000);

fn main() {
    let client = reqwest::blocking::Client::new();

    let client_id = ledify::ClientID::get_from_file();
    let token = TokenRes::new(&client, &client_id);

    let mut playback_state: ledify::PlaybackState = Default::default();
    let mut now = Instant::now();
    let mut progress = Default::default();
    let mut track_analysis = Default::default();
    
    let (mut tx, mut rx): (Sender<bool>, Receiver<bool>) = mpsc::channel();

    let mut beats_thread: Option<thread::JoinHandle<()>> = None::<JoinHandle<()>>;

    loop {
        let new_playback_state = match ledify::req_playback_state(&client, &token) {
            Ok(res) => res,
            Err(_) => {
                thread::sleep(SLEEP_TIME);
                continue;
            }
        };
        
        if !new_playback_state.is_playing {
            beats_thread = stop_thread(beats_thread, tx.clone());
            thread::sleep(SLEEP_TIME);
            continue;
            
        }
        let another_track = new_playback_state.item.id != playback_state.item.id;
        if another_track || (progress + now.elapsed()) - Duration::from_millis(playback_state.progress_ms as u64) > Duration::from_millis(600) {
            playback_state = new_playback_state;
            now = Instant::now();
            progress = Duration::from_millis(playback_state.progress_ms as u64);

            if another_track {
                track_analysis = match ledify::req_track_analysis(&client, &token, &playback_state.item.id) {
                    Ok(res) => res,
                        Err(_) => {
                        thread::sleep(SLEEP_TIME);
                        continue;
                    }
                };
            }

            
            beats_thread = stop_thread(beats_thread, tx);

            (tx, rx) = mpsc::channel();
            let mut artists_string = String::new();
            for i in playback_state.item.artists {
                artists_string.push_str(&i.name);
                artists_string.push_str(", ");
            }
            artists_string.pop();
            artists_string.pop();

            let name = if playback_state.item.name.len() > 16 {
                playback_state.item.name[..16].to_owned()
            } else {
                playback_state.item.name
            };
            artists_string = if artists_string.len() > 16 {
                artists_string[..16].to_owned()
            } else {
                artists_string
            };
            let tatums = track_analysis.tatums.clone();
            beats_thread = Some(thread::spawn(move || {
                iter_beats(tatums, now, progress, rx, &name, &artists_string);
            }));
            thread::sleep(SLEEP_TIME);
        }   
    }
}

fn send_over_usb(title: &str, artists: &str, blink: bool) {
    // println!("{:#?}", serialport::available_ports());
    let change = match blink {
        true => "1",
        false => "0"
    };
    let mut usb = serialport::new("COM4", 9600).open().expect("failed to open a COM port");
    let str = change.to_owned() + title + "&" + artists;
    usb.write_all(str.as_bytes()).expect("failed to write to the USB");
}

fn iter_beats(beats: Vec<ledify::BBTSection>, now: Instant, progress: Duration, rx: Receiver<bool>, name: &str, artists: &str) {
    let latency = Duration::from_millis(5);
    for i in beats {
        if rx.try_recv().unwrap_or(false) {
            return;
        }
        if progress + now.elapsed() > Duration::from_secs_f64(i.start) {
            continue;
        } else {
            loop {
                if progress + now.elapsed() >= Duration::from_secs_f64(i.start + i.duration) {
                    break;
                }
                thread::sleep(latency);
            }
            send_over_usb(name, artists, true);
            // println!("{:#?}", i);
        }
    }
}

fn stop_thread(thread_handle: Option<thread::JoinHandle<()>>, tx: mpsc::Sender<bool>) -> Option<JoinHandle<()>> {
    if let Some(el) = thread_handle {
        let _ = tx.send(true);
        el.join().unwrap();
        return None;
    }
    thread_handle
}
