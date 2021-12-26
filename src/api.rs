use crate::error::ErrorKind;
use serde::{/*de,*/ Deserialize, /*Deserializer**/};
use serde_json::Value;

#[derive(Debug, Deserialize, Clone)]
pub struct Song {
    #[serde(rename = "id")]
    pub id: Option<String>,

    #[serde(rename = "title")]
    pub title: String,

    #[serde(rename = "artist")]
    pub artist: String,

    #[serde(rename = "position")]
    pub position: Option<u64>,

    #[serde(rename = "lastPosition")]
    pub prev_position: Option<u64>,

    #[serde(rename = "trackHistoryUrl")]
    pub url: String,

    #[serde(rename = "imageUrl")]
    pub image: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SongList {
    songs: Vec<Song>,
}

#[derive(Debug, Clone)]
pub struct NowOnAir {
    pub song: Song,
    pub img_url: Option<String>,
}

// fn to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     let s: String = Deserialize::deserialize(deserializer)?;
//     s.parse().map_err(de::Error::custom)
// }

impl Song {
    pub async fn get_description(&self) -> Result<String, ErrorKind> {
        // let url = format!(
        //     "https://www.nporadio2.nl/?option=com_ajax&plugin=Trackdata&format=json&songid={}",
        //     self.id
        // );
        // let body = reqwest::get(&url)
        //     .await
        //     .map_err(ErrorKind::RequestError)?
        //     .text()
        //     .await
        //     .map_err(ErrorKind::RequestError)?;

        // let desc_unparsed = &serde_json::from_str::<Value>(&body).map_err(ErrorKind::JsonError)?
        //     ["data"][0]["description"];
        // if let Value::String(desc) = desc_unparsed {
        //     Ok(desc.to_owned())
        // } else {
        //     Err(ErrorKind::GenericError)
        // }

        // TODO fix return value
        Ok("".to_owned())
    }

    pub fn get_last_year_position(&self) -> Option<u64> {
        self.prev_position
    }
}

impl SongList {
    pub fn new() -> Result<SongList, ErrorKind> {
        let body = include_str!("2021.json");
        let unparsed_songs =
            &serde_json::from_str::<Value>(&body).map_err(ErrorKind::JsonError)?;
        let songs: Vec<_> =
            serde_json::from_value(unparsed_songs["positions"].to_owned()).map_err(ErrorKind::JsonError)?;

        println!("Successfully parsed {} songs!", &songs.len());

        Ok(SongList { songs })
    }

    pub fn get_song(&self, position: usize) -> Option<&Song> {
        self.songs.get(position)
    }

    pub async fn get_now_on_air(&self) -> Result<NowOnAir, ErrorKind> {
        let body =
            reqwest::get("https://www.nporadio2.nl/api/miniplayer/info?channel=npo-radio-2").await
                .map_err(ErrorKind::RequestError)?
                .text().await
                .map_err(ErrorKind::RequestError)?;

        let parsed_json = serde_json::from_str::<Value>(&body).map_err(ErrorKind::JsonError)?;
        let now_playing = &parsed_json["data"]["radio_track_plays"]["data"][0];
        // let id_unparsed = &parsed_json["data"][0]["id"];
        // if let Value::String(id) = id_unparsed {
        //     if let Ok(id_unwrapped) = id.parse::<u64>() {
        //         let song_option = self.songs.iter().find(|s| s.id == id_unwrapped);
        //         if let Some(song) = song_option {
        //             let img_url_unparsed = &now_playing["radio_tracks"]["cover_url"];
        //             let img_url = if let Value::String(img) = img_url_unparsed {
        //                 Some(img.to_string())
        //             } else {
        //                 None
        //             };

        //             return Ok(NowOnAir {
        //                 song: song.clone(),
        //                 img_url,
        //             });
        //         }
        //     }
        // }

        println!("Songs found: {}", &parsed_json["data"]["radio_track_plays"]["data"]);

        let artist_val = &now_playing["artist"];
        let title_val = &now_playing["song"];

        if let Value::String(artist) = artist_val {
            if let Value::String(title) = title_val {
                let song = self
                    .songs
                    .iter()
                    .find(|s| s.artist.to_lowercase() == *artist.to_lowercase() && s.title.to_lowercase() == *title.to_lowercase());

                let img_url = if true {
                    Some("https://radio-images.npo.nl/{format}/34e8df87-0f33-45c6-bd1a-f2528ff87626/f13f6e02-d9b0-407c-824b-f1fb542c3ec4.jpg".to_string())
                } else {
                    None
                };

                let default_id = if false {
                    Some("".to_string())
                } else {
                    None
                };

                return match song {
                    Some(song_some) => Ok(NowOnAir {
                        song: song_some.to_owned(),
                        img_url: song_some.image.clone(),
                    }),
                    None => Ok(NowOnAir {
                        song: Song {
                            id: default_id,
                            title: title.to_string(),
                            artist: artist.to_string(),
                            position: None,
                            url: "".to_string(),
                            image: None,
                            prev_position: None,
                        },
                        img_url: img_url,
                    }),
                };
            }
        }

        Err(ErrorKind::GenericError)
    }
}
