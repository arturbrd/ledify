use std::{time::{Duration, Instant}, thread};

use ledify::TokenRes;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

fn main() {
    let client = reqwest::blocking::Client::new();

    let client_id = ledify::ClientID::get_from_file();
    let token = TokenRes::new(&client, &client_id);

    let mut playback_state: ledify::PlaybackState = Default::default();
    let mut now;
    let mut progress;
    let mut track_analysis = Default::default();
    
    let (mut tx, mut rx): (Sender<bool>, Receiver<bool>) = mpsc::channel();

    let mut beats_thread: Option<thread::JoinHandle<_>> = None;

    loop {
        let new_playback_state = match ledify::req_playback_state(&client, &token) {
            Ok(res) => res,
            Err(_) => continue
        };
        
        if new_playback_state.item.id != playback_state.item.id {
            playback_state = new_playback_state;
            now = Instant::now();
            progress = Duration::from_millis(playback_state.progress_ms as u64);

            track_analysis = match ledify::req_track_analysis(&client, &token, &playback_state.item.id) {
                Ok(res) => res,
                Err(e) => continue
            };

            tx.send(true).unwrap();
            if let Some(el) = beats_thread {
                el.join().unwrap();
            }
            //stop thread
            (tx, rx) = mpsc::channel();
            let mut artists_string = String::new();
            for i in playback_state.item.artists {
                artists_string.push_str(&i.name);
                artists_string.push_str(", ");
            }
            artists_string.pop();
            artists_string.pop();

            beats_thread = Some(thread::spawn(move || {
                iter_beats(track_analysis.beats, now, progress, rx, &playback_state.item.name, &artists_string);
            }));
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
            }
            send_over_usb(name, artists, true);
        }
    }
}
