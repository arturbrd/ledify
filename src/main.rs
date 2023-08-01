use ledify::TokenRes;

fn main() {
    let client = reqwest::blocking::Client::new();

    let client_id = ledify::ClientID::get_from_file();
    // println!("{:#?}", user_auth);
    let token = TokenRes::new(&client, &client_id);
    println!("{:#?}", ledify::req_playback_state(&client, &token).item.name);

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
}

