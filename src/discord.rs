use crate::api::{NowOnAir, SongList};
use crate::voice::VoiceManager;
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use serenity::model::channel::{ChannelType, GuildChannel};
use serenity::model::gateway::Activity;
use serenity::model::id::GuildId;
use serenity::model::user::OnlineStatus;
use serenity::prelude::{Context, EventHandler, Mutex};
use serenity::{async_trait, framework::standard::StandardFramework};
use serenity::{/*voice,**/ Client};
use std::env;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration as StdDuration;

type Channels = Mutex<Vec<GuildChannel>>;

struct Handler {
    song_list: SongList,
    text_channels: Channels,
    voice_channels: Channels,
    now_on_air: Option<NowOnAir>,
    force_server: Option<u64>,
}

impl Handler {
    async fn async_clone(&self) -> Self {
        let text_channels_lock = self.text_channels.lock().await;
        let voice_channels_lock = self.voice_channels.lock().await;
        Handler {
            song_list: self.song_list.clone(),
            text_channels: Mutex::new(text_channels_lock.clone()),
            voice_channels: Mutex::new(voice_channels_lock.clone()),
            now_on_air: self.now_on_air.clone(),
            force_server: self.force_server,
        }
    }
}

impl Handler {
    pub fn new(song_list: SongList, force_server: Option<u64>) -> Handler {
        Handler {
            song_list,
            text_channels: Mutex::new(vec![]),
            voice_channels: Mutex::new(vec![]),
            now_on_air: None,
            force_server,
        }
    }

    // async fn join_voice_channels(&self, ctx: &Context) {
    //     let voice_channels = self.voice_channels.lock().await;
    //     let voice_ref = &*voice_channels;

    //     let manager_lock = ctx
    //         .data
    //         .read()
    //         .await
    //         .get::<VoiceManager>()
    //         .cloned()
    //         .unwrap();
    //     let mut manager = manager_lock.lock().await;

    //     for channel in voice_ref {
    //         if manager.get(channel.guild_id).is_some() {
    //             manager.remove(channel.guild_id);
    //             println!("Rejoining channel top2000 on server {}!", channel.guild_id);
    //         }

    //         let joined_channel = manager.join(channel.guild_id, channel.id);
    //         if let Some(handler) = joined_channel {
    //             println!("Joined channel top2000 on server {}!", channel.guild_id);

    //             let source = match voice::ytdl("https://icecast.omroep.nl/radio2-bb-mp3").await {
    //                 Ok(source) => source,
    //                 Err(why) => {
    //                     panic!("Err starting source: {:?}", why);
    //                 }
    //             };

    //             handler.play_only(source);
    //         } else {
    //             panic!("Failed to join channel on server {}!", channel.guild_id);
    //         }
    //     }
    // }

    async fn background_loop(&self, ctx: &Context) {
        let mut self_clone = self.async_clone().await;
        let ctx_clone = ctx.clone();

        loop {
            let now_on_air = self_clone.song_list.get_now_on_air().await;

            if let Ok(on_air) = now_on_air {
                let prev_now_on_air = self_clone.now_on_air.as_ref();
                if prev_now_on_air.is_none()
                    || on_air.song.id != prev_now_on_air.unwrap().song.id
                    || on_air.song.title != prev_now_on_air.unwrap().song.title
                    || on_air.song.artist != prev_now_on_air.unwrap().song.artist
                      // ensures no jumping around occurs when multiple songs are somehow in the currently playing list
                {
                    self_clone.now_on_air = Some(on_air.clone());

                    self_clone.update_presence(&ctx_clone, &on_air).await;
                    self_clone.generate_embed(&ctx_clone, &on_air).await;
                    self_clone.handle_first_place(&ctx_clone, &on_air).await;
                    let title = &on_air.song.title;

                    let duration = StdDuration::from_secs(15);

                    println!(
                        "New song: {}. Sleeping for {} seconds",
                        title,
                        duration.as_secs()
                    );

                    tokio::time::delay_for(duration).await;

                    continue;
                }
            } else {
                println!("Getting now on air failed miserably!");
            }

            tokio::time::delay_for(StdDuration::from_secs(15)).await;
        }
    }

    async fn handle_first_place(&self, ctx: &Context, now_on_air: &NowOnAir) {
        let first_song = match self.song_list.get_song(0) {
            Some(s) => s,
            _ => return,
        };

        if now_on_air.song.id == first_song.id {
            loop {
                let date2022_res = DateTime::from_str("2022-01-01T00:00:00+01:00");
                if let Ok(date2022) = date2022_res {
                    let now_min_2022 = Utc::now() - date2022;
                    if now_min_2022.num_seconds() > 0 {
                        let text_channels = self.text_channels.lock().await;
                        let text_ref = &*text_channels;

                        for text_channel in text_ref {
                            let _ = text_channel.send_message(ctx, |m| {
                                m.content("Happy new year @everyone! :partying_face:")
                            });
                        }

                        exit(0);
                    }
                }
                tokio::time::delay_for(StdDuration::from_secs(1)).await;
            }
        }
    }

    async fn generate_embed(&self, ctx: &Context, now_on_air: &NowOnAir) {
        let text_channels = self.text_channels.lock().await;
        let text_ref = &*text_channels;

        let description = now_on_air
            .song
            .get_description()
            .await
            .unwrap_or_else(|_| "".to_string());

        for text_channel in text_ref {
            let sent_message = text_channel
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        let date2022_res = DateTime::from_str("2022-01-01T00:00:00+01:00");
                        let minutes_till_2022 = if let Ok(date2022) = date2022_res {
                            let minutes = (date2022 - Utc::now()).num_minutes();

                            format!("{:02}:{:02}", (minutes / 60) as u64, (minutes % 60) as u64)
                        } else {
                            "unknown".to_string()
                        };

                        let curr_position = now_on_air
                            .song
                            .position
                            .map_or("unknown".to_string(), |f| f.to_string());

                        let last_year_position = now_on_air
                            .song
                            .get_last_year_position()
                            .map(|f| {
                                let current_song_pos = now_on_air.song.position;
                                let emoji = match (f, current_song_pos) {
                                    (prev_pos, Some(cur_pos)) if prev_pos == cur_pos => "🔵",
                                    (prev_pos, Some(cur_pos)) if prev_pos > cur_pos => "🔼",
                                    (prev_pos, Some(cur_pos)) if prev_pos < cur_pos => "🔽",
                                    _ => "",
                                };

                                format!(" (last year: {} {})", f.to_string(), emoji)
                            })
                            .unwrap_or_else(|| "".to_string());

                        e.title(format!(
                            "{} by {}",
                            now_on_air.song.title, now_on_air.song.artist
                        ))
                        .description(description.clone())
                        .image(now_on_air.img_url.as_ref().unwrap_or(&"".to_string()))
                        .field(
                            "Position",
                            format!("{}{}", curr_position, last_year_position),
                            true,
                        )
                        .field("Time until 2022", minutes_till_2022, true)
                        .url(format!("https://www.nporadio2.nl{}", now_on_air.song.url))
                    })
                })
                .await;

            if let Err(err) = sent_message {
                println!("Failed sending message, {}", err);
            }
        }
    }

    async fn update_presence(&self, ctx: &Context, now_on_air: &NowOnAir) {
        let activity = Activity::listening(
            format!("{} by {}", now_on_air.song.title, now_on_air.song.artist).as_ref(),
        );
        ctx.set_presence(Some(activity), OnlineStatus::Online).await;
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        {
            let mut text_channels = self.text_channels.lock().await;
            let mut voice_channels = self.voice_channels.lock().await;

            for guild in guilds {
                if self.force_server.is_none() || guild.0 == self.force_server.unwrap() {
                    let channels_res = ctx.http.get_channels(guild.0).await;

                    if let Ok(channels) = channels_res {
                        for channel in channels {
                            if channel.name == "top2000" && channel.kind == ChannelType::Text {
                                let ref_vec = &mut *text_channels;
                                ref_vec.push(channel);
                            } else if channel.name == "top2000"
                                && channel.kind == ChannelType::Voice
                            {
                                let ref_vec = &mut *voice_channels;
                                ref_vec.push(channel);
                            }
                        }
                    }
                }
            }
        }

        // self.join_voice_channels(&ctx).await;
        self.background_loop(&ctx).await;
    }
}

pub async fn create_bot(song_list: SongList) {
    let _ = dotenv();
    let env_token =
        env::var("DISCORD_TOKEN").expect("Environment variable DISCORD_TOKEN not found");
    let force_server = env::var("FORCE_SERVER")
        .ok()
        .map(|s| s.parse().ok())
        .flatten();

    println!("Received discord token {}", env_token);
    println!(
        "Received force server {}",
        force_server
            .map(|f: u64| f.to_string())
            .unwrap_or_else(|| "-".to_string())
    );

    let handler = Handler::new(song_list, force_server);
    let mut client = Client::builder(env_token)
        .event_handler(handler)
        .framework(StandardFramework::new().configure(|c| c.prefix("top2000-")))
        .await
        .expect("error creating bot");

    client
        .data
        .write()
        .await
        .insert::<VoiceManager>(Arc::clone(&client.voice_manager));

    let _ = client
        .start()
        .await
        .map_err(|why| println!("Client ended: {:?}", why));
}
