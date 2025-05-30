use std::fmt::Display;

use jiff::civil::Date;
use reqwest::StatusCode;
use serde::{
    Deserialize,
    de::{DeserializeOwned, IntoDeserializer},
};
use thiserror::Error;

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

#[derive(Deserialize, Debug)]
pub struct TmdbApiError {
    #[serde(rename = "status_code")]
    pub code: i32,
    #[serde(rename = "status_message")]
    pub message: String,
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("failed to connect")]
    Connect(#[source] reqwest::Error),
    #[error("failed to parse response")]
    Parsing(#[source] reqwest::Error),
    #[error("unknown http error")]
    UnknownHttp(#[source] reqwest::Error),
    #[error("resource not found")]
    NotFound,
    #[error("unknown api error")]
    Unknown(TmdbApiError),
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
    ) -> Result<ListResponse<SearchResultEntry>, ApiError> {
        // TODO: handle person results
        Self::json_request(
            self.client
                .get(format!("{}/search/multi", Self::BASE_URL))
                .query(&[("query", query)])
                .bearer_auth(self.api_key.to_string()),
        )
        .await
    }

    pub async fn fetch_config(&self) -> Result<Config, ApiError> {
        Self::json_request(
            self.client
                .get(format!("{}/configuration", Self::BASE_URL))
                .bearer_auth(self.api_key.to_string()),
        )
        .await
    }

    pub async fn fetch_full_movie(&self, movie_id: &TmdbId) -> Result<FullMovie, ApiError> {
        Self::json_request(
            self.client
                .get(format!("{}/movie/{}", Self::BASE_URL, movie_id.0))
                .bearer_auth(self.api_key.to_string()),
        )
        .await
    }

    pub async fn fetch_full_show(&self, show_id: &TmdbId) -> Result<FullShow, ApiError> {
        Self::json_request(
            self.client
                .get(format!("{}/tv/{}", Self::BASE_URL, show_id.0))
                .bearer_auth(self.api_key.to_string()),
        )
        .await
    }

    pub async fn fetch_movie_images(&self, movie_id: &TmdbId) -> Result<Images, ApiError> {
        Self::json_request(
            self.client
                .get(format!("{}/movie/{}/images", Self::BASE_URL, movie_id.0))
                .bearer_auth(self.api_key.to_string()),
        )
        .await
    }

    async fn json_request<T: DeserializeOwned>(
        req: reqwest::RequestBuilder,
    ) -> Result<T, ApiError> {
        let res = req.send().await.map_err(map_reqwest_error)?;

        let status = res.status();

        if !status.is_success() {
            return match status {
                StatusCode::NOT_FOUND => Err(ApiError::NotFound),
                _ => {
                    let api_error = res.json().await.map_err(map_reqwest_error);
                    match api_error {
                        Ok(err) => Err(ApiError::Unknown(err)),
                        Err(err) => Err(err),
                    }
                }
            };
        }

        res.json().await.map_err(map_reqwest_error)
    }
}

fn map_reqwest_error(err: reqwest::Error) -> ApiError {
    if err.is_connect() {
        return ApiError::Connect(err);
    }
    if err.is_decode() {
        return ApiError::Parsing(err);
    }

    ApiError::UnknownHttp(err)
}

pub fn build_image_url(base_url: &str, size: &ImageSize, image_path: &str) -> String {
    format!("{}{}{}", base_url, size.0, image_path)
}
