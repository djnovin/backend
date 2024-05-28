mod emails;
use emails::{fetch_inbox, summarize_email};

use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde_json::{json, Value};
use std::env;

async fn handler(_: LambdaEvent<Value>) -> Result<Value, Error> {
    let username = env::var("EMAIL_USERNAME")?;
    let password = env::var("EMAIL_PASSWORD")?;
    let api_key = env::var("OPENAI_API_KEY")?;

    let result = match fetch_inbox(&username, &password).unwrap() {
        Some(email) => {
            let summary = summarize_email(&email, &api_key).await.unwrap();
            json!({
                "status": "success",
                "summary": summary
            })
        }
        None => json!({
            "status": "no emails found"
        }),
    };

    Ok(result)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = service_fn(handler);
    lambda_runtime::run(func).await?;
    Ok(())
}
