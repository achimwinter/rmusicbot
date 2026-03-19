# RMusicBot

A Discord music bot written in Rust.

### Introduction

RMusicBot is a Discord music bot built with [Serenity](https://github.com/serenity-rs/serenity) and [Songbird](https://github.com/serenity-rs/songbird). It provides YouTube audio playback with support for direct links, playlists, live streams, and search queries.

The bot supports Discord's DAVE (Discord Audio Video Encryption) protocol for encrypted voice channels.

RMusicBot is open-sourced under the AGPL-3.0 License, see the [LICENSE](LICENSE) file.

### Features

| Command | Alias | Description |
|---------|-------|-------------|
| `play <url/query>` | `p` | Play or queue a song from YouTube URL or search |
| `pause` | | Pause the current song |
| `resume` | | Resume playback |
| `skip` | | Skip the current song |
| `stop` | | Stop playback and clear the queue |
| `clear` | | Clear the queue |
| `current` | | Show information about the current song |
| `leave` | | Leave the voice channel |
| `help` | | Display the help menu |

### Prerequisites

Before running RMusicBot, ensure you have the following installed:

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) - YouTube video/audio downloader
- [ffmpeg](https://ffmpeg.org/) - Audio processing

### Installation

#### Docker (Recommended)

Build and run using Docker:

```bash
docker build -t rmusicbot .
docker run -d \
  --name rmusicbot \
  -e DISCORD_TOKEN=your_token_here \
  -e PREFIX=~ \
  -e DISCORD_STATUS=Music \
  --restart unless-stopped \
  rmusicbot
```

#### Docker Compose

```bash
# Copy and configure environment
cp .env.example .env
# Edit .env with your Discord token

# Run
docker compose up -d

# View logs
docker compose logs -f
```

#### Build from Source

Ensure you have Rust installed, then:

```bash
# Clone the repository
git clone https://github.com/yourusername/rmusicbot.git
cd rmusicbot

# Build for production
cargo build --release --no-default-features

# Run
./target/release/rmusicbot
```

For development (with .env file support):

```bash
cargo run
```

### Configuration

RMusicBot is configured using environment variables:

| Variable | Required | Description |
|----------|----------|-------------|
| `DISCORD_TOKEN` | Yes | Your Discord bot token from the [Developer Portal](https://discord.com/developers/applications) |
| `PREFIX` | Yes | Command prefix (e.g., `~`, `!`, `.`) |
| `DISCORD_STATUS` | Yes | Bot status message displayed in Discord |

For development, create a `.env` file in the project root:

```env
DISCORD_TOKEN=your_token_here
PREFIX=~
DISCORD_STATUS=Music
```

### Discord Bot Setup

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application and add a bot
3. Enable the following **Privileged Gateway Intents**:
   - Message Content Intent
   - Server Members Intent (optional)
4. Generate an invite URL with these permissions:
   - Send Messages
   - Embed Links
   - Connect
   - Speak
   - Use Voice Activity
5. Invite the bot to your server using the generated URL

### Usage

1. Join a voice channel in Discord
2. Use the play command with a YouTube URL or search query:
   ```
   ~play https://www.youtube.com/watch?v=dQw4w9WgXcQ
   ~play never gonna give you up
   ```
3. The bot will join your voice channel and start playing

### Dependencies

- [Serenity](https://github.com/serenity-rs/serenity) - Discord API library for Rust
- [Songbird](https://github.com/serenity-rs/songbird) - Discord voice library
- [Symphonia](https://github.com/pdeljanov/Symphonia) - Audio decoding
- [Tokio](https://tokio.rs/) - Async runtime

### License

This project is licensed under the GNU Affero General Public License v3.0 - see the [LICENSE](LICENSE) file for details.
