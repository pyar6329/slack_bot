use anyhow::{Error, Result};
use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
// {"ok":true}, {"ok":false}の条件分岐をserde(tag = "ok")で行う
struct RequestBody {
    trigger_id: String,
    #[serde(rename = "view")]
    body: RequestBodyData,
}

#[derive(Debug, Clone, Serialize)]
struct RequestBodyData {
    #[serde(rename = "type")]
    view_type: String,
    title: RequestBodyPlainText,
    blocks: Vec<RequestBodyInput>,
    submit: RequestBodyPlainText,
    close: RequestBodyPlainText,
}

#[derive(Debug, Clone, Serialize)]
struct RequestBodyPlainText {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(rename = "text")]
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct RequestBodyInput {
    #[serde(rename = "block_id")]
    id: String,
    #[serde(rename = "type")]
    content_type: String,
    #[serde(rename = "label")]
    content: RequestBodyPlainText,
    #[serde(rename = "element")]
    detail: RequestBodyInputDetail,
}

#[derive(Debug, Clone, Serialize)]
struct RequestBodyInputDetail {
    #[serde(rename = "action_id")]
    id: String,
    #[serde(rename = "type")]
    content_type: String,
    #[serde(rename = "placeholder")]
    detail: RequestBodyPlainText,
}

impl RequestBody {
    fn new_modal(trigger_id: &str, title: &str, blocks: &[RequestBodyInput]) -> Self {
        RequestBody {
            trigger_id: trigger_id.to_owned(),
            body: RequestBodyData {
                view_type: "modal".to_string(),
                title: RequestBodyPlainText {
                    content_type: "plain_text".to_string(),
                    content: title.to_string(),
                },
                blocks: blocks.to_vec(),
                close: RequestBodyPlainText {
                    content_type: "plain_text".to_string(),
                    content: "cancel".to_string(),
                },
                submit: RequestBodyPlainText {
                    content_type: "plain_text".to_string(),
                    content: "submit".to_string(),
                },
            },
        }
    }

    fn new_block(id: &str, title: &str, placeholder: &str) -> RequestBodyInput {
        RequestBodyInput {
            id: id.to_string(),
            content_type: "input".to_string(),
            content: RequestBodyPlainText {
                content_type: "plain_text".to_string(),
                content: title.to_string(),
            },
            detail: RequestBodyInputDetail {
                id: format!("action_{}", id),
                content_type: "plain_text_input".to_string(),
                detail: RequestBodyPlainText {
                    content_type: "plain_text".to_string(),
                    content: placeholder.to_string(),
                },
            },
        }
    }

    fn new_email_block() -> RequestBodyInput {
        Self::new_block("email", "Email Address", "Enter an email")
    }

    fn new_password_block() -> RequestBodyInput {
        Self::new_block("password", "Password", "Password must be over 8 words")
    }

    fn new_create_user_modal(trigger_id: &str) -> Self {
        Self::new_modal(
            trigger_id,
            "Create User",
            &[Self::new_email_block(), Self::new_password_block()],
        )
    }
}

impl From<RequestBody> for reqwest::Body {
    fn from(request_body: RequestBody) -> Self {
        let json = serde_json::to_string(&request_body).unwrap_or("{}".to_string());
        reqwest::Body::from(json)
    }
}

#[derive(Debug, Deserialize)]
struct ResponseBodySucceed {
    #[serde(rename = "ok")]
    _is_ok: bool,
    #[serde(rename = "view")]
    payload: ResponseBodySucceedPayload,
}

#[derive(Debug, Deserialize)]
pub struct ResponseBodySucceedPayload {
    pub id: String,
    pub callback_id: String,
}

#[derive(Debug, Deserialize)]
struct ResponseBodyFailed {
    #[serde(rename = "ok")]
    _is_ok: bool,
    error_message: String,
}

pub async fn create_modal(
    token: &str,
    trigger_id: &str,
) -> Result<ResponseBodySucceedPayload, Error> {
    let client = Client::new();

    let request_body = RequestBody::new_create_user_modal(trigger_id);

    let body_bytes = client
        .post("https://slack.com/api/views.open")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(CONTENT_TYPE, "application/json")
        .body(request_body)
        .send()
        .await?
        .bytes()
        .await?;

    let maybe_succeed_response: Result<ResponseBodySucceed, _> =
        serde_json::from_slice(&body_bytes);
    let maybe_failed_response: Result<ResponseBodyFailed, _> = serde_json::from_slice(&body_bytes);

    if let Ok(err) = maybe_failed_response {
        tracing::error!("failed to create modal: {}", err.error_message);
        return Err(Error::msg(err.error_message));
    }

    maybe_succeed_response.map(|res| res.payload).map_err(|_| {
        tracing::error!("failed to create modal: parse error succeed reponse");

        Error::msg("failed to create modal: parse error succeed reponse")
    })
}
