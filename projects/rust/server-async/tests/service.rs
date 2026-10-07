use rm_server_async::Service;
use serde_json::{Value, json};

#[test]
fn input_validation_and_baseline() {
    let service = Service::default();
    assert_eq!(
        service.handle("GET", "/ping", &Value::Null, ""),
        (200, json!({"data":"pong"}))
    );
    for body in [
        Value::Null,
        json!([]),
        json!({"username":true,"password":"password1"}),
        json!({"username":"a/b","password":"password1"}),
    ] {
        assert_eq!(service.handle("POST", "/users", &body, "").0, 400);
    }
    assert_eq!(service.handle("GET", "/texts", &Value::Null, "").0, 401);
    assert_eq!(service.handle("GET", "/missing", &Value::Null, "").0, 404);
}

#[test]
fn concurrent_registration_has_one_winner() {
    let service = std::sync::Arc::new(Service::default());
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let service = service.clone();
            std::thread::spawn(move || {
                service
                    .handle(
                        "POST",
                        "/users",
                        &json!({"username":"alice","password":"password1"}),
                        "",
                    )
                    .0
            })
        })
        .collect();
    let statuses: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(statuses.iter().filter(|&&s| s == 201).count(), 1);
    assert_eq!(statuses.iter().filter(|&&s| s == 409).count(), 3);
}
#[test]
fn echo_returns_text() {
    let service = Service::default();

    let body = json!({
        "text": "hello"
    });

    assert_eq!(
        service.handle("POST", "/echo", &body, ""),
        (
            200,
            json!({
                "data": "hello"
            })
        )
    );
}

#[test]
fn echo_rejects_invalid_fields() {
    let service = Service::default();

    assert_eq!(service.handle("POST", "/echo", &json!({}), "").0, 400);

    assert_eq!(
        service.handle("POST", "/echo", &json!({"text":123}), ""),
        (400, json!({"message":"Expected text"}))
    );
}
#[test]
fn echo_rejects_extra_fields() {
    let service = Service::default();

    assert_eq!(
        service
            .handle(
                "POST",
                "/echo",
                &json!({
                    "text":"hello",
                    "extra":123
                }),
                ""
            )
            .0,
        400
    );
}
#[test]
fn text_upload_creates_and_overwrites() {
    let service = Service::default();

    let account = json!({
        "username": "alice",
        "password": "password1"
    });

    assert_eq!(service.handle("POST", "/users", &account, "").0, 201);

    let login = service.handle("POST", "/sessions", &account, "").1;

    let token = format!("Bearer {}", login["data"]["token"].as_str().unwrap());

    assert_eq!(
        service
            .handle(
                "PUT",
                "/texts/note",
                &json!({
                    "text":"hello"
                }),
                &token
            )
            .0,
        200
    );

    assert_eq!(
        service
            .handle(
                "PUT",
                "/texts/note",
                &json!({
                    "text":"world"
                }),
                &token
            )
            .0,
        200
    );

    assert_eq!(
        service.handle("GET", "/texts", &Value::Null, &token).1,
        json!({
            "data":["note"]
        })
    );
}
#[test]
fn text_upload_requires_login() {
    let service = Service::default();

    assert_eq!(
        service
            .handle(
                "PUT",
                "/texts/note",
                &json!({
                    "text":"hello"
                }),
                "Bearer wrong"
            )
            .0,
        401
    );
}
#[test]
fn reading_missing_text_returns_404() {
    let service = Service::default();

    let account = json!({
        "username":"alice",
        "password":"password1"
    });

    service.handle("POST", "/users", &account, "");

    let login = service.handle("POST", "/sessions", &account, "").1;

    let token = format!("Bearer {}", login["data"]["token"].as_str().unwrap());

    assert_eq!(
        service
            .handle("GET", "/texts/not_exist", &Value::Null, &token)
            .0,
        404
    );
}
#[test]
fn users_have_separate_texts() {
    let service = Service::default();

    let alice = json!({
        "username":"alice",
        "password":"password1"
    });

    let bob = json!({
        "username":"bob",
        "password":"password1"
    });

    service.handle("POST", "/users", &alice, "");

    service.handle("POST", "/users", &bob, "");

    let alice_token = {
        let login = service.handle("POST", "/sessions", &alice, "").1;

        format!("Bearer {}", login["data"]["token"].as_str().unwrap())
    };

    let bob_token = {
        let login = service.handle("POST", "/sessions", &bob, "").1;

        format!("Bearer {}", login["data"]["token"].as_str().unwrap())
    };

    service.handle(
        "PUT",
        "/texts/note",
        &json!({
            "text":"alice"
        }),
        &alice_token,
    );

    service.handle(
        "PUT",
        "/texts/note",
        &json!({
            "text":"bob"
        }),
        &bob_token,
    );

    assert_eq!(
        service.handle("GET", "/texts/note", &Value::Null, &alice_token),
        (
            200,
            json!({
                "data":"alice"
            })
        )
    );

    assert_eq!(
        service.handle("GET", "/texts/note", &Value::Null, &bob_token),
        (
            200,
            json!({
                "data":"bob"
            })
        )
    );
}
