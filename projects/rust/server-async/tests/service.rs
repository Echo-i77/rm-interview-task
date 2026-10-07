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
