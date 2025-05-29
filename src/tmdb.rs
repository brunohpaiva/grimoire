use jiff::civil::Date;
use serde::{Deserialize, Deserializer, de::IntoDeserializer};

pub struct TmdbApi {
    api_key: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
pub struct ImageSize(String);

#[derive(Deserialize, Debug)]
pub struct TmdbId(i32);

pub struct MovieId(i32);

pub struct ShowId(i32);

#[derive(Deserialize)]
pub struct ImagesConfig {
    pub secure_base_url: String,
    pub poster_sizes: Vec<ImageSize>,
}

#[derive(Deserialize)]
pub struct Config {
    pub images: ImagesConfig,
}

#[derive(Deserialize)]
pub struct Image {
    #[serde(rename = "iso_639_1")]
    pub lang: Option<String>,
    pub file_path: String,
}

#[derive(Deserialize)]
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

    pub async fn fetch_movie_images(&self, movie_id: MovieId) -> anyhow::Result<Images> {
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
