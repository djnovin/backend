#[macro_use]
extern crate lazy_static;
use axum::{routing::get, Json, Router};
use chrono::{Duration, Utc};
use dotenv::dotenv;
use imap;
use native_tls::{Identity, TlsConnector};
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref CERTIFICATE: Arc<Mutex<Option<Identity>>> = Arc::new(Mutex::new(None));
}

#[derive(serde::Serialize)]
struct Value {
    value: String,
}

async fn create_summary() -> Result<(StatusCode, Json<Value>), reqwest::Error> {
    dotenv().ok();

    // IMAP CLIENT SETUP
    let imap_server = "imappro.zoho.com.au";
    let imap_port = 993;

    let tls_connector = TlsConnector::builder().build().unwrap();

    let client = imap::connect((imap_server, imap_port), imap_server, &tls_connector).unwrap();

    // LOGIN TO EMAIL CLIENT
    let email = "admin@novinnoori.com";
    let password = "mXL@Xyy4S%Q5gh";

    let mut imap_session = client
        .login(email, password)
        .map_err(|(err, _)| err)
        .unwrap();

    // FETCH EMAILS
    imap_session.select("INBOX").unwrap();

    let twenty_four_hours_ago = (Utc::now() - Duration::hours(12))
        .format("%d-%b-%Y")
        .to_string();

    let search_query = format!("SINCE {}", twenty_four_hours_ago);
    let message_ids = imap_session.search(search_query).unwrap();

    let mut email_contents = Vec::new();
    for message_id in message_ids.iter() {
        let messages = imap_session
            .fetch(message_id.to_string(), "RFC822")
            .unwrap();
        for message in messages.iter() {
            if let Some(body) = message.body() {
                let email_content = std::str::from_utf8(body).unwrap();
                email_contents.push(email_content.to_string());
            } else {
                println!("Message didn't have a body!");
            }
        }
    }

    let batch_size = 1;
    let mut batched_email_contents = Vec::new();

    for chunk in email_contents.chunks(batch_size) {
        let email_chunk = chunk.join("\n\n");

        // OPENAI API REQUEST
        let openai_api_url = "https://api.openai.com/v1/chat/completions";
        let api_key = "sk-proj-d8ZoI7fCoiz6Kw0eret4T3BlbkFJ7B7d5LdvLLvQkDfWeuZc";

        let ai_client = Client::new();

        let summary_prompt = format!("Please summarize the following email:\n\n{}", email_chunk);

        let prompt = json!({
            "model": "gpt-4",
            "messages": [{
                "role": "system",
                "content": "You are an AI assistant that helps summarize emails and create reports for users to review with suggestions of actions depending on how relevant the information is to the user's day to day tasks."
            }, {
                "role": "user",
                "content": summary_prompt
            }],
            "temperature": 0.7,
            "max_tokens": 1024,
            "top_p": 1
        });

        let response = ai_client
            .post(openai_api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&prompt)
            .send()
            .await?
            .json::<HashMap<String, serde_json::Value>>()
            .await?;

        println!("{:?}", response);

        if let Some(choices) = response.get("choices") {
            if let Some(choice) = choices.get(0) {
                if let Some(message) = choice.get("message") {
                    if let Some(content) = message.get("content") {
                        batched_email_contents.push(content.as_str().unwrap_or("").to_string());
                    }
                }
            }
        } else {
            println!("No summary found");
        }
    }

    imap_session.logout().unwrap();

    let summary = batched_email_contents.join("\n\n");

    match summary {
        summary if summary.is_empty() => Ok((
            StatusCode::OK,
            Json(Value {
                value: "No emails found".to_string(),
            }),
        )),
        _ => Ok((StatusCode::OK, Json(Value { value: summary }))),
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route(
            "/summary",
            get(|| async { create_summary().await.unwrap() }),
        )
        .route("/", get(|| async { "Hello, world!" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
