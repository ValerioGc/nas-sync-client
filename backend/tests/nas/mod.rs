use super::*;

use crate::test_support::{
    https, reply, Reply, Request, CA, CLIENT, CLIENT_KEY, NAS_ID, OTHER, PC_ID,
};

fn local(port: u16) -> Endpoint {
    Endpoint::new("127.0.0.1", port).unwrap()
}

#[test]
fn an_endpoint_is_a_name_or_an_address_with_a_port() {
    assert_eq!(
        Endpoint::new(" nas-studio.local ", 443).unwrap().host,
        "nas-studio.local"
    );
    assert_eq!(
        Endpoint::new("[fd00::5]", 8443).unwrap().url("/health"),
        "https://[fd00::5]:8443/health"
    );
    assert_eq!(
        Endpoint::new("192.168.1.15", 443)
            .unwrap()
            .sync_address(22000),
        "tcp://192.168.1.15:22000"
    );
    assert_eq!(
        Endpoint::new("fd00::5", 443).unwrap().sync_address(22000),
        "tcp://[fd00::5]:22000"
    );
    for (host, port) in [
        ("", 443),
        ("nas", 0),
        ("-nas", 443),
        ("nas/x", 443),
        ("a..b", 443),
    ] {
        assert_eq!(Endpoint::new(host, port), Err(ClientError::InvalidAddress));
    }
}

fn gateway_answers(authority: &'static str) -> impl Fn(&Request) -> Reply {
    move |request| match request.path.as_str() {
        "/health" => reply(200, "ok"),
        "/auth/ca.crt" => reply(200, authority),
        _ => reply(404, "{}"),
    }
}

#[tokio::test]
async fn the_first_link_learns_the_authority_that_issued_the_certificate_of_the_nas() {
    let server = https(gateway_answers(CA));
    let probe = probe(&local(server.port)).await.unwrap();
    assert_eq!(probe.fingerprint, tls::authority_fingerprint(CA).unwrap());
    assert_eq!(probe.authority, CA);

    let impostor = https(gateway_answers(OTHER));
    assert_eq!(probe_error(impostor.port).await, ClientError::NotANas);
    let silent = https(|_| reply(200, "hello"));
    assert_eq!(probe_error(silent.port).await, ClientError::NotANas);
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    assert_eq!(probe_error(port).await, ClientError::NasUnreachable);
}

async fn probe_error(port: u16) -> ClientError {
    probe(&local(port)).await.unwrap_err()
}

fn issuing(login: u16) -> impl Fn(&Request) -> Reply {
    move |request| match request.path.as_str() {
        "/auth/login" if login == 200 => Reply {
            status: 200,
            headers: vec![
                ("Set-Cookie", "nas_session=token; HttpOnly".to_owned()),
                ("Content-Type", "application/json".to_owned()),
            ],
            body: "{}".to_owned(),
        },
        "/auth/login" if login == 403 => Reply {
            status: 403,
            headers: vec![("Content-Type", "text/html; charset=utf-8".to_owned())],
            body: "<html></html>".to_owned(),
        },
        "/auth/login" => reply(login, "{}"),
        "/auth/confirm" => reply(200, r#"{"code":"once"}"#),
        "/auth/clients" => reply(
            201,
            &serde_json::json!({ "certificate": CLIENT, "key": CLIENT_KEY, "authority": CA })
                .to_string(),
        ),
        _ => reply(204, ""),
    }
}

#[tokio::test]
async fn the_password_is_used_once_to_be_given_the_certificate() {
    let server = https(issuing(200));
    let issued = enroll(
        &local(server.port),
        CA,
        "admin",
        "secret",
        "Studio PC",
        PC_ID,
    )
    .await
    .unwrap();
    assert_eq!(issued.certificate, CLIENT);
    assert_eq!(
        server.calls(),
        [
            "POST /auth/login",
            "POST /auth/confirm",
            "POST /auth/clients",
            "POST /auth/logout"
        ]
    );
    let seen = server.seen.lock().unwrap().clone();
    assert_eq!(seen[2].header("cookie"), Some("nas_session=token"));
    assert_eq!(seen[2].header("x-nas-confirmation"), Some("once"));
    assert!(seen[2].body.contains(PC_ID));
}

#[tokio::test]
async fn a_refused_sign_in_says_why() {
    for (status, error) in [
        (401, ClientError::InvalidCredentials),
        (429, ClientError::TooManyAttempts),
        (403, ClientError::SecurityRequired),
        (500, ClientError::NasUnreachable),
    ] {
        let server = https(issuing(status));
        assert_eq!(
            enroll(&local(server.port), CA, "admin", "bad", "PC", PC_ID)
                .await
                .unwrap_err(),
            error
        );
    }
}

fn sync_json() -> String {
    serde_json::json!({
        "id": "f-1", "name": "Photos", "nasPath": "v-1/Photos", "pcPath": "/home/me/Photos",
        "mode": "two-way", "paused": false, "revoked": false, "state": "synced",
    })
    .to_string()
}

#[tokio::test]
async fn the_sync_routes_are_reached_with_the_certificate_of_the_computer() {
    let server = https(|request| {
        if !request.certified {
            return reply(401, "{}");
        }
        match (request.method.as_str(), request.path.as_str()) {
            ("POST", "/client/sync/pair") => {
                reply(200, &format!(r#"{{"deviceId":"{NAS_ID}","port":22000}}"#))
            }
            ("GET", path) if path.starts_with("/client/sync/folders") => {
                reply(200, r#"[{"name":"Dati","path":"v-1"}]"#)
            }
            ("GET", "/client/sync/syncs") => reply(200, &format!("[{}]", sync_json())),
            ("POST", "/client/sync/syncs") | ("PUT", "/client/sync/syncs/f-1") => {
                reply(200, &sync_json())
            }
            ("DELETE", "/client/sync/syncs/f-1") => reply(204, ""),
            _ => reply(404, "{}"),
        }
    });
    let gateway = Gateway::new(local(server.port), CA, CLIENT, CLIENT_KEY).unwrap();
    assert_eq!(gateway.pair(PC_ID, "PC").await.unwrap().device_id, NAS_ID);
    assert_eq!(
        gateway.folders("Dati Nuovi/è").await.unwrap()[0].path,
        "v-1"
    );
    assert_eq!(gateway.syncs().await.unwrap()[0].name, "Photos");
    let new = NewSync {
        name: "Photos".into(),
        nas_path: "v-1/Photos".into(),
        pc_path: "/home/me/Photos".into(),
        mode: "two-way".into(),
    };
    assert_eq!(gateway.create(&new).await.unwrap().id, "f-1");
    let change = SyncChange {
        paused: Some(true),
        ..SyncChange::default()
    };
    assert_eq!(gateway.change("f-1", &change).await.unwrap().id, "f-1");
    gateway.remove("f-1").await.unwrap();
    assert!(server
        .calls()
        .contains(&"GET /client/sync/folders?path=Dati%20Nuovi/%C3%A8".to_owned()));
    assert!(server.seen.lock().unwrap()[4]
        .body
        .contains(r#""paused":true"#));
}

#[tokio::test]
async fn a_refused_sync_route_says_why() {
    for (status, html, error) in [
        (403, true, ClientError::SecurityRequired),
        (403, false, ClientError::NotPaired),
        (401, false, ClientError::NotPaired),
        (404, false, ClientError::AppMissing),
        (409, false, ClientError::FolderTaken),
        (400, false, ClientError::InvalidSync),
        (502, false, ClientError::NasUnreachable),
    ] {
        let server = https(move |_| Reply {
            status,
            headers: vec![(
                "Content-Type",
                if html {
                    "text/html"
                } else {
                    "application/json"
                }
                .to_owned(),
            )],
            body: "{}".to_owned(),
        });
        let gateway = Gateway::new(local(server.port), CA, CLIENT, CLIENT_KEY).unwrap();
        assert_eq!(gateway.syncs().await.unwrap_err(), error);
    }
}
