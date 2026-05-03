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

    Response::ok("")
}
