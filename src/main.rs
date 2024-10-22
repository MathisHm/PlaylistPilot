use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::env;
use dotenv::dotenv;

mod models;
use models::*;

fn parse_llm_response(response: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cleaned_response = response.trim().trim_matches('`');
    Ok(cleaned_response.to_string())
}

fn get_spotify_access(client_id: &str, client_secret: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let auth_url = "https://accounts.spotify.com/api/token";

    let auth_response: SpotifyAuthResponse = client
        .post(auth_url)
        .basic_auth(client_id, Some(client_secret))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("grant_type=client_credentials")
        .send()?
        .json()?;

    Ok(auth_response.access_token)
}



fn get_playlist(access_token: &str, playlist_id: &str) -> Result<PlaylistResponse, String> {
    let client = Client::new();
    let playlist_url = format!("https://api.spotify.com/v1/playlists/{}", playlist_id);

    let response = client
        .get(&playlist_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send();

    match response {
        Ok(resp) => {
            match resp.status() {
                StatusCode::OK => {
                    let playlist_response: PlaylistResponse = resp.json().map_err(|e| e.to_string())?;
                    Ok(playlist_response)
                },
                StatusCode::NOT_FOUND => Err("Invalid Playlist ID: The playlist could not be found.".into()),
                _ => Err(format!("Error fetching playlist: {}", resp.status()).into()),
            }
        },
        Err(e) => Err(format!("{}", e)),
    }
}

fn ask_llm(api_key: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = "https://integrate.api.nvidia.com/v1/chat/completions";

    let request_body = LlmRequest {
        model: "nvidia/llama-3.1-nemotron-70b-instruct".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| format!("{}", e))?;

    if response.status().is_success() {
        let llm_response: LlmResponse = response.json().map_err(|e| format!("Failed to parse response: {}", e))?;
        if let Some(choice) = llm_response.choices.get(0) {
            Ok(choice.message.content.clone())
        } else {
            Err("No response choices available".into())
        }
    } else {
        Err(format!("{}", response.status()).into())
    }
}


fn search_song(access_token: &str, artist: &str, track: &str) -> Result<String, String> {
    let client = Client::new();
    let search_url = format!(
        "https://api.spotify.com/v1/search?q=artist:{}+track:{}&type=track&limit=1",
        artist, track
    );

    let response = client
        .get(&search_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send();

    match response {
        Ok(resp) => {
            match resp.status() {
                StatusCode::OK => {
                    let search_response: SearchResponse = resp.json().map_err(|e| e.to_string())?;
                    if let Some(track) = search_response.tracks.items.get(0) {
                        Ok(track.uri.clone())
                    } else {
                        Err("No result found for the specified artist and track.".into())
                    }
                },
                StatusCode::NOT_FOUND => Err("No results found for the specified artist and track.".into()),
                _ => Err(format!("{}", resp.status()).into()),
            }
        },
        Err(e) => Err(format!("{}", e)),
    }
}

fn add_to_playlist(access_token: &str, playlist_id: &str, uris: Vec<String>) -> Result<(), String> {
    let client = Client::new();
    let playlist_url = format!("https://api.spotify.com/v1/playlists/{playlist_id}/tracks");

    let body = AddTracksRequest { uris };

    let response = client
        .post(&playlist_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send();

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(())
            } else {
                Err(format!("Failed to add tracks to playlist: {}", resp.status()))
            }
        },
        Err(e) => Err(format!("{}", e)),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let spotify_client_id = env::var("spotify_client_id").expect("spotify client id not set");
    let spotify_client_secret = env::var("spotify_client_secret").expect("spotify client secret key not set");
    let llm_client_secret = env::var("llm_client_secret").expect("llm client secret key not set");
    let playlist_id = env::var("playlist_id").expect("playlist id not set");

    let access_token = get_spotify_access(&spotify_client_id, &spotify_client_secret)?;

    let mut output = String::new();
    match get_playlist(&access_token, &playlist_id) {
        Ok(playlist_response) => {
            for item in playlist_response.tracks.items {
                let track = item.track;
                let artist_names: Vec<String> = track.artists.iter().map(|a| a.name.clone()).collect();
                output.push_str(&format!("{} by {}, ", track.name, artist_names.join(", ")));
            }
        },
        Err(e) => {
            println!("{}", e);
        }
    }

    const X: i32 = 5;
    let prompt = &format!("I will give you a playlist, give me {X} songs that are similar to the songs in the playlist, 
        no songs that you give me should be the same as the songs in the playlist. Your goal is to give me songs that fit the vibe of the playlist.
        You are only allowed to give me the songs nothing more. the format of your answer will be a json object 
        with the key 'songs' and the value being a list of song objects. Each song object should have the keys 'name' and 'artist'. Here is the playlist: {output}");

    let mut uris_to_add = Vec::new();

    // Request songs from LLM and search for their URIs on Spotify
    match ask_llm(&llm_client_secret, prompt) {
        Ok(response) => {
            match parse_llm_response(&response) {
                Ok(cleaned_response) => {
                    // Now we can safely deserialize the cleaned response
                    let llm_songs: LlmSongsResponse = serde_json::from_str(&cleaned_response)?;
                    
                    // Iterate over the songs and search for their Spotify URIs
                    for song in llm_songs.songs {
                        match search_song(&access_token, &song.artist, &song.name) {
                            Ok(uri) => uris_to_add.push(uri),
                            Err(e) => println!("Error finding song '{} - {}': {}", song.name, song.artist, e),
                        }
                    }
                },
                Err(e) => println!("Error parsing LLM response: {}", e),
            }
        },
        Err(e) => println!("{}", e),
    }

    // Add the found URIs to the playlist
    if !uris_to_add.is_empty() {
        match add_to_playlist(&access_token, &playlist_id, uris_to_add) {
            Ok(_) => println!("Successfully added songs to the playlist."),
            Err(e) => println!("{}", e),
        }
    }
    Ok(())
}
