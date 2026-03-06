use std::io::{self, Write};

use ipc::{CommandFrame, ResponseFrame};

pub fn run_shell(transport: impl FnMut(CommandFrame) -> ResponseFrame) -> io::Result<()> {
    run_shell_with_auth(None, transport)
}

pub fn run_shell_with_auth(
    auth_token: Option<String>,
    mut transport: impl FnMut(CommandFrame) -> ResponseFrame,
) -> io::Result<()> {
    println!("AuroraOS shell v0.1");
    println!("Type 'help' for commands.");

    let stdin = io::stdin();
    let mut next_frame_id: u64 = 1;

    loop {
        print!("aurora> ");
        io::stdout().flush()?;

        let mut line = String::new();
        let bytes = stdin.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        let request =
            CommandFrame::with_auth(next_frame_id, auth_token.clone(), line.trim().to_string());
        next_frame_id = next_frame_id.saturating_add(1);

        let response = transport(request);
        if !response.payload.is_empty() {
            println!("{}", response.payload);
        }

        if response.exit {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use ipc::{encode_response, decode_response, encode_command, decode_command, CommandFrame, ResponseFrame};

    #[test]
    fn response_encoding_stays_stable() {
        let frame = ResponseFrame::new(42, false, "test");
        let encoded = encode_response(&frame);
        let decoded = decode_response(&encoded).expect("response should decode");
        assert_eq!(decoded, frame);
    }

    #[test]
    fn command_frame_creation() {
        let frame = CommandFrame::new(1, "help");
        assert_eq!(frame.id, 1);
        assert_eq!(frame.payload, "help");
        assert!(frame.auth_token.is_none());
    }

    #[test]
    fn command_frame_with_auth_token() {
        let frame = CommandFrame::with_auth(10, Some("tok".into()), "status");
        let encoded = encode_command(&frame);
        let decoded = decode_command(&encoded).unwrap();
        assert_eq!(decoded.auth_token, Some("tok".into()));
        assert_eq!(decoded.payload, "status");
    }

    #[test]
    fn response_exit_true() {
        let frame = ResponseFrame::new(1, true, "bye");
        let encoded = encode_response(&frame);
        let decoded = decode_response(&encoded).unwrap();
        assert!(decoded.exit);
    }

    #[test]
    fn response_exit_false() {
        let frame = ResponseFrame::new(1, false, "ok");
        let encoded = encode_response(&frame);
        let decoded = decode_response(&encoded).unwrap();
        assert!(!decoded.exit);
    }

    #[test]
    fn response_with_multiline_payload() {
        let frame = ResponseFrame::new(5, false, "line1\nline2\nline3");
        let encoded = encode_response(&frame);
        let decoded = decode_response(&encoded).unwrap();
        assert_eq!(decoded.payload, "line1\nline2\nline3");
    }

    #[test]
    fn response_shutdown_flag() {
        let frame = ResponseFrame::with_shutdown(1, true, true, "shutting down");
        let encoded = encode_response(&frame);
        let decoded = decode_response(&encoded).unwrap();
        assert!(decoded.shutdown);
        assert!(decoded.exit);
    }

    #[test]
    fn command_roundtrip_preserves_id() {
        for id in [0u64, 1, 100, u64::MAX] {
            let frame = CommandFrame::new(id, "test");
            let encoded = encode_command(&frame);
            let decoded = decode_command(&encoded).unwrap();
            assert_eq!(decoded.id, id);
        }
    }
}
