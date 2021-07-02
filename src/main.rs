use std::fs;
use std::process::Command;

extern crate discord;

use discord::model::{ChannelId, Event, ServerId, UserId};
use discord::{Connection, Discord, State};
use serde_json::Value;

struct Session {
    discord: Discord,
    connection: Connection,
    state: State,
    cache: String,
}

// enum command {
// play,
// stop,
// }

impl Session {
    fn login(token: String, cache: &str) -> &Self {
        let discord = Discord::from_bot_token(&token).expect("login failed");
        let (mut connection, ready) = discord.connect().expect("connect failed");
        let mut state = State::new(ready);
        connection.sync_calls(&state.all_private_channels());

        // Ensure cache exists
        fs::create_dir_all(cache).expect("could not create cache dir");

        Session {
            discord: discord,
            connection: connection,
            state: state,
            cache: cache,
        }
    }

    fn get_chan(&self, id: UserId) -> Option<(Option<ServerId>, ChannelId)> {
        self.state.find_voice_user(id)
    }

    fn play(&self, id: UserId, link: &str) {
        let vchan = self.get_chan(id);
        let mut output = String::new();

        match self.get_chan(id) {
            Some((server_id, channel_id)) => {
                warn(self.discord.send_message(
                    channel_id,
                    &format!("Searching for \"{}\"...", link),
                    "",
                    false,
                ));
                let output = Command::new("youtube-dl")
                    .arg("-f")
                    .arg("webm[abr>0]/bestaudio/best")
                    .arg("--output")
                    .arg(format!("{}/%(title)s.%(ext)s", self.cache))
                    .arg("--print-json")
                    .arg("--default-search")
                    .arg("ytsearch")
                    .arg(&link)
                    .output()
                    .expect("failed to spawn youtube-dl process");
                if output.status.success() {
                    let video_meta: Value = serde_json::from_slice(&output.stdout)
                        .expect("Failed to parse youtube-dl output");
                    warn(self.discord.send_message(
                        channel_id,
                        &format!(
                            "Playing **{}** ({})",
                            video_meta["title"].as_str().unwrap(),
                            video_meta["webpage_url"].as_str().unwrap()
                        ),
                        "",
                        false,
                    ));
                    match discord::voice::open_ffmpeg_stream(
                        video_meta["_filename"].as_str().unwrap(),
                    ) {
                        Ok(stream) => {
                            let voice = self.connection.voice(server_id);
                            voice.set_deaf(true);
                            voice.connect(channel_id);
                            voice.play(stream);
                            String::new()
                        }
                        Err(error) => format!("Error: {}", error),
                    }
                } else {
                    format!("Error: {}", String::from_utf8_lossy(&output.stderr))
                }
            }
            _ => "You must be in a voice channel to DJ".to_owned(),
        }

        if !output.is_empty() {
            warn(self.discord.send_message(channel_id, &output, "", false));
        }
    }
}

fn main() {}
