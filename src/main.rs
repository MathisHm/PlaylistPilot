use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::env;
use dotenv::dotenv;

#[derive(Debug, Deserialize)]
struct SpotifyAuthResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct Track {
    name: String,
    artists: Vec<Artist>,
}

#[derive(Debug, Deserialize)]
struct Artist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct PlaylistResponse {
    tracks: PlaylistTracks,
}

#[derive(Debug, Deserialize)]
struct PlaylistTracks {
    items: Vec<TrackItem>,
}

#[derive(Debug, Deserialize)]
struct TrackItem {
    track: Track,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    tracks: SearchTracks,
}

#[derive(Debug, Deserialize)]
struct SearchTracks {
    items: Vec<Track>,
}

#[derive(Debug, Serialize)]
struct LlmRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct LlmResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageResponse,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: String,
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
        Err(e) => Err(format!("Request error: {}", e)),
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
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let llm_response: LlmResponse = response.json().map_err(|e| format!("Failed to parse response: {}", e))?;
        if let Some(choice) = llm_response.choices.get(0) {
            Ok(choice.message.content.clone())
        } else {
            Err("No response choices available".into())
        }
    } else {
        Err(format!("Error: Received status code {}", response.status()).into())
    }
}


fn search_song(access_token: &str, artist: &str, track: &str) -> Result<SearchResponse, String> {
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
                    Ok(search_response)
                },
                StatusCode::NOT_FOUND => Err("No results found for the specified artist and track.".into()),
                _ => Err(format!("Error searching for track: {}", resp.status()).into()),
            }
        },
        Err(e) => Err(format!("Request error: {}", e)),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let spotify_client_id = env::var("spotify_client_id").expect("spotify client id not set");
    let spotify_client_secret = env::var("spotify_client_secret").expect("spotify client secret key not set");
    let llm_client_secret = env::var("llm_client_secret").expect("openai client secret key not set");
    let playlist_id = "0Rbcvtg0rxWxz64eWejZ5q";

    let artist = "Radiohead";
    let track = "Creep";

    let access_token = get_spotify_access(&spotify_client_id, &spotify_client_secret)?;

    match search_song(&access_token, artist, track) {
        Ok(search_response) => {
            if let Some(track) = search_response.tracks.items.get(0) {
                println!("Track: {}", track.name);
                if let Some(artist) = track.artists.get(0) {
                    println!("Artist: {}", artist.name);
                }
            } else {
                println!("No result found");
            }
        },
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    let mut output = String::new();
    match get_playlist(&access_token, playlist_id) {
        Ok(playlist_response) => {
            for item in playlist_response.tracks.items {
                let track = item.track;
                let artist_names: Vec<String> = track.artists.iter().map(|a| a.name.clone()).collect();
                output.push_str(&format!("{} by {}, ", track.name, artist_names.join(", ")));
            }
        },
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    const X: i32 = 10;
    let prompt = &format!("I will give you a playlist, give me {X} songs that are similar to the songs in the playlist, 
        no songs that you give me should be the same as the songs in the playlist. You are only allowed to give me the songs nothing more. the format of your answer will be a json object 
        with the key 'songs' and the value being a list of song objects. Each song object should have the keys 'name' and 'artist'. Here is my playlist: {output}");

    match ask_llm(&llm_client_secret, prompt) {
        Ok(response) => println!("LLM response: {}", response),
        Err(e) => println!("Error: {}", e),
    }
    Ok(())
}
