#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFrame {
    pub id: u64,
    pub auth_token: Option<String>,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseFrame {
    pub id: u64,
    pub exit: bool,
    pub shutdown: bool,
    pub payload: String,
}

impl CommandFrame {
    pub fn new(id: u64, payload: impl Into<String>) -> Self {
        Self::with_auth(id, None, payload)
    }

    pub fn with_auth(
        id: u64,
        auth_token: Option<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            id,
            auth_token,
            payload: payload.into(),
        }
    }
}

impl ResponseFrame {
    pub fn new(id: u64, exit: bool, payload: impl Into<String>) -> Self {
        Self::with_shutdown(id, exit, false, payload)
    }

    pub fn with_shutdown(
        id: u64,
        exit: bool,
        shutdown: bool,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            id,
            exit,
            shutdown,
            payload: payload.into(),
        }
    }
}

pub fn encode_response(frame: &ResponseFrame) -> String {
    let payload = frame.payload.replace('\n', "\\n");
    format!(
        "id={};exit={};shutdown={};payload={}",
        frame.id,
        if frame.exit { 1 } else { 0 },
        if frame.shutdown { 1 } else { 0 },
        payload
    )
}

pub fn encode_command(frame: &CommandFrame) -> String {
    let payload = frame.payload.replace('\n', "\\n");
    let auth = frame
        .auth_token
        .as_deref()
        .unwrap_or("")
        .replace('\n', "\\n");
    format!("id={};auth={};payload={}", frame.id, auth, payload)
}

pub fn decode_command(encoded: &str) -> Result<CommandFrame, String> {
    let mut id = None;
    let mut auth_token = None;
    let mut payload = None;

    for part in encoded.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };

        match key {
            "id" => {
                id = value.parse::<u64>().ok();
            }
            "auth" => {
                let token = value.replace("\\n", "\n");
                auth_token = if token.is_empty() { None } else { Some(token) };
            }
            "payload" => {
                payload = Some(value.replace("\\n", "\n"));
            }
            _ => {}
        }
    }

    let id = id.ok_or_else(|| "missing id".to_string())?;
    let payload = payload.ok_or_else(|| "missing payload".to_string())?;
    Ok(CommandFrame {
        id,
        auth_token,
        payload,
    })
}

pub fn decode_response(encoded: &str) -> Result<ResponseFrame, String> {
    let mut id = None;
    let mut exit = None;
    let mut shutdown = None;
    let mut payload = None;

    for part in encoded.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };

        match key {
            "id" => {
                id = value.parse::<u64>().ok();
            }
            "exit" => {
                exit = match value {
                    "0" => Some(false),
                    "1" => Some(true),
                    _ => None,
                };
            }
            "shutdown" => {
                shutdown = match value {
                    "0" => Some(false),
                    "1" => Some(true),
                    _ => None,
                };
            }
            "payload" => {
                payload = Some(value.replace("\\n", "\n"));
            }
            _ => {}
        }
    }

    let id = id.ok_or_else(|| "missing id".to_string())?;
    let exit = exit.ok_or_else(|| "missing exit".to_string())?;
    let shutdown = shutdown.ok_or_else(|| "missing shutdown".to_string())?;
    let payload = payload.ok_or_else(|| "missing payload".to_string())?;

    Ok(ResponseFrame {
        id,
        exit,
        shutdown,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        decode_command, decode_response, encode_command, encode_response, CommandFrame, ResponseFrame,
    };

    #[test]
    fn roundtrips_response_frame() {
        let frame = ResponseFrame::with_shutdown(7, false, true, "ok\nline2");
        let encoded = encode_response(&frame);
        let decoded = decode_response(&encoded).expect("decode should work");

        assert_eq!(decoded, frame);
    }

    #[test]
    fn roundtrips_command_frame() {
        let frame = CommandFrame::with_auth(9, Some("token-123".to_string()), "status\nnow");
        let encoded = encode_command(&frame);
        let decoded = decode_command(&encoded).expect("decode should work");

        assert_eq!(decoded, frame);
    }

    #[test]
    fn command_frame_no_auth() {
        let frame = CommandFrame::new(1, "help");
        assert_eq!(frame.id, 1);
        assert!(frame.auth_token.is_none());
        assert_eq!(frame.payload, "help");
    }

    #[test]
    fn command_frame_with_auth() {
        let frame = CommandFrame::with_auth(5, Some("secret".into()), "status");
        assert_eq!(frame.auth_token, Some("secret".into()));
    }

    #[test]
    fn response_frame_no_shutdown() {
        let frame = ResponseFrame::new(1, false, "ok");
        assert!(!frame.exit);
        assert!(!frame.shutdown);
        assert_eq!(frame.payload, "ok");
    }

    #[test]
    fn response_frame_with_shutdown() {
        let frame = ResponseFrame::with_shutdown(1, true, true, "bye");
        assert!(frame.exit);
        assert!(frame.shutdown);
    }

    #[test]
    fn roundtrip_empty_payload() {
        let frame = CommandFrame::new(0, "");
        let encoded = encode_command(&frame);
        let decoded = decode_command(&encoded).unwrap();
        assert_eq!(decoded.payload, "");
    }

    #[test]
    fn roundtrip_response_empty_payload() {
        let frame = ResponseFrame::new(0, false, "");
        let encoded = encode_response(&frame);
        let decoded = decode_response(&encoded).unwrap();
        assert_eq!(decoded.payload, "");
    }

    #[test]
    fn decode_command_missing_id_fails() {
        let result = decode_command("auth=;payload=test");
        assert!(result.is_err());
    }

    #[test]
    fn decode_command_missing_payload_fails() {
        let result = decode_command("id=1;auth=");
        assert!(result.is_err());
    }

    #[test]
    fn decode_response_missing_exit_fails() {
        let result = decode_response("id=1;shutdown=0;payload=ok");
        assert!(result.is_err());
    }

    #[test]
    fn decode_response_missing_shutdown_fails() {
        let result = decode_response("id=1;exit=0;payload=ok");
        assert!(result.is_err());
    }

    #[test]
    fn encode_command_escapes_newlines() {
        let frame = CommandFrame::new(1, "line1\nline2");
        let encoded = encode_command(&frame);
        assert!(!encoded.contains('\n'));
        assert!(encoded.contains("\\n"));
    }

    #[test]
    fn encode_response_escapes_newlines() {
        let frame = ResponseFrame::new(1, false, "a\nb");
        let encoded = encode_response(&frame);
        assert!(!encoded.contains('\n'));
    }

    #[test]
    fn command_no_auth_roundtrip() {
        let frame = CommandFrame::new(42, "list services");
        let encoded = encode_command(&frame);
        let decoded = decode_command(&encoded).unwrap();
        assert!(decoded.auth_token.is_none());
        assert_eq!(decoded.payload, "list services");
    }

    #[test]
    fn response_exit_flag() {
        let frame = ResponseFrame::new(1, true, "goodbye");
        let encoded = encode_response(&frame);
        let decoded = decode_response(&encoded).unwrap();
        assert!(decoded.exit);
        assert!(!decoded.shutdown);
    }
}
