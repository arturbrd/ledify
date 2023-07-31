fn main() {
    let client = reqwest::blocking::Client::new();

    let client_id = ledify::ClientID::get_from_file();
    let user_auth = ledify::req_user_auth(&client_id);
    // println!("{:#?}", user_auth);
    let token = ledify::req_token(&client, user_auth, &client_id);
    println!("{:#?}", ledify::req_playback_state(&client, &token).item.name);

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
}

