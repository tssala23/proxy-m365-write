use serde::{Deserialize, Serialize};

const MAX_SUBJECT_CHARS: usize = 200;
const MAX_BODY_CHARS: usize = 10_000;
const MAX_RECIPIENTS: usize = 10;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub subject: String,
    pub body: Body,
    #[serde(
        rename = "toRecipients",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub to_recipients: Vec<Recipient>,
    #[serde(
        rename = "ccRecipients",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub cc_recipients: Vec<Recipient>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Recipient {
    #[serde(rename = "emailAddress")]
    pub email_address: EmailAddress,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EmailAddress {
    pub address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

fn valid_email(value: &str) -> bool {
    let value = value.trim();
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !value.chars().any(char::is_whitespace)
        && value.len() <= 254
}

impl Draft {
    pub fn parse_and_validate(bytes: &[u8]) -> Result<Vec<u8>, String> {
        let draft: Draft = serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid draft JSON: {error}"))?;
        let subject_len = draft.subject.trim().chars().count();
        if subject_len == 0 || subject_len > MAX_SUBJECT_CHARS {
            return Err("subject must contain 1 to 200 characters".into());
        }
        if !matches!(draft.body.content_type.as_str(), "Text" | "HTML") {
            return Err("body.contentType must be Text or HTML".into());
        }
        if draft.body.content.chars().count() > MAX_BODY_CHARS {
            return Err("body content exceeds 10000 characters".into());
        }
        let recipients = draft.to_recipients.len() + draft.cc_recipients.len();
        if recipients > MAX_RECIPIENTS {
            return Err("draft exceeds 10 recipients".into());
        }
        for recipient in draft.to_recipients.iter().chain(draft.cc_recipients.iter()) {
            if !valid_email(&recipient.email_address.address) {
                return Err("recipient contains an invalid email address".into());
            }
            if recipient
                .email_address
                .name
                .as_ref()
                .is_some_and(|name| name.chars().count() > 200)
            {
                return Err("recipient name exceeds 200 characters".into());
            }
        }
        serde_json::to_vec(&draft).map_err(|error| format!("failed to encode draft: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::Draft;

    #[test]
    fn accepts_a_minimal_draft() {
        let payload = br#"{"subject":"Test","body":{"contentType":"Text","content":"Hello"}}"#;
        assert!(Draft::parse_and_validate(payload).is_ok());
    }

    #[test]
    fn rejects_unknown_fields_and_invalid_recipients() {
        let attachment = br#"{"subject":"Test","body":{"contentType":"Text","content":"Hello"},"attachments":[]}"#;
        assert!(Draft::parse_and_validate(attachment).is_err());
        let invalid = br#"{"subject":"Test","body":{"contentType":"Text","content":"Hello"},"toRecipients":[{"emailAddress":{"address":"not-an-email"}}]}"#;
        assert!(Draft::parse_and_validate(invalid).is_err());
    }
}
