fn main() {
    let track_id = "0e7ipj03S05BNilyu5bRzt";
    let client = reqwest::blocking::Client::new();
    
    let token = ledify::req_token(&client);

    println!("{}", token.get_token());

    let track_analysis = ledify::req_track_analysis(&client, &token, track_id);
    
    println!("{}", track_analysis.track.tempo);

    for i in &track_analysis.bars {
        println!("{:#?}", i);
    }
    println!("A number of bars: {}", track_analysis.bars.len());

    let playback_state = ledify::req_playback_state(&client, &token);
    println!("Currently played song: {}", playback_state.item.name);
}

