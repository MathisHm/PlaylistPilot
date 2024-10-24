# PlaylistPilot

PlaylistPilot is a Rust-based application designed to interact with the Spotify API to enhance playlists with songs replicating the identity of the playlist.
This project requires a Spotify account with rights to modify the playlist wanted.

## Requirements
- A Spotify account. Setup your account with an API and an URI (the URI: http://localhost:3000 is perfectly fine).
- A nvidia API key for the nvidia/llama-3.1-nemotron-70b-instruct model.
## Setup

1. **Clone the repository:**
    ```sh
    git clone https://github.com/yourusername/PlaylistPilot.git
    cd PlaylistPilot
    ```

2. **Create a `.env` file:**
    Create a `.env` file in the root directory of the project with the following variables:
    ```env
    spotify_client_id=your_spotify_client_id
    spotify_client_secret=your_spotify_client_secret
    spotify_redirect_uri=your_spotify_uri
    llm_client_secret=llm_secret_key
    playlist_id=your-playlist_id (can be found in the link when sharing your playlist)
    ```

3. **Install dependencies:**
    Ensure you have Rust installed. Then, run:
    ```sh
    cargo build
    ```

## Execution

1. **Run the application:**
    ```sh
    cargo run
    ```

2. **Authenticate with Spotify:**
    Open the link in the console (copy the whole link), go in your browser paste it and hit enter, it will most likely redirect you and say "unable to connect"
    or something, just copy the id field in the url and paste it in the console.

## LLM Model

This project uses a specific LLM model for certain functionalities. The model can be changed as long as it is compatible with the existing setup.

