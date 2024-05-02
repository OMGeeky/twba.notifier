use actix_web::{get, post, web, App, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use twba_common::prelude::*;

#[derive(Clone)]
struct AppState {
    config: Conf,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct NotificationWebhookMessage {
    content: String,
}

#[get("/")]
#[instrument]
async fn index() -> impl Responder {
    "Hello, world!"
}

#[post("/notify")]
#[instrument(skip(data))]
async fn post_notify(req_body: String, data: web::Data<AppState>) -> impl Responder {
    match post_notify_inner(req_body, data).await {
        Ok(response) => response,
        Err(e) => e,
    }
}

async fn post_notify_inner(req_body: String, data: web::Data<AppState>) -> Result<String, String> {
    let req_body = serde_json::from_str::<twba_common::notify::NotificationRequest>(&req_body)
        .map_err(|e| format!("Could not parse request body: {e}"))
        .map(|req| NotificationWebhookMessage {
            content: req.message,
        })
        // .and_then(|msg| serde_json::to_string(&msg).map_err(|e| format!("Could not serialize message: {e}"))
        .map_err(|e| format!("Could not serialize message: {e}"))?;
    info!("req_body: {:?}", req_body);
    println!("req_body: {:?}", req_body);
    let webhook_url = &data.config.notifier.webhook_url;
    if let Some(webhook_url) = webhook_url {
        let req_body = NotificationWebhookMessage {
            content: req_body.content,
        };
        let response = notify_webhook(webhook_url, &req_body).await?;
        info!("response: {}", response);
        println!("response: {}", response);
        Ok(format!(
            "Sent something successfully. Return of webhook: '{}'",
            response
        ))
    } else {
        Err("No webhook URL configured and smtp not supported yet".to_string())
    }
}

async fn notify_webhook(
    webhook_url: &str,
    req_body: &NotificationWebhookMessage,
) -> Result<String, String> {
    reqwest::Client::new()
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(req_body).map_err(|e| {
            format!("Could not notify webhook (error parsing message to json): {e}")
        })?)
        .send()
        .await
        .map_err(|e| format!("Could not notify webhook: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Could not notify webhook (text response): {e}"))
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _guard = init_tracing("twba_common");
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(AppState {
                config: get_config(),
            }))
            .service(index)
            .service(post_notify)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
