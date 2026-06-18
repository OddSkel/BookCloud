use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

fn keycloak_url() -> String {
    std::env::var("KEYCLOAK_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

fn keycloak_client_id() -> String {
    std::env::var("KEYCLOAK_CLIENT_ID").unwrap_or_else(|_| "bookcloud-app".to_string())
}

fn keycloak_client_secret() -> String {
    std::env::var("KEYCLOAK_CLIENT_SECRET")
        .or_else(|_| std::env::var("KEYCLOAK_ADMIN_SECRET"))
        .unwrap_or_else(|_| "bookcloud-app-secret".to_string())
}

pub async fn login(body: web::Json<LoginRequest>) -> HttpResponse {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/realms/bookcloud/protocol/openid-connect/token",
        keycloak_url()
    );
    let params = [
        ("client_id", keycloak_client_id()),
        ("client_secret", keycloak_client_secret()),
        ("grant_type", "password".to_string()),
        ("username", body.username.clone()),
        ("password", body.password.clone()),
    ];
    match client.post(&url).form(&params).send().await {
        Ok(res) => {
            if res.status().is_success() {
                let token: serde_json::Value = res.json().await.unwrap_or_default();
                HttpResponse::Ok().json(token)
            } else {
                HttpResponse::Unauthorized().json(json!({
                    "error": "invalid_credentials",
                    "status": res.status().as_u16()
                }))
            }
        }
        Err(_) => {
            HttpResponse::ServiceUnavailable().json(json!({ "error": "keycloak_unreachable" }))
        }
    }
}

pub async fn logout(body: web::Json<LogoutRequest>) -> HttpResponse {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/realms/bookcloud/protocol/openid-connect/logout",
        keycloak_url()
    );
    let params = [
        ("client_id", keycloak_client_id()),
        ("client_secret", keycloak_client_secret()),
        ("refresh_token", body.refresh_token.clone()),
    ];
    match client.post(&url).form(&params).send().await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(_) => {
            HttpResponse::ServiceUnavailable().json(json!({ "error": "keycloak_unreachable" }))
        }
    }
}

pub async fn register(body: web::Json<serde_json::Value>) -> HttpResponse {
    let client = reqwest::Client::new();

    // Passo 1: token de admin via client_credentials no realm bookcloud
    let token_url = format!(
        "{}/realms/bookcloud/protocol/openid-connect/token",
        keycloak_url()
    );
    let admin_client_id =
        std::env::var("KEYCLOAK_ADMIN_CLIENT_ID").unwrap_or_else(|_| "bookcloud-app".to_string());
    let admin_secret = std::env::var("KEYCLOAK_ADMIN_SECRET").unwrap_or_default();

    let token_params = [
        ("client_id", admin_client_id),
        ("client_secret", admin_secret),
        ("grant_type", "client_credentials".to_string()),
    ];

    let admin_token: serde_json::Value = match client
        .post(&token_url)
        .form(&token_params)
        .send()
        .await
        .and_then(|r| Ok(r))
    {
        Ok(res) => res.json().await.unwrap_or_default(),
        Err(_) => {
            return HttpResponse::ServiceUnavailable()
                .json(json!({ "error": "keycloak_unreachable" }));
        }
    };

    let access_token = match admin_token["access_token"].as_str() {
        Some(t) => t.to_string(),
        None => {
            return HttpResponse::InternalServerError()
                .json(json!({ "error": "admin_token_failed" }));
        }
    };

    // Passo 2: criar utilizador no realm bookcloud
    let users_url = format!("{}/admin/realms/bookcloud/users", keycloak_url());
    let register_body = body.into_inner();

    let username = register_body
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let email = register_body
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let password = register_body
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let keycloak_user = json!({
        "username": username,
        "email": email,
        "enabled": true,
        "credentials": [
            {
                "type": "password",
                "value": password,
                "temporary": false
            }
        ]
    });

    let res = client
        .post(&users_url)
        .bearer_auth(&access_token)
        .json(&keycloak_user)
        .send()
        .await;

    match res {
        Ok(r) => {
            if r.status().as_u16() == 201 {
                let user_lookup_url = format!(
                    "{}/admin/realms/bookcloud/users?username={}",
                    keycloak_url(),
                    username
                );

                let users: serde_json::Value = match client
                    .get(&user_lookup_url)
                    .bearer_auth(&access_token)
                    .send()
                    .await
                {
                    Ok(resp) => resp.json().await.unwrap_or_default(),
                    Err(_) => serde_json::Value::Null,
                };

                let user_id = match users
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|u| u.get("id"))
                    .and_then(|id| id.as_str())
                {
                    Some(id) => id,
                    None => {
                        return HttpResponse::InternalServerError()
                            .json(json!({ "error": "user_created_but_id_lookup_failed" }));
                    }
                };

                let role_url = format!("{}/admin/realms/bookcloud/roles/user", keycloak_url());

                let role: serde_json::Value = match client
                    .get(&role_url)
                    .bearer_auth(&access_token)
                    .send()
                    .await
                {
                    Ok(resp) => resp.json().await.unwrap_or_default(),
                    Err(_) => serde_json::Value::Null,
                };

                let role_id = match role.get("id").and_then(|id| id.as_str()) {
                    Some(id) => id,
                    None => {
                        return HttpResponse::InternalServerError()
                            .json(json!({ "error": "user_created_but_role_lookup_failed" }));
                    }
                };

                let role_mapping_url = format!(
                    "{}/admin/realms/bookcloud/users/{}/role-mappings/realm",
                    keycloak_url(),
                    user_id
                );

                let role_assign_resp = client
                    .post(&role_mapping_url)
                    .bearer_auth(&access_token)
                    .json(&json!([
                        {
                            "id": role_id,
                            "name": "user"
                        }
                    ]))
                    .send()
                    .await;

                match role_assign_resp {
                    Ok(resp) if resp.status().as_u16() == 204 => {
                        HttpResponse::Created().json(json!({
                            "message": "User created successfully",
                            "role": "user"
                        }))
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let err: serde_json::Value = resp.json().await.unwrap_or_default();
                        HttpResponse::build(
                            actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap()
                        ).json(json!({
                            "error": "user_created_but_role_assignment_failed",
                            "details": err
                        }))
                    }
                    Err(_) => HttpResponse::ServiceUnavailable()
                        .json(json!({ "error": "user_created_but_keycloak_unreachable_during_role_assignment" })),
                }
            } else {
                let status = r.status();
                let body: serde_json::Value = r.json().await.unwrap_or_default();
                HttpResponse::build(actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap())
                    .json(body)
            }
        }
        Err(_) => {
            HttpResponse::ServiceUnavailable().json(json!({ "error": "keycloak_unreachable" }))
        }
    }
}
