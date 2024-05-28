use std::collections::HashMap;

use native_tls::TlsConnector;
use reqwest::Client;

const IMAP_SERVER: &str = "imap.zoho.com";
const IMAP_PORT: u16 = 993;
const OPENAI_API_URL: &str = "https://api.openai.com/v1/engines/davinci-codex/completions";
const EMAIL_FIELD: &str = "RFC822";
const MAX_TOKENS: &str = "150";

pub fn fetch_inbox(
    username: &str,
    password: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let tls_connector = TlsConnector::builder().build().unwrap();
    let client = imap::connect((IMAP_SERVER, IMAP_PORT), IMAP_SERVER, &tls_connector)?;

    let mut imap_session = client.login(username, password).map_err(|e| e.0)?;

    imap_session.select("INBOX")?;
    let messages = imap_session.fetch("1", EMAIL_FIELD)?;
    let message = messages.iter().next();

    let body = match message {
        Some(m) => {
            let body = m.body().expect("Message did not have a body!");
            std::str::from_utf8(body)
                .expect("Message was not valid UTF-8")
                .to_string()
        }
        None => return Ok(None),
    };

    imap_session.logout()?;

    Ok(Some(body))
}

pub async fn summarize_email(
    email_content: &str,
    api_key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let prompt = create_summary_prompt(email_content);

    let response = client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&prompt)
        .send()
        .await?
        .json::<HashMap<String, Vec<HashMap<String, String>>>>()
        .await?;

    extract_summary(response)
}

fn create_summary_prompt(email_content: &str) -> HashMap<&str, String> {
    let mut prompt = HashMap::new();
    prompt.insert(
        "prompt",
        format!(
            "Please summarize the following email and return the details in a JSON format with the following keys:\n\n\
            {{\n\
            \"main_topics\": [\"...\"],\n\
            \"action_items\": [\"...\"],\n\
            \"deadlines\": [\"...\"]\n\
            }}\n\n\
            Email Content:\n{}",
            email_content
        ),
    );
    prompt.insert("max_tokens", MAX_TOKENS.to_string());
    prompt
}
fn extract_summary(
    response: HashMap<String, Vec<HashMap<String, String>>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let choices = response.get("choices").ok_or("No choices in response")?;
    let choice = choices.get(0).ok_or("No choice in choices")?;
    let summary = choice.get("text").ok_or("No text in choice")?;
    Ok(summary.clone())
}
