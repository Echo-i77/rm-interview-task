use reqwest::{Method, blocking::Client};
use rm_client_sync::{exchange, make_echo_body, process_echo_line};
use serde_json::json;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::time::Duration;

#[test]
fn sends_http_authorization_and_preserves_error_status() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            if line == "\r\n" {
                break;
            }
            headers.push_str(&line);
        }
        assert!(headers.starts_with("GET /texts HTTP/1.1\r\n"));
        assert!(
            headers
                .to_lowercase()
                .contains("authorization: bearer sample\r\n")
        );
        stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 7\r\nConnection: close\r\n\r\nexpired").unwrap();
    });
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let result = exchange(&client, &url, Method::GET, "/texts", "sample", None).unwrap();
    assert_eq!(result, (401, json!({"message":"expired"})));
    peer.join().unwrap();
}

#[test]
fn echo_body_puts_input_in_text_field() {
    let text = "你好🙂\nEND\n";
    let body = make_echo_body(text);
    assert_eq!(
        body,
        json!({
            "text": "你好🙂\nEND\n"
        })
    );
}
#[test]
fn echo_can_be_empty() {
    let mut output = String::new();

    let finished = process_echo_line(&mut output, "END\n");

    assert!(finished);
    assert_eq!(output, "");
}

#[test]
fn echo_preserves_unicode_and_multiple_lines() {
    let mut output = String::new();

    process_echo_line(&mut output, "你好🙂\n");
    process_echo_line(&mut output, "hello\n");
    process_echo_line(&mut output, "END\n");

    assert_eq!(output, "你好🙂\nhello");
}

#[test]
fn echo_can_have_trailing_newline() {
    let mut output = String::new();

    process_echo_line(&mut output, "hello\n");
    process_echo_line(&mut output, "\n");
    process_echo_line(&mut output, "END\n");

    assert_eq!(output, "hello\n");
}

#[test]
fn end_can_be_used_as_text() {
    let mut output = String::new();

    process_echo_line(&mut output, "\\END\n");
    process_echo_line(&mut output, "END\n");

    assert_eq!(output, "END");
}

#[test]
fn backslash_end_can_be_used_as_text() {
    let mut output = String::new();

    process_echo_line(&mut output, "\\\\END\n");
    process_echo_line(&mut output, "END\n");

    assert_eq!(output, "\\END");
}

#[test]
fn echo_can_have_no_trailing_newline() {
    let mut output = String::new();

    process_echo_line(&mut output, "hello\n");
    process_echo_line(&mut output, "END\n");

    assert_eq!(output, "hello");
}

#[test]
fn echo_text_round_trips_over_http() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());

    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();

        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();

        let mut reader = BufReader::new(stream.try_clone().unwrap());

        let mut headers = String::new();
        let mut content_length = 0;

        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);

            if line == "\r\n" {
                break;
            }

            if let Some(value) = line.to_lowercase().strip_prefix("content-length: ") {
                content_length = value.trim().parse::<usize>().unwrap();
            }

            headers.push_str(&line);
        }

        assert!(headers.starts_with("POST /echo HTTP/1.1\r\n"));

        let mut body_bytes = vec![0; content_length];

        reader.read_exact(&mut body_bytes).unwrap();

        let body_text = String::from_utf8(body_bytes).unwrap();

        let body_json: serde_json::Value = serde_json::from_str(&body_text).unwrap();

        assert_eq!(body_json["text"].as_str(), Some("你好🙂\nEND\n"));

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            body_text.len(),
            body_text
        );

        stream.write_all(response.as_bytes()).unwrap();
    });

    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let original_text = "你好🙂\nEND\n";

    let body = make_echo_body(original_text);

    let (status, response) =
        exchange(&client, &url, Method::POST, "/echo", "", Some(&body)).unwrap();

    assert_eq!(status, 200);

    assert_eq!(response["text"].as_str(), Some(original_text));

    peer.join().unwrap();
}
