// Import necessary modules and crates
use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::env;
use dotenv::dotenv;
use std::collections::HashMap;

// Import models
mod models;
use models::*;

// Helper function to parse the LLM response
// Cleans the response by trimming and removing surrounding backticks (`) if present.
fn parse_llm_response(response: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cleaned_response = response.trim().trim_matches('`');
    Ok(cleaned_response.to_string())
}

// Function to exchange the authorization code for an access token
fn get_spotify_access(
    client_id: &str, 
    client_secret: &str, 
    code: &str, 
    redirect_uri: &str
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let auth_url = "https://accounts.spotify.com/api/token";

    // Prepare the request body as a HashMap
    let mut body = HashMap::new();
    body.insert("grant_type", "authorization_code");
    body.insert("code", code);
    body.insert("redirect_uri", redirect_uri);
    body.insert("client_id", client_id);
    body.insert("client_secret", client_secret);

    // Send POST request to the Spotify token endpoint
    let auth_response: SpotifyAuthResponse = client
        .post(auth_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&body)
        .send()?
        .json()?;

    // Return the access token from the response
    Ok(auth_response.access_token)
}

// Function to generate the Spotify authorization URL
fn get_authorization_url(client_id: &str, redirect_uri: &str) -> String {
    let scopes = "playlist-modify-public playlist-modify-private";
    format!(
        "https://accounts.spotify.com/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}",
        client_id, scopes, redirect_uri
    )
}

// Function to fetch a playlist from Spotify using its ID and an access token
fn get_playlist(access_token: &str, playlist_id: &str) -> Result<PlaylistResponse, String> {
    let client = Client::new();
    let playlist_url = format!("https://api.spotify.com/v1/playlists/{}", playlist_id);

    let response = client
        .get(&playlist_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send();

    // Handle the response and map to PlaylistResponse
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

// Function to interact with an LLM API to generate new song suggestions
fn ask_llm(api_key: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = "https://integrate.api.nvidia.com/v1/chat/completions";

    // Prepare the request body with model and prompt
    let request_body = LlmRequest {
        model: "nvidia/llama-3.1-nemotron-70b-instruct".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    // Send the request to the LLM API
    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| format!("{}", e))?;

    // Parse the response
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

// Function to search for a specific song by artist and track name on Spotify
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

    // Handle the response and return the first track's URI if found
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

// Function to add tracks to a playlist by their URIs
fn add_to_playlist(access_token: &str, playlist_id: &str, uris: Vec<String>) -> Result<(), String> {
    let client = Client::new();
    let playlist_url = format!("https://api.spotify.com/v1/playlists/{playlist_id}/tracks");

    let body = AddTracksRequest { uris };

    // Send POST request to add tracks to the playlist
    let response = client
        .post(&playlist_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send();

    // Check if the operation was successful
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

// Main function to handle user input and the entire process flow
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    // Read necessary environment variables
    let spotify_client_id = env::var("spotify_client_id").expect("spotify client id not set");
    let spotify_client_secret = env::var("spotify_client_secret").expect("spotify client secret key not set");
    let spotify_redirect_uri = env::var("spotify_redirect_uri").expect("spotify redirect uri not set");
    let llm_client_secret = env::var("llm_client_secret").expect("llm client secret key not set");
    let playlist_id = env::var("playlist_id").expect("playlist id not set");

    // Ask the user how many songs they want to add
    println!("Enter the number of songs you want to add to the playlist:");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)
        .expect("Failed to read input");

    let number: i32 = input.trim().parse()
        .expect("Please enter a valid number");

    // Generate Spotify authorization URL and instruct the user to visit it
    let auth_url = get_authorization_url(&spotify_client_id, &spotify_redirect_uri);
    println!("Go to this URL to authorize: {}", auth_url);

    // Get the authorization code from the user
    let mut code = String::new();
    println!("Enter the authorization code:");
    std::io::stdin().read_line(&mut code)?;
    let code = code.trim();

    // Obtain access token using the authorization code
    let access_token = get_spotify_access(&spotify_client_id, &spotify_client_secret, &code, &spotify_redirect_uri)?;

    // Fetch the playlist and format the output for the LLM prompt
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

    // Prepare prompt for the LLM to generate similar songs
    let prompt = &format!(
        "I will give you a playlist, give me {number} songs that are similar to the songs in the playlist, \
        no songs that you give me should be the same as the songs in the playlist. Your goal is to give me songs that fit the vibe of the playlist. \
        You are only allowed to give me the songs nothing more. The format of your answer will be a JSON object \
        with the key 'songs' and the value being a list of song objects. Each song object should have the keys 'name' and 'artist'. Here is the playlist: {output}"
    );

    // Ask the LLM for song suggestions and search for their URIs on Spotify
    let mut uris_to_add = Vec::new();
    match ask_llm(&llm_client_secret, prompt) {
        Ok(response) => {
            match parse_llm_response(&response) {
                Ok(cleaned_response) => {
                    let llm_songs: LlmSongsResponse = serde_json::from_str(&cleaned_response)?;
                    for song in llm_songs.songs {
                        match search_song(&access_token, &song.artist, &song.name) {
                            Ok(uri) => uris_to_add.push(uri),
                            Err(e) => println!("Error finding song '{} - {}': {}", song.name, song.artist, e),
                        }
                    }
                },
                Err(e) => println!("{}", e),
            }
        },
        Err(e) => println!("{}", e),
    }

    // If songs are found, add them to the playlist
    if !uris_to_add.is_empty() {
        match add_to_playlist(&access_token, &playlist_id, uris_to_add) {
            Ok(_) => println!("Successfully added songs to the playlist."),
            Err(e) => println!("{}", e),
        }
    }
    Ok(())
}
