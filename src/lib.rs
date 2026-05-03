use base64::prelude::*;
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
    let req_data: Vec<SendGridEvent> = if let Ok(res) = req.json().await {
        res
    } else {
        return Response::error("", 400);
    };

    let mm_webhook_url = env.secret("MATTERMOST_WEBHOOK_URL")?.to_string();
    let username = env
        .var("MATTERMOST_USERNAME")
        .map_or_else(|_| "メール送信お知らせくん".to_string(), |v| v.to_string());

    for ev in req_data.iter().filter(|ev| ev.event == "Processed") {
        let mm_ev = MmEvent {
            text: format!("`{}`へのメール配信処理が実行されました。", ev.email),

            username: username.clone(),
        };
        let body = Some(serde_json::to_string(&mm_ev)?.into());

        let mm_init = RequestInit {
            method: Method::Post,
            headers: Headers::from_iter([("Content-Type", "application/json")]),
            body,
            ..Default::default()
        };

        Fetch::Request(Request::new_with_init(&mm_webhook_url, &mm_init)?)
            .send()
            .await?;
    }

    Response::ok("")
}
