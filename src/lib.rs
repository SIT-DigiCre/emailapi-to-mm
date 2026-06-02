use std::collections::HashMap;

use base64::{Engine as _, prelude::BASE64_STANDARD};
use ctutils::CtEq as _;
use hmac::{Hmac, KeyInit, Mac as _};
use indoc::formatdoc;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use worker::*;

#[derive(Deserialize)]
struct ResendEvent {
    #[serde(rename = "type")]
    event: String,
    data: ResendEventData,
}
#[derive(Deserialize)]
struct ResendEventData {
    from: String,
    to: Vec<String>,
    subject: String,
}

#[derive(Serialize)]
struct MmEvent {
    username: String,
    text: String,
}
impl MmEvent {
    fn new(username: String, ev: &ResendEventData) -> Self {
        Self {
            username,
            text: formatdoc! {"
                From: `{from}`
                To: `{to}`
                Subject: `{subject}`
            ", from = ev.from, to = ev.to.join(", "), subject = ev.subject},
        }
    }
    async fn send(&self, url: &str) -> Result<()> {
        let body = Some(serde_json::to_string(self)?.into());
        let mm_init = RequestInit {
            method: Method::Post,
            headers: Headers::from_iter([("Content-Type", "application/json")]),
            body,
            ..Default::default()
        };
        Fetch::Request(Request::new_with_init(url, &mm_init)?)
            .send()
            .await?;

        Ok(())
    }
}

fn verify_signature(
    body: &[u8],
    secret: &str,
    id: &str,
    timestamp: &str,
    signatures: &str,
) -> bool {
    let secret = secret.strip_prefix("whsec_").unwrap_or(secret);
    let Ok(secret_bytes) = BASE64_STANDARD.decode(secret) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&secret_bytes) else {
        return false;
    };
    mac.update(id.as_bytes());
    mac.update(b".");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    let mac = mac.finalize();

    signatures.split(' ').any(|sig| {
        if let Some(sig_payload) = sig.strip_prefix("v1,")
            && let Some(sig_bytes) = BASE64_STANDARD.decode(sig_payload).ok()
        {
            mac.as_bytes().ct_eq(&sig_bytes).into()
        } else {
            false
        }
    })
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    if req.method() != Method::Post {
        return Response::error("", 405);
    }
    // にんしょ〜

    let secret = env.secret("WEBHOOK_SECRET")?.to_string();

    let mut req = req;
    let body = req.bytes().await?;
    let headers = req.headers();
    let svix_headers = ["svix-id", "svix-timestamp", "svix-signature"]
        .into_iter()
        .filter_map(|key| {
            // Result→Optionと内側のOptionで、2回剥がす
            let val = headers.get(key).ok()??;
            Some((key.into(), val))
        })
        .collect::<HashMap<String, String>>();
    if svix_headers.len() != 3 {
        return Response::error("Missing Headers", 400);
    }
    if !verify_signature(
        &body,
        &secret,
        &svix_headers["svix-id"],
        &svix_headers["svix-timestamp"],
        &svix_headers["svix-signature"],
    ) {
        return Response::error("", 401);
    }
    // 認証 :done:

    let req_data = match serde_json::from_slice::<ResendEvent>(&body) {
        Ok(v) => v,
        Err(e) => {
            console_error!("Parse: {e:?}");
            return Response::error(format!("Parse Error: {:?}", e), 406);
        }
    };

    let mm_webhook_url = env.secret("MATTERMOST_WEBHOOK_URL")?.to_string();
    let username = env
        .var("MATTERMOST_USERNAME")
        .map_or_else(|_| "メール送信お知らせくん 改".into(), |s| s.to_string());

    if req_data.event == "email.sent" {
        let username = username.clone();
        let mm_webhook_url = mm_webhook_url.clone();
        let mm_ev = MmEvent::new(username, &req_data.data);
        let sender = mm_ev.send(&mm_webhook_url);

        sender
            .await
            .unwrap_or_else(|e| console_error!("Failed to send notice to MM: {:?}", e))
    }

    Response::ok("")
}
