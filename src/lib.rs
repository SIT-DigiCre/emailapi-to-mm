use base64::prelude::*;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Deserialize)]
struct SendGridEvent {
    email: String,
    event: String,
}

#[derive(Serialize)]
struct MmEvent {
    username: String,
    text: String,
}
impl MmEvent {
    fn new(username: String, email: &str) -> Self {
        Self {
            username,
            text: format!("`{}`へのメール配信処理が実行されました。", email),
        }
    }
    async fn send(&self, url: &str) -> Result<()> {
        let body = Some(serde_json::to_string(&self)?.into());
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

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // にんしょ〜
    let expected_auth = format!(
        "Basic {}",
        BASE64_STANDARD.encode(env.secret("WORKER_AUTH")?.to_string())
    );
    if req.headers().get("Authorization")? != Some(expected_auth) {
        return Response::error("", 401);
    }
    // 認証 :done:

    let mut req = req;
    let Ok(req_data) = req.json::<Vec<SendGridEvent>>().await else {
        return Response::error("", 400);
    };

    let mm_webhook_url = env.secret("MATTERMOST_WEBHOOK_URL")?.to_string();
    let username = env
        .var("MATTERMOST_USERNAME")
        .map_or_else(|_| "メール送信お知らせくん".to_string(), |v| v.to_string());

    let tasks = req_data
        .into_iter()
        .filter(|ev| ev.event == "processed")
        .map(|ev| {
            let username = username.clone();
            let mm_webhook_url = mm_webhook_url.clone();
            async move {
                let mm_ev = MmEvent::new(username, &ev.email);
                let sender = mm_ev.send(&mm_webhook_url);

                sender.await.unwrap_or_else(|e| {
                    console_error!("Failed to send to MM for {}: {:?}", ev.email, e)
                })
            }
        });
    join_all(tasks).await;

    Response::ok("")
}
