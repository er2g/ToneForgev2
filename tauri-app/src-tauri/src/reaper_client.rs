// src-tauri/src/reaper_client.rs
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;

fn normalize_param_token(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

#[derive(Debug, Clone)]
pub struct ReaperClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackFXInfo {
    pub index: i32,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackInfo {
    pub index: i32,
    pub name: String,
    pub fx_count: i32,
    pub fx_list: Vec<TrackFXInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackListResponse {
    pub track_count: i32,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FXParamEntry {
    pub index: i32,
    pub name: String,
    pub value: f64,
    pub display: String,
    pub unit: String,
    pub format_hint: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FXParamSnapshot {
    pub track: i32,
    pub fx: i32,
    pub params: Vec<FXParamEntry>,
}

impl ReaperClient {
    pub fn new() -> Self {
        Self {
            base_url: "http://127.0.0.1:8888".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Health check - REAPER extension çalışıyor mu?
    pub async fn ping(&self) -> Result<bool, Box<dyn Error>> {
        let response = self
            .client
            .get(&format!("{}/ping", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// Track ve FX listesini al
    pub async fn get_tracks(&self) -> Result<TrackListResponse, Box<dyn Error>> {
        let response = self
            .client
            .get(&format!("{}/tracks", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to get tracks: {}", response.status()).into());
        }

        let tracks: TrackListResponse = response.json().await?;
        Ok(tracks)
    }

    /// FX parametresini ayarla
    pub async fn set_param(
        &self,
        track: i32,
        fx: i32,
        param: &str,
        value: f64,
    ) -> Result<(), Box<dyn Error>> {
        let response = self
            .client
            .post(&format!("{}/fx/param", self.base_url))
            .json(&json!({
                "track": track,
                "fx": fx,
                "param": param,
                "value": value
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to set parameter: {}", error_text).into());
        }

        Ok(())
    }

    /// FX parametresini oku
    pub async fn get_param(&self, track: i32, fx: i32, param: &str) -> Result<f64, Box<dyn Error>> {
        let response = self
            .client
            .get(&format!("{}/fx/param", self.base_url))
            .query(&[("track", track), ("fx", fx)])
            .query(&[("param", param)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err("Failed to get parameter".into());
        }

        let json: serde_json::Value = response.json().await?;
        let value = json["value"].as_f64().ok_or("Invalid value")?;
        Ok(value)
    }

    /// Plugin ekle
    pub async fn add_plugin(&self, track: i32, plugin_name: &str) -> Result<i32, Box<dyn Error>> {
        let response = self
            .client
            .post(&format!("{}/fx/add", self.base_url))
            .json(&json!({
                "track": track,
                "plugin": plugin_name
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to add plugin: {}", error_text).into());
        }

        let json: serde_json::Value = response.json().await?;
        let fx_index_i64 = json["fx_index"].as_i64().ok_or("Invalid response")?;

        // Safely convert i64 to i32 with bounds checking
        let fx_index = i32::try_from(fx_index_i64)
            .map_err(|_| format!("FX index {} out of i32 range", fx_index_i64))?;

        Ok(fx_index)
    }

    /// Plugin sil
    pub async fn remove_plugin(&self, track: i32, fx: i32) -> Result<(), Box<dyn Error>> {
        let response = self
            .client
            .delete(&format!("{}/fx/remove", self.base_url))
            .query(&[("track", track), ("fx", fx)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err("Failed to remove plugin".into());
        }

        Ok(())
    }

    /// FX bypass durumunu ayarla
    pub async fn set_fx_enabled(
        &self,
        track: i32,
        fx: i32,
        enabled: bool,
    ) -> Result<bool, Box<dyn Error>> {
        let response = self
            .client
            .post(&format!("{}/fx/toggle", self.base_url))
            .json(&json!({
                "track": track,
                "fx": fx,
                "enabled": enabled
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to toggle FX: {}", error_text).into());
        }

        let json: serde_json::Value = response.json().await?;
        let current = json["enabled"].as_bool().unwrap_or(enabled);
        Ok(current)
    }

    /// BPM ayarla
    pub async fn set_bpm(&self, bpm: f64) -> Result<(), Box<dyn Error>> {
        let response = self
            .client
            .post(&format!("{}/transport/bpm", self.base_url))
            .json(&json!({"bpm": bpm}))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err("Failed to set BPM".into());
        }

        Ok(())
    }

    /// BPM oku
    pub async fn get_bpm(&self) -> Result<f64, Box<dyn Error>> {
        let response = self
            .client
            .get(&format!("{}/transport/bpm", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err("Failed to get BPM".into());
        }

        let json: serde_json::Value = response.json().await?;
        let bpm = json["bpm"].as_f64().ok_or("Invalid BPM")?;
        Ok(bpm)
    }

    /// Proje kaydet
    pub async fn save_project(&self, preset_name: &str) -> Result<String, Box<dyn Error>> {
        let response = self
            .client
            .post(&format!("{}/project/save", self.base_url))
            .json(&json!({"name": preset_name}))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err("Failed to save project".into());
        }

        let json: serde_json::Value = response.json().await?;
        let path = json["project_path"]
            .as_str()
            .ok_or("Invalid response")?
            .to_string();
        Ok(path)
    }

    /// Proje yükle
    pub async fn load_project(&self, project_path: &str) -> Result<(), Box<dyn Error>> {
        let response = self
            .client
            .post(&format!("{}/project/load", self.base_url))
            .json(&json!({"path": project_path}))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to load project: {}", error_text).into());
        }

        Ok(())
    }

    /// Parametre snapshot al
    pub async fn get_fx_params(
        &self,
        track: i32,
        fx: i32,
    ) -> Result<FXParamSnapshot, Box<dyn Error>> {
        let response = self
            .client
            .get(&format!("{}/fx/params", self.base_url))
            .query(&[("track", track), ("fx", fx)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to get FX params: {}", response.status()).into());
        }

        let snapshot: FXParamSnapshot = response.json().await?;
        Ok(snapshot)
    }

    pub fn find_param_entry<'a>(
        &self,
        params: &'a [FXParamEntry],
        query: &str,
    ) -> Option<&'a FXParamEntry> {
        let normalized_query = normalize_param_token(query);
        params
            .iter()
            .find(|entry| normalize_param_token(&entry.name).contains(&normalized_query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ping() {
        let client = ReaperClient::new();
        // Bu test sadece REAPER extension çalışıyorsa geçer
        let result = client.ping().await;
        println!("Ping result: {:?}", result);
    }
}
