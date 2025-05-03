use reqwest::{Client, Url};
use anyhow::Result;
use rustls::{ClientConfig, RootCertStore};
use serde::de::DeserializeOwned;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub data: Option<T>, // 使用 Option<T> 以处理 data 可能为空的情况
    pub message: Option<String>, // 假设可能还有 message 字段
}

pub struct Api {
    client: Client,
    base_url: Url,
}

impl Api {
    pub fn new(base_url: &str) -> Result<Self> {
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let client = Client::builder().use_preconfigured_tls(config).build()?;
        let base_url = Url::parse(base_url)?;
        Ok(Self { client, base_url })
    }
    
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<ApiResponse<T>> {
        let url = self.base_url.join(path)?;
        let response = self.client.get(url).send().await?;
        let api_response = response.json::<ApiResponse<T>>().await?;
        Ok(api_response)
    }
    
    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: B) -> Result<ApiResponse<T>> {
        let url = self.base_url.join(path)?;
        let response = self.client.post(url).json(&body).send().await?;
        let api_response = response.json::<ApiResponse<T>>().await?;
        Ok(api_response)
    }
}