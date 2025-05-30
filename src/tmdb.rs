use std::fmt::Display;

use jiff::civil::Date;
use serde::{Deserialize, de::IntoDeserializer};

pub struct TmdbApi {
    api_key: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
pub struct ImageSize(String);

#[derive(Deserialize, Debug, Clone)]
pub struct TmdbId(pub i32);

impl Display for TmdbId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Deserialize)]
pub struct ImagesConfig {
    pub secure_base_url: String,
    pub poster_sizes: Vec<ImageSize>,
}

#[derive(Deserialize)]
pub struct Config {
    pub images: ImagesConfig,
}

#[derive(Deserialize, Debug)]
pub struct Image {
    #[serde(rename = "iso_639_1")]
    pub lang: Option<String>,
    pub file_path: String,
}

#[derive(Deserialize, Debug)]
pub struct Images {
    pub backdrops: Vec<Image>,
    pub posters: Vec<Image>,
}

#[derive(Deserialize, Debug)]
pub struct ListResponse<T> {
    pub page: i32,
    pub results: Vec<T>,
    pub total_pages: i32,
    pub total_results: i32,
}

#[derive(Deserialize, Debug)]
pub struct SearchResultEntry {
    pub id: TmdbId,
    #[serde(alias = "name")]
    pub title: String,
    #[serde(alias = "original_name")]
    pub original_title: String,
    pub overview: String,
    pub poster_path: Option<String>,
    pub media_type: String,
    pub original_language: String,
    #[serde(deserialize_with = "empty_string_as_none", alias = "first_air_date")]
    pub release_date: Option<Date>,
}

#[derive(Deserialize, Debug)]
pub struct FullMovie {
    pub id: TmdbId,
    pub original_title: String,
    pub original_language: String,
    pub title: String,
    pub overview: String,
    pub tagline: String,
    pub status: String,
    #[serde(deserialize_with = "empty_string_as_none")]
    pub release_date: Option<Date>,
    pub runtime: i32,
    pub imdb_id: String,
    pub images: Option<Images>,
    // TODO: fetch collections
}

#[derive(Deserialize, Debug)]
pub struct Season {
    pub id: TmdbId,
    pub season_number: i32,
    pub episode_count: i32,
    pub name: String,
    pub overview: String,
}

#[derive(Deserialize, Debug)]
pub struct FullShow {
    pub id: TmdbId,
    #[serde(rename = "original_name")]
    pub original_title: String,
    pub original_language: String,
    #[serde(rename = "name")]
    pub title: String,
    pub overview: String,
    pub tagline: String,
    pub status: String,
    #[serde(deserialize_with = "empty_string_as_none", rename = "first_air_date")]
    pub release_date: Option<Date>,
    #[serde(rename = "episode_run_time")]
    pub episode_runtimes: Vec<i32>,
    pub number_of_seasons: i32,
    pub number_of_episodes: i32,
    pub seasons: Vec<Season>,
    pub images: Option<Images>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    let opt = Option::<String>::deserialize(de)?;
    let opt = opt.as_ref().map(String::as_str);
    match opt {
        None | Some("") => Ok(None),
        Some(s) => T::deserialize(s.into_deserializer()).map(Some),
    }
}

impl TmdbApi {
    const BASE_URL: &'static str = "https://api.themoviedb.org/3";

    pub fn new(api_key: &str) -> TmdbApi {
        let client = reqwest::Client::new();
        TmdbApi {
            api_key: api_key.to_string(),
            client,
        }
    }

    pub async fn multi_search(
        &self,
        query: &str,
    ) -> anyhow::Result<ListResponse<SearchResultEntry>> {
        // TODO: handle person results
        let response: ListResponse<SearchResultEntry> = self
            .client
            .get(format!("{}/search/multi", Self::BASE_URL))
            .query(&[("query", query)])
            .bearer_auth(self.api_key.to_string())
            .send()
            .await?
            .json()
            .await?;
        Ok(response)
    }

    pub async fn fetch_config(&self) -> anyhow::Result<Config> {
        let config: Config = self
            .client
            .get(format!("{}/configuration", Self::BASE_URL))
            .bearer_auth(self.api_key.to_string())
            .send()
            .await?
            .json()
            .await?;
        Ok(config)
    }

    pub async fn fetch_full_movie(&self, movie_id: &TmdbId) -> anyhow::Result<FullMovie> {
        let res: FullMovie = self
            .client
            .get(format!("{}/movie/{}", Self::BASE_URL, movie_id.0))
            .bearer_auth(self.api_key.to_string())
            .send()
            .await?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn fetch_full_show(&self, show_id: &TmdbId) -> anyhow::Result<FullShow> {
        let res: FullShow = self
            .client
            .get(format!("{}/tv/{}", Self::BASE_URL, show_id.0))
            .bearer_auth(self.api_key.to_string())
            .send()
            .await?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn fetch_movie_images(&self, movie_id: &TmdbId) -> anyhow::Result<Images> {
        let images: Images = self
            .client
            .get(format!("{}/movie/{}/images", Self::BASE_URL, movie_id.0))
            .bearer_auth(self.api_key.to_string())
            .send()
            .await?
            .json()
            .await?;

        Ok(images)
    }
}

pub fn build_image_url(base_url: &str, size: &ImageSize, image_path: &str) -> String {
    format!("{}{}{}", base_url, size.0, image_path)
}
