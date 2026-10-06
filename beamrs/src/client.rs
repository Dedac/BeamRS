#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
use serde::{de::DeserializeOwned, Serialize};

use crate::domain::{Frequency, PrismResult, Ray, User};
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
use crate::domain::{FrequencyInput, PrismInput, RayInput, UsernameInput};

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
const STORAGE_KEY: &str = "beamrs.identity";

#[cfg(any(all(feature = "hydrate", target_arch = "wasm32"), test))]
#[derive(Debug, serde::Deserialize)]
struct ErrorBody {
    error: String,
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
async fn parse_response<T: DeserializeOwned>(
    response: gloo_net::http::Response,
) -> Result<T, String> {
    if response.ok() {
        return response
            .json::<T>()
            .await
            .map_err(|error| format!("invalid server response: {error}"));
    }

    let status = response.status();
    let message = response
        .json::<ErrorBody>()
        .await
        .map(|body| body.error)
        .unwrap_or_else(|_| format!("request failed with status {status}"));
    Err(message)
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
async fn get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let response = gloo_net::http::Request::get(path)
        .send()
        .await
        .map_err(|error| format!("network request failed: {error}"))?;
    parse_response(response).await
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
async fn send<B: Serialize + ?Sized, T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: &B,
) -> Result<T, String> {
    let request = match method {
        "POST" => gloo_net::http::Request::post(path),
        "PATCH" => gloo_net::http::Request::patch(path),
        _ => return Err(format!("unsupported client method: {method}")),
    };
    let response = request
        .json(body)
        .map_err(|error| format!("could not encode request: {error}"))?
        .send()
        .await
        .map_err(|error| format!("network request failed: {error}"))?;
    parse_response(response).await
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
async fn delete<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let response = gloo_net::http::Request::delete(path)
        .send()
        .await
        .map_err(|error| format!("network request failed: {error}"))?;
    parse_response(response).await
}

#[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
async fn unavailable<T>() -> Result<T, String> {
    Err("browser API is unavailable during server rendering".to_string())
}

pub async fn list_frequencies() -> Result<Vec<Frequency>, String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    return get("/api/frequencies").await;
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    unavailable().await
}

pub async fn create_frequency(name: String) -> Result<Frequency, String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    return send("POST", "/api/frequencies", &FrequencyInput { name }).await;
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    {
        let _ = name;
        unavailable().await
    }
}

pub async fn list_frequency_rays(frequency_id: i32) -> Result<Vec<Ray>, String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    return get(&format!("/api/frequencies/{frequency_id}/rays")).await;
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    {
        let _ = frequency_id;
        unavailable().await
    }
}

pub async fn list_authored_rays(username: &str) -> Result<Vec<Ray>, String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    return get(&format!(
        "/api/users/{}/rays",
        js_sys::encode_uri_component(username)
    ))
    .await;
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    {
        let _ = username;
        unavailable().await
    }
}

pub async fn list_prismed_rays(username: &str) -> Result<Vec<Ray>, String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    return get(&format!(
        "/api/users/{}/prisms",
        js_sys::encode_uri_component(username)
    ))
    .await;
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    {
        let _ = username;
        unavailable().await
    }
}

pub async fn create_ray(frequency_id: i32, user_id: i32, text: String) -> Result<Ray, String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    return send(
        "POST",
        "/api/rays",
        &RayInput {
            frequency_id,
            user_id,
            text,
        },
    )
    .await;
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    {
        let _ = (frequency_id, user_id, text);
        unavailable().await
    }
}

pub async fn set_prism(user_id: i32, ray_id: i32, prismed: bool) -> Result<PrismResult, String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        if prismed {
            send("POST", "/api/prisms", &PrismInput { user_id, ray_id }).await
        } else {
            delete(&format!("/api/prisms/{user_id}/{ray_id}")).await
        }
    }
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    {
        let _ = (user_id, ray_id, prismed);
        unavailable().await
    }
}

pub async fn update_username(user_id: i32, username: String) -> Result<User, String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    return send(
        "PATCH",
        &format!("/api/users/{user_id}"),
        &UsernameInput { username },
    )
    .await;
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    {
        let _ = (user_id, username);
        unavailable().await
    }
}

pub async fn load_identity() -> Result<User, String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        let storage = web_sys::window()
            .ok_or_else(|| "browser window is unavailable".to_string())?
            .local_storage()
            .map_err(|_| "could not access browser storage".to_string())?
            .ok_or_else(|| "browser storage is unavailable".to_string())?;
        let saved = storage
            .get_item(STORAGE_KEY)
            .map_err(|_| "could not read browser identity".to_string())?
            .and_then(|value| serde_json::from_str::<User>(&value).ok());
        let username = saved
            .map(|user| user.username)
            .unwrap_or_else(random_anonymous_name);
        let user: User = send("POST", "/api/users", &UsernameInput { username }).await?;
        store_identity(&user)?;
        Ok(user)
    }
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    unavailable().await
}

pub fn store_identity(user: &User) -> Result<(), String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        let storage = web_sys::window()
            .ok_or_else(|| "browser window is unavailable".to_string())?
            .local_storage()
            .map_err(|_| "could not access browser storage".to_string())?
            .ok_or_else(|| "browser storage is unavailable".to_string())?;
        let value = serde_json::to_string(user)
            .map_err(|error| format!("could not encode browser identity: {error}"))?;
        storage
            .set_item(STORAGE_KEY, &value)
            .map_err(|_| "could not persist browser identity".to_string())
    }
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    {
        let _ = user;
        Err("browser storage is unavailable during server rendering".to_string())
    }
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn random_anonymous_name() -> String {
    let suffix = (js_sys::Math::random() * 900_000.0) as u32 + 100_000;
    format!("Anon{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_body_matches_server_contract() {
        let body: ErrorBody = serde_json::from_str(r#"{"error":"bad wavelength"}"#).unwrap();
        assert_eq!(body.error, "bad wavelength");
    }
}
