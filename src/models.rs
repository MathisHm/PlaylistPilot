use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SpotifyAuthResponse {
    pub access_token: String,
}

#[derive(Debug, Deserialize)]
pub struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub struct Artist {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistResponse {
    pub tracks: PlaylistTracks,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistTracks {
    pub items: Vec<TrackItem>,
}

#[derive(Debug, Deserialize)]
pub struct TrackItem {
    pub track: Track,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub tracks: SearchTracks,
}

#[derive(Debug, Deserialize)]
pub struct SearchTracks {
    pub items: Vec<Track>,
}

#[derive(Debug, Serialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct LlmResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: MessageResponse,
}

#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LlmSongsResponse {
    pub songs: Vec<Song>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Song {
    pub name: String,
    pub artist: String,
}

#[derive(Debug, Serialize)]
pub struct AddTracksRequest {
    pub uris: Vec<String>, 
}