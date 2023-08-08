use std::{time::{Duration, Instant}, thread::{self, JoinHandle}, sync::{Arc, Mutex, MutexGuard}};

use ledify::TokenRes;
use serialport::SerialPort;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

const SLEEP_TIME: Duration = Duration::from_millis(3500);

fn main() {
    let client = reqwest::blocking::Client::new();

    let client_id = ledify::ClientID::get_from_file();
    let (exp_tx, exp_rx) = mpsc::channel();
    let mut token = TokenRes::new(&client, &client_id, exp_tx);

    let mut playback_state: ledify::PlaybackState = Default::default();
    let mut now = Instant::now();
    let mut progress = Default::default();
    let mut track_analysis = Default::default();
    let usb_connection: Arc<Mutex<Box<dyn SerialPort>>> = Arc::new(Mutex::new(serialport::new("COM4", 9600).open().expect("failed to open a COM port")));
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
        // println!("{:#?}", new_playback_state);

        
        if !new_playback_state.is_playing {
            beats_thread = stop_thread(beats_thread, tx.clone());
            thread::sleep(SLEEP_TIME);
            continue;
        }
        let another_track = new_playback_state.item.id != playback_state.item.id;

        let skipped = playback_state.timestamp != new_playback_state.timestamp;
        if skipped {
            // println!("skipped\n");
            playback_state = new_playback_state;
            now = Instant::now();
            progress = Duration::from_millis(playback_state.progress_ms as u64);

            if another_track {
                // println!("track changed\n");
                track_analysis = match ledify::req_track_analysis(&client, &token, &playback_state.item.id) {
                    Ok(res) => {
                        let mut artists_string = String::new();
                        for i in playback_state.item.artists {
                            artists_string.push_str(&i.name);
                            artists_string.push_str(", ");
                        }
                        artists_string.pop();
                        artists_string.pop();                        
                        
                        playback_state.item.name = convert_polish(playback_state.item.name );
                        playback_state.item.name = brutal_cut(playback_state.item.name );

                        artists_string = convert_polish(artists_string);
                        artists_string = brutal_cut(artists_string);
            
                        send_title_and_artists(&playback_state.item.name, &artists_string, Arc::clone(&usb_connection));
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
            let sections = track_analysis.sections.clone();
            let usb_clone = Arc::clone(&usb_connection);
            beats_thread = Some(thread::spawn(move || {
                iter_beats(units, sections, now, progress, rx, Arc::clone(&usb_clone));
            }));
        }   
        thread::sleep(SLEEP_TIME);

    }
}

fn brutal_cut(text: String) -> String {
    let mut buf = Vec::new();
    for (i, char) in text.bytes().enumerate() {
        if i > 15 {
            break;
        }
        buf.push(char);
    }
    String::from_utf8_lossy(&buf).to_string()
}

fn convert_polish(mut text: String) -> String {
    text = text.replace(|c: char| c == 'ą', "a");
    text = text.replace(|c: char| c == 'ć', "c");
    text = text.replace(|c: char| c == 'ę', "e");
    text = text.replace(|c: char| c == 'ł', "l");
    text = text.replace(|c: char| c == 'ń', "n");
    text = text.replace(|c: char| c == 'ó', "o");
    text = text.replace(|c: char| c == 'ś', "s");
    text = text.replace(|c: char| c == 'ź' || c == 'ż', "z");

    text = text.replace(|c: char| c == 'Ą', "A");
    text = text.replace(|c: char| c == 'Ć', "C");
    text = text.replace(|c: char| c == 'Ę', "E");
    text = text.replace(|c: char| c == 'Ł', "L");
    text = text.replace(|c: char| c == 'Ń', "N");
    text = text.replace(|c: char| c == 'Ó', "O");
    text = text.replace(|c: char| c == 'Ś', "S");
    text = text.replace(|c: char| c == 'Ź' || c == 'Ż', "Z");

    text = text.replace(|c: char| !c.is_ascii(), "?");
    text
}

fn blink(mut usb_connection: MutexGuard<Box<dyn SerialPort>>, section: ledify::SectionSection) {
    let mode = if section.loudness > -11.5 {"1"} else {"0"};
    let str = "1".to_owned() + mode;
    usb_connection.write_all(str.as_bytes()).expect("failed to write to the USB");
    // println!("{}: {}", section.start, section.loudness);
}

fn send_title_and_artists(title: &str, artists: &str, usb_connection: Arc<Mutex<Box<dyn SerialPort>>>) {
    let str = "0".to_owned() + title + "&" + artists;
    usb_connection.lock().unwrap().write_all(str.as_bytes()).expect("failed to write to the USB");
    thread::sleep(Duration::from_millis(5));
}

fn iter_beats(units: Vec<ledify::BBTSection>, sections: Vec<ledify::SectionSection>, now: Instant, progress: Duration, rx: Receiver<bool>, usb_connection: Arc<Mutex<Box<dyn SerialPort>>>) {
    let latency = Duration::from_millis(1);
    let mut sections = sections.iter();
    let mut section = sections.next();
    for i in units.iter() {
        if rx.try_recv().unwrap_or(false) {
            return;
        }

        if let Some(inner) = section {
            if inner.start + inner.duration <= i.start + 0.001 {
                section = sections.next();
            }
        }
        if Duration::from_secs_f64(i.start + 0.05) < progress + now.elapsed() {
            // println!("{num} skipped");
            continue;
        }
        while progress + now.elapsed() < Duration::from_secs_f64(i.start - 0.01) {
            thread::sleep(latency);
        }
        let con_clone = Arc::clone(&usb_connection);
        let section_clone = section.unwrap().clone();
        // println!("{}: {}", num, i.start);

        thread::spawn(move || {
            blink(con_clone.lock().expect("Couldn't get access to the usb"), section_clone);
        });
    }
}


fn stop_thread(thread_handle: Option<thread::JoinHandle<()>>, tx: mpsc::Sender<bool>) -> Option<JoinHandle<()>> {
    // println!("stop thread called");
    if let Some(el) = thread_handle {
        let _ = tx.send(true);
        el.join().unwrap();
        return None;
    }
    thread_handle
}
