//! Hosting a Spotify Jam, through the private social-connect service.
//!
//! The Web API has no Jam endpoints. Spotify's own clients create a session
//! with `current_or_new`, read it with `current`, and end it with a `DELETE`
//! keyed by the session's device ID, all on `spclient`. The session is a
//! protobuf without a published schema; the fields read here are the ones
//! the desktop client's own answers show, and every one this module does not
//! recognise is ignored rather than rejected.
//!
//! Only hosting is implemented. A Jam is a shared queue on this computer's
//! player: guests add songs from their own devices, Spotify routes those
//! commands to the host over Connect, and librespot already applies them.

use anyhow::{Result, anyhow};
use librespot_core::Session;
use reqwest::{Client, Method, StatusCode};

/// The session Spotify keeps for this account, as the service describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JamSession {
    pub id: String,
    /// The session's device ID, the key the end request takes.
    pub device_id: String,
    /// `spotify:socialsession:…`, the link to share.
    pub uri: String,
    /// `hm://social-connect/v2/sessions/join/…`, the service's own join URI.
    pub join_uri: String,
    pub active: bool,
    pub owner: bool,
    pub members: Vec<Member>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub username: String,
    pub name: String,
    pub image: Option<String>,
}

impl Member {
    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            &self.username
        } else {
            &self.name
        }
    }
}

/// What the app shows about this account's Jam. Optimistic: starting and
/// ending change it the moment the button is used, before Spotify answers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Idle,
    Starting,
    Hosting(JamSession),
    /// Ending, with the session still at hand in case Spotify refuses.
    Ending(JamSession),
    Failed(String),
}

impl State {
    pub fn session(&self) -> Option<&JamSession> {
        match self {
            Self::Hosting(session) | Self::Ending(session) => Some(session),
            _ => None,
        }
    }

    pub fn busy(&self) -> bool {
        matches!(self, Self::Starting | Self::Ending(_))
    }
}

/// What the runtime learned about the Jam, for the app's request generation.
#[derive(Clone, Debug)]
pub enum Update {
    Started(Result<JamSession, String>),
    Refreshed(Result<Option<JamSession>, String>),
    Ended(Result<(), String>),
}

/// What the service says about this account's session now, or why it could
/// not be read.
pub async fn current(session: &Session, http: &Client) -> Result<Option<JamSession>> {
    let Some(bytes) = call(session, http, Method::GET, CURRENT).await? else {
        return Ok(None);
    };
    Ok(Some(decode_session(&bytes)?))
}

/// Create the account's session if it has none, and make this device the
/// one playing into it.
pub async fn start(session: &Session, http: &Client) -> Result<JamSession> {
    let Some(bytes) = call(session, http, Method::GET, CURRENT_OR_NEW).await? else {
        return Err(anyhow!("Spotify did not return a Jam session."));
    };
    decode_session(&bytes)
}

/// End the session this computer hosts. A session that is already gone
/// counts as ended.
pub async fn end(session: &Session, http: &Client, device_id: &str) -> Result<()> {
    let path = format!("{SESSIONS}/{device_id}");
    match call(session, http, Method::DELETE, &path).await? {
        Some(_) => Ok(()),
        // `call` maps 404 to `None`, and a session that is not there is over.
        None => Ok(()),
    }
}

const SESSIONS: &str = "/social-connect/v2/sessions";
const CURRENT: &str = "/social-connect/v2/sessions/current?alt=protobuf";
const CURRENT_OR_NEW: &str = "/social-connect/v2/sessions/current_or_new?activate=true";

/// One request, with the session's access token and the client token the
/// service is used to. A 404 is an answer, not a failure: it says there is
/// no session.
async fn call(
    session: &Session,
    http: &Client,
    method: Method,
    path: &str,
) -> Result<Option<Vec<u8>>> {
    let base = session.spclient().base_url().await?;
    let token = session.login5().auth_token().await?;
    let client_token = session.spclient().client_token().await.ok();
    let mut request = http
        .request(method, format!("{base}{path}"))
        .bearer_auth(&token.access_token)
        .header("Accept", "application/x-protobuf");
    if let Some(client_token) = client_token {
        request = request.header("client-token", client_token);
    }
    let response = request.send().await.map_err(reqwest::Error::without_url)?;
    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(explain(status, &body)));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(reqwest::Error::without_url)?;
    Ok(Some(bytes.to_vec()))
}

/// What to show the listener, without the service's JSON or a bare status.
fn explain(status: StatusCode, body: &str) -> String {
    if body.contains("SESSION_NOT_FOUND") {
        return "The Jam has already ended.".to_string();
    }
    if status == StatusCode::FORBIDDEN || status == StatusCode::UNAUTHORIZED {
        return "Spotify refused the Jam request. Check that playback is enabled and the account is Premium.".to_string();
    }
    format!("Spotify could not start the Jam ({status}). Check the connection and try again.")
}

/// Reads `socialconnect.Session` as far as this client uses it. Unknown
/// fields are skipped, so a field Spotify adds later cannot break the read.
fn decode_session(bytes: &[u8]) -> Result<JamSession> {
    let mut id = None;
    let mut device_id = None;
    let mut uri = None;
    let mut join_uri = None;
    let mut members = Vec::new();
    let mut owner = false;
    let mut active = false;
    for (field, value) in fields(bytes) {
        match (field, value) {
            (2, Value::Bytes(value)) => device_id = utf8(value),
            (3, Value::Bytes(value)) => id = utf8(value),
            (4, Value::Bytes(value)) => join_uri = utf8(value),
            (6, Value::Bytes(value)) => {
                if let Some(member) = decode_member(value) {
                    members.push(member);
                }
            }
            (7, Value::Bytes(value)) => uri = utf8(value),
            (9, Value::Varint(value)) => owner = value == 1,
            (10, Value::Varint(value)) => active = value == 1,
            _ => {}
        }
    }
    let (Some(id), Some(device_id)) = (id, device_id) else {
        return Err(anyhow!(
            "Spotify sent a Jam session this version does not understand."
        ));
    };
    Ok(JamSession {
        uri: uri.unwrap_or_else(|| format!("spotify:socialsession:{id}")),
        join_uri: join_uri.unwrap_or_default(),
        id,
        device_id,
        active,
        owner,
        members,
    })
}

fn decode_member(bytes: &[u8]) -> Option<Member> {
    let mut username = None;
    let mut name = None;
    let mut image = None;
    for (field, value) in fields(bytes) {
        if let (Value::Bytes(value), 3..=5) = (&value, field) {
            let Some(text) = utf8(value) else { continue };
            match field {
                3 => username = Some(text),
                4 => name = Some(text),
                _ => {
                    // The larger of the two avatars answers first; the
                    // compact one is enough for a list row.
                    if image.is_none() {
                        image = Some(text);
                    }
                }
            }
        }
    }
    let username = username?;
    Some(Member {
        username,
        name: name.unwrap_or_default(),
        image,
    })
}

fn utf8(bytes: &[u8]) -> Option<String> {
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

#[derive(Debug, PartialEq, Eq)]
enum Value<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
    Skipped,
}

/// The protobuf wire format, field by field. Groups are skipped, not read:
/// the service does not use them.
fn fields(bytes: &[u8]) -> Vec<(u64, Value<'_>)> {
    let mut fields = Vec::new();
    let mut rest = bytes;
    while let Some((key, tail)) = varint(rest) {
        let field = key >> 3;
        match key & 7 {
            0 => {
                let Some((value, tail)) = varint(tail) else {
                    break;
                };
                fields.push((field, Value::Varint(value)));
                rest = tail;
            }
            1 => {
                let Some(tail) = tail.get(8..) else { break };
                fields.push((field, Value::Skipped));
                rest = tail;
            }
            2 => {
                let Some((length, tail)) = varint(tail) else {
                    break;
                };
                let Some((value, tail)) = tail.split_at_checked(length as usize) else {
                    break;
                };
                fields.push((field, Value::Bytes(value)));
                rest = tail;
            }
            5 => {
                let Some(tail) = tail.get(4..) else { break };
                fields.push((field, Value::Skipped));
                rest = tail;
            }
            _ => break,
        }
    }
    fields
}

fn varint(bytes: &[u8]) -> Option<(u64, &[u8])> {
    let mut value = 0u64;
    for (index, byte) in bytes.iter().enumerate().take(10) {
        value |= u64::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Some((value, &bytes[index + 1..]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varint_bytes(value: u64) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut value = value;
        while value >= 0x80 {
            bytes.push((value as u8) | 0x80);
            value >>= 7;
        }
        bytes.push(value as u8);
        bytes
    }

    fn string_field(field: u64, text: &str) -> Vec<u8> {
        let mut bytes = varint_bytes(field << 3 | 2);
        bytes.extend(varint_bytes(text.len() as u64));
        bytes.extend_from_slice(text.as_bytes());
        bytes
    }

    fn varint_field(field: u64, value: u64) -> Vec<u8> {
        let mut bytes = varint_bytes(field << 3);
        bytes.extend(varint_bytes(value));
        bytes
    }

    fn member(username: &str, name: &str) -> Vec<u8> {
        let mut bytes = string_field(3, username);
        bytes.extend(string_field(4, name));
        bytes.extend(string_field(5, "https://i.scdn.co/small.jpg"));
        bytes
    }

    fn session_bytes() -> Vec<u8> {
        let mut bytes = varint_field(1, 1_793_199_172_539);
        bytes.extend(string_field(2, "4d13b7e477130477c7f2d4b28919adcb"));
        bytes.extend(string_field(3, "2lrq4k8j9oFLl2g4YcBg5Z"));
        bytes.extend(string_field(
            4,
            "hm://social-connect/v2/sessions/join/2lrq4k8j9oFLl2g4YcBg5Z",
        ));
        // A field this version does not know, between the ones it reads.
        bytes.extend(string_field(5, "b08732b5c537613a92701a7ef074f293"));
        let member = member("baturkacamak", "Batur Kacamak");
        let mut member_field = varint_bytes(6 << 3 | 2);
        member_field.extend(varint_bytes(member.len() as u64));
        member_field.extend(member);
        bytes.extend(member_field);
        bytes.extend(string_field(
            7,
            "spotify:socialsession:2lrq4k8j9oFLl2g4YcBg5Z",
        ));
        bytes.extend(varint_field(9, 1));
        bytes.extend(varint_field(10, 1));
        bytes.extend(varint_field(11, 1));
        bytes
    }

    #[test]
    fn reads_the_session_the_desktop_service_returns() {
        let session = decode_session(&session_bytes()).unwrap();
        assert_eq!(session.id, "2lrq4k8j9oFLl2g4YcBg5Z");
        assert_eq!(session.device_id, "4d13b7e477130477c7f2d4b28919adcb");
        assert_eq!(session.uri, "spotify:socialsession:2lrq4k8j9oFLl2g4YcBg5Z");
        assert_eq!(
            session.join_uri,
            "hm://social-connect/v2/sessions/join/2lrq4k8j9oFLl2g4YcBg5Z"
        );
        assert!(session.owner);
        assert!(session.active);
        assert_eq!(
            session.members,
            vec![Member {
                username: "baturkacamak".into(),
                name: "Batur Kacamak".into(),
                image: Some("https://i.scdn.co/small.jpg".into()),
            }]
        );
    }

    #[test]
    fn a_session_without_its_ids_is_not_half_read() {
        let mut bytes = varint_field(1, 1_793_199_172_539);
        bytes.extend(string_field(3, "2lrq4k8j9oFLl2g4YcBg5Z"));
        assert!(decode_session(&bytes).is_err());
    }

    #[test]
    fn unknown_fields_and_odd_members_are_skipped() {
        let mut bytes = session_bytes();
        bytes.extend(varint_field(99, 7));
        bytes.extend(string_field(98, "later"));
        // A member with no username is left out rather than shown blank.
        let nameless = string_field(4, "Nobody");
        let mut member_field = varint_bytes(6 << 3 | 2);
        member_field.extend(varint_bytes(nameless.len() as u64));
        member_field.extend(nameless);
        bytes.extend(member_field);
        let session = decode_session(&bytes).unwrap();
        assert_eq!(session.members.len(), 1);
    }

    #[test]
    fn the_member_name_falls_back_to_the_username() {
        let member = Member {
            username: "alice".into(),
            name: String::new(),
            image: None,
        };
        assert_eq!(member.display_name(), "alice");
    }

    #[test]
    fn a_service_error_is_explained_in_the_listeners_terms() {
        assert_eq!(
            explain(
                StatusCode::NOT_FOUND,
                r#"{"error_type":"SESSION_NOT_FOUND"}"#
            ),
            "The Jam has already ended."
        );
        assert!(explain(StatusCode::FORBIDDEN, "").contains("Premium"));
        assert!(explain(StatusCode::INTERNAL_SERVER_ERROR, "").contains("500"));
    }
}
