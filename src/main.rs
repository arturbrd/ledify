use std::{time::{Duration, Instant}, thread::{self, JoinHandle}, sync::{Arc, Mutex, MutexGuard}};

use ledify::TokenRes;
use serialport::{SerialPort, new};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

const SLEEP_TIME: Duration = Duration::from_millis(2000);

fn main() {
    let client = reqwest::blocking::Client::new();

    let client_id = ledify::ClientID::get_from_file();
    let (exp_tx, exp_rx) = mpsc::channel();
    let mut token = TokenRes::new(&client, &client_id, exp_tx);

    let mut playback_state: ledify::PlaybackState = Default::default();
    let mut now = Instant::now();
    let mut progress = Default::default();
    let mut track_analysis = Default::default();
    let usb_connection: Arc<Mutex<Box<dyn SerialPort>>> = Arc::new(Mutex::new(serialport::new("COM3", 9600).open().expect("failed to open a COM port")));
    let (mut tx, mut rx): (Sender<bool>, Receiver<bool>) = mpsc::channel();

    let mut beats_thread: Option<thread::JoinHandle<()>> = None::<JoinHandle<()>>;

    loop {
        if exp_rx.try_recv().unwrap_or(false) {
            token = token.refresh_token(&client, &client_id);
        }
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

        let skipped = playback_state.timestamp != new_playback_state.timestamp;
        if skipped {
            println!("somthing changed");
            playback_state = new_playback_state;
            now = Instant::now();
            progress = Duration::from_millis(playback_state.progress_ms as u64);

            if another_track {
                println!("track changed");
                track_analysis = match ledify::req_track_analysis(&client, &token, &playback_state.item.id) {
                    Ok(res) => {
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
            
                        send_title_and_artists(&name, &artists_string, Arc::clone(&usb_connection));
                        res
                    },
                    Err(_) => {
                        thread::sleep(SLEEP_TIME);
                        continue;
                    }
                };
            }

            beats_thread = stop_thread(beats_thread, tx);

            (tx, rx) = mpsc::channel();
                        
            let units = track_analysis.beats.clone();
            let usb_clone = Arc::clone(&usb_connection);
            beats_thread = Some(thread::spawn(move || {
                iter_beats(units, now, progress, rx, Arc::clone(&usb_clone));
            }));
        }   
        thread::sleep(SLEEP_TIME);

    }
}

fn blink(mut usb_connection: MutexGuard<Box<dyn SerialPort>>) {
    usb_connection.write_all("1".as_bytes()).expect("failed to write to the USB");
    // println!("blink");
}

fn send_title_and_artists(title: &str, artists: &str, usb_connection: Arc<Mutex<Box<dyn SerialPort>>>) {
    let str = "0".to_owned() + title + "&" + artists;
    usb_connection.lock().unwrap().write_all(str.as_bytes()).expect("failed to write to the USB");
}

fn iter_beats(units: Vec<ledify::BBTSection>, now: Instant, progress: Duration, rx: Receiver<bool>, usb_connection: Arc<Mutex<Box<dyn SerialPort>>>) {
    let latency = Duration::from_millis(1);
    for (num, i) in units.iter().enumerate() {
        if rx.try_recv().unwrap_or(false) {
            return;
        }    
        
        if progress + now.elapsed() > Duration::from_secs_f64(i.start + 0.04) {
            continue;
        } else {
            loop {
                if progress + now.elapsed() >= Duration::from_secs_f64(i.start + i.duration - 0.001) {
                    break;
                }
                thread::sleep(latency);

            }
            if num%1 == 0 {
                let con_clone = Arc::clone(&usb_connection);
                thread::spawn(move || {
                    blink(con_clone.lock().expect("Couldn't get access to the usb"));
                });
            }
        }
            

        println!("{}", num);
    }
}


fn stop_thread(thread_handle: Option<thread::JoinHandle<()>>, tx: mpsc::Sender<bool>) -> Option<JoinHandle<()>> {
    println!("stop thread called");
    if let Some(el) = thread_handle {
        let _ = tx.send(true);
        el.join().unwrap();
        return None;
    }
    thread_handle
}
