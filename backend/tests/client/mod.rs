use super::*;
use crate::secret::Memory;

use crate::test_support::{
    http, https, reply, Reply, Request, Scratch, Server, CA, CLIENT, CLIENT_KEY, NAS_ID, OTHER,
    PC_ID,
};

fn remote(id: &str, state: &str) -> RemoteSync {
    RemoteSync {
        id: id.into(),
        name: "Photos".into(),
        nas_path: "v-1/Photos".into(),
        pc_path: "/home/me/Photos".into(),
        mode: "two-way".into(),
        paused: false,
        revoked: false,
        state: state.into(),
        progress: Some(40.0),
        last_sync: None,
    }
}

fn volumes() -> Vec<Choice> {
    vec![Choice {
        name: "Dati".into(),
        path: "v-1".into(),
    }]
}

#[test]
fn folders_overlap_when_one_holds_the_other() {
    assert!(overlaps(
        Path::new("/home/me"),
        Path::new("/home/me/Photos")
    ));
    assert!(overlaps(
        Path::new("/home/me/Photos"),
        Path::new("/home/me/Photos")
    ));
    assert!(!overlaps(
        Path::new("/home/me/Photos"),
        Path::new("/home/me/Photos2")
    ));
    assert!(!computer_name().is_empty());
}

#[test]
fn the_folder_of_the_nas_shows_the_name_of_its_disk() {
    assert_eq!(nas_label("v-1/Photos", &volumes()), "Dati/Photos");
    assert_eq!(nas_label("v-1", &volumes()), "Dati");
    assert_eq!(nas_label("v-9/x", &volumes()), "v-9/x");
}

#[test]
fn a_sync_shows_what_the_computer_and_the_nas_say_of_it() {
    let synced = remote("f-1", "synced");
    let state = |sync: &RemoteSync, local: Option<&FolderStatus>, connected, paused| {
        view_of(sync, local, connected, paused, &volumes(), true).state
    };
    assert_eq!(state(&synced, None, true, false), "synced");
    assert_eq!(state(&synced, None, true, true), "paused");
    assert_eq!(state(&synced, None, false, false), "offline");
    let revoked = RemoteSync {
        revoked: true,
        ..synced.clone()
    };
    assert_eq!(state(&revoked, None, true, false), "revoked");
    let failing = FolderStatus {
        error: "no space".into(),
        ..FolderStatus::default()
    };
    assert_eq!(state(&synced, Some(&failing), true, false), "error");
    let busy = FolderStatus {
        state: "syncing".into(),
        global_bytes: 200,
        need_bytes: 50,
        need_files: 2,
        ..FolderStatus::default()
    };
    let view = view_of(&synced, Some(&busy), true, false, &volumes(), true);
    assert_eq!(
        (view.state.as_str(), view.progress),
        ("syncing", Some(75.0))
    );
    assert_eq!(view.nas_label, "Dati/Photos");
    assert_eq!(
        state(&remote("f-1", "scanning"), None, true, false),
        "syncing"
    );
    assert_eq!(state(&remote("f-1", "error"), None, true, false), "error");

    assert_eq!(
        view_of(&synced, None, true, false, &volumes(), false).state,
        "unchosen"
    );
}

#[test]
fn a_sync_takes_only_the_folder_chosen_on_this_computer() {
    let synced = remote("f-1", "synced");
    assert!(chosen_here(None, &synced));
    let mut chosen = BTreeMap::new();
    assert!(!chosen_here(Some(&chosen), &synced));
    chosen.insert("f-1".to_owned(), synced.pc_path.clone());
    assert!(chosen_here(Some(&chosen), &synced));
    chosen.insert("f-1".to_owned(), "/somewhere/else".to_owned());
    assert!(!chosen_here(Some(&chosen), &synced));
}

#[test]
fn no_sync_takes_the_top_of_a_disk_the_home_the_system_or_the_application() {
    let own = Path::new("/home/someone/.config/com.nasos.sync");
    assert!(forbidden(Path::new("/"), &[own]));
    assert!(forbidden(Path::new("/etc/nas"), &[own]));
    assert!(forbidden(Path::new("/usr"), &[own]));
    assert!(forbidden(own, &[own]));
    assert!(forbidden(Path::new("/home/someone/.config"), &[own]));
    if let Some(home) = std::env::var_os("HOME") {
        assert!(forbidden(Path::new(&home), &[own]));
    }
    assert!(!forbidden(Path::new("/tmp/nas-sync-photos"), &[own]));
    assert!(!forbidden(
        Path::new("/home/someone/Documents/test"),
        &[own]
    ));
}

fn linked() -> Settings {
    Settings {
        link: Some(Link {
            name: "PC".into(),
            host: "127.0.0.1".into(),
            port: 443,
            authority: CA.into(),
            certificate: CLIENT.into(),
            nas_id: NAS_ID.into(),
            sync_port: 22000,
        }),
        ..Settings::default()
    }
}

#[test]
fn the_client_is_as_far_as_its_least_advanced_sync() {
    let view = |state: &str| SyncView {
        state: state.into(),
        ..view_of(&remote("f", "synced"), None, true, false, &[], true)
    };
    let settings = linked();
    assert_eq!(
        overall(&Settings::default(), true, None, true, &[]),
        (Status::Unconfigured, None)
    );
    let paused = Settings {
        paused: true,
        ..settings.clone()
    };
    assert_eq!(overall(&paused, true, None, true, &[]).0, Status::Paused);
    assert_eq!(
        overall(&settings, false, None, true, &[]),
        (Status::Disconnected, Some(ClientError::EngineUnavailable))
    );
    assert_eq!(
        overall(
            &settings,
            true,
            Some(ClientError::NasUnreachable),
            true,
            &[]
        ),
        (Status::Disconnected, Some(ClientError::NasUnreachable))
    );
    assert_eq!(
        overall(&settings, true, None, false, &[view("synced")]),
        (Status::Disconnected, Some(ClientError::NotConnected))
    );
    assert_eq!(overall(&settings, true, None, false, &[]).0, Status::Synced);
    assert_eq!(
        overall(
            &settings,
            true,
            None,
            true,
            &[view("synced"), view("syncing")]
        )
        .0,
        Status::Syncing
    );
    assert_eq!(
        overall(&settings, true, None, true, &[view("error")]).0,
        Status::Error
    );
    assert_eq!(
        overall(&settings, true, None, true, &[view("synced")]).0,
        Status::Synced
    );
}

fn nas(authority: &'static str) -> Server {
    https(move |request: &Request| {
        let sync = serde_json::json!({
            "id": "f-1", "name": "Photos", "nasPath": "v-1/Photos", "pcPath": "/tmp/nas-sync-photos",
            "mode": "two-way", "paused": request.body.contains(r#""paused":true"#), "revoked": false,
            "state": "synced",
        })
        .to_string();
        match (request.method.as_str(), request.path.as_str()) {
            (_, "/health") => reply(200, "ok"),
            (_, "/auth/ca.crt") => reply(200, authority),
            (_, "/auth/login") => Reply {
                status: 200,
                headers: vec![("Set-Cookie", "nas_session=token".to_owned())],
                body: "{}".into(),
            },
            (_, "/auth/confirm") => reply(200, r#"{"code":"once"}"#),
            (_, "/auth/clients") => reply(
                201,
                &serde_json::json!({ "certificate": CLIENT, "key": CLIENT_KEY, "authority": CA })
                    .to_string(),
            ),
            (_, "/client/sync/pair") => {
                reply(200, &format!(r#"{{"deviceId":"{NAS_ID}","port":22000}}"#))
            }
            ("GET", path) if path.starts_with("/client/sync/folders") => {
                reply(200, r#"[{"name":"Dati","path":"v-1"}]"#)
            }
            ("GET", "/client/sync/syncs") => reply(200, &format!("[{sync}]")),
            ("POST", "/client/sync/syncs") | ("PUT", "/client/sync/syncs/f-1") => reply(200, &sync),
            ("DELETE", _) => reply(204, ""),
            _ => reply(204, ""),
        }
    })
}

fn engine_api() -> Server {
    http(|request| match request.path.as_str() {
        "/rest/system/status" => reply(200, &format!(r#"{{"myID":"{PC_ID}"}}"#)),
        "/rest/config/devices" => reply(
            200,
            &format!(r#"[{{"deviceID":"{PC_ID}"}},{{"deviceID":"OLD"}}]"#),
        ),
        "/rest/config/folders" => reply(200, r#"[{"id":"stale"}]"#),
        "/rest/system/connections" => reply(
            200,
            &format!(r#"{{"connections":{{"{NAS_ID}":{{"connected":true}}}}}}"#),
        ),
        path if path.starts_with("/rest/db/status") => reply(200, r#"{"state":"idle"}"#),
        path if path.starts_with("/rest/events") => reply(
            200,
            r#"[{"id":3,"type":"RemoteChangeDetected","time":"t","data":{"folder":"f-1","type":"file","action":"added","path":"a.jpg"}}]"#,
        ),
        _ => reply(200, ""),
    })
}

#[tokio::test]
async fn a_computer_pairs_syncs_and_unpairs() {
    let gateway = nas(CA);
    let api = engine_api();
    let scratch = Scratch::new("client");
    let client = Client::new(
        scratch.0.clone(),
        Arc::new(Memory::default()),
        Some(Engine::answering(scratch.0.join("engine"), api.port)),
    );
    assert_eq!(client.snapshot().await.status, Status::Unconfigured);
    let probe = client.probe("127.0.0.1", gateway.port).await.unwrap();
    assert_eq!(
        client
            .connect(
                "127.0.0.1",
                gateway.port,
                "wrong",
                "admin",
                "secret",
                "Studio PC"
            )
            .await,
        Err(ClientError::TrustChanged)
    );
    assert_eq!(
        client
            .connect(
                "127.0.0.1",
                gateway.port,
                &probe.fingerprint,
                "admin",
                "secret",
                ""
            )
            .await,
        Err(ClientError::InvalidSync)
    );
    client
        .connect(
            "127.0.0.1",
            gateway.port,
            &probe.fingerprint,
            "admin",
            "secret",
            "Studio PC",
        )
        .await
        .unwrap();
    assert_eq!(
        settings::load(&scratch.0).unwrap().link.unwrap().nas_id,
        NAS_ID
    );

    client.tick().await;
    let snapshot = client.snapshot().await;
    assert_eq!(snapshot.status, Status::Synced);
    assert_eq!(snapshot.syncs[0].nas_label, "Dati/Photos");
    assert_eq!(snapshot.link.unwrap().name, "Studio PC");

    assert_eq!(snapshot.syncs[0].state, "unchosen");
    assert!(!api
        .calls()
        .contains(&"PUT /rest/config/folders/f-1".to_owned()));
    client
        .change(
            "f-1",
            SyncChange {
                pc_path: Some("/tmp/nas-sync-photos".into()),
                ..SyncChange::default()
            },
        )
        .await
        .unwrap();
    assert!(api
        .calls()
        .contains(&"DELETE /rest/config/folders/stale".to_owned()));
    assert!(api
        .calls()
        .contains(&"DELETE /rest/config/devices/OLD".to_owned()));
    assert!(api
        .calls()
        .contains(&"PUT /rest/config/folders/f-1".to_owned()));

    let (last, entries) = client.follow_events(0).await.unwrap();
    assert_eq!(last, 3);
    assert_eq!(client.record(entries).len(), 1);
    assert_eq!(client.history(10)[0].path, "a.jpg");

    assert_eq!(client.folders("").await.unwrap()[0].name, "Dati");
    let taken = NewSync {
        name: "Nested".into(),
        nas_path: "v-1/Other".into(),
        pc_path: "/tmp/nas-sync-photos/inner".into(),
        mode: "two-way".into(),
    };
    assert_eq!(client.create(taken).await, Err(ClientError::FolderTaken));
    let relative = NewSync {
        name: "Relative".into(),
        nas_path: "v-1/Other".into(),
        pc_path: "relative".into(),
        mode: "two-way".into(),
    };
    assert_eq!(client.create(relative).await, Err(ClientError::InvalidSync));

    let computer = Scratch::new("client-computer");
    let inside = NewSync {
        name: "Inside".into(),
        nas_path: "v-1/Docs".into(),
        pc_path: scratch.0.join("Docs").to_string_lossy().into_owned(),
        mode: "nas-to-pc".into(),
    };
    assert_eq!(client.create(inside).await, Err(ClientError::InvalidSync));
    let fresh = NewSync {
        name: "Docs".into(),
        nas_path: "v-1/Docs".into(),
        pc_path: computer.0.join("Docs").to_string_lossy().into_owned(),
        mode: "nas-to-pc".into(),
    };
    client.create(fresh).await.unwrap();
    client
        .change(
            "f-1",
            SyncChange {
                paused: Some(true),
                ..SyncChange::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(client.snapshot().await.syncs[0].state, "paused");
    assert_eq!(
        client.change("unknown", SyncChange::default()).await,
        Err(ClientError::InvalidSync)
    );
    client.remove("f-1").await.unwrap();

    client.set_paused(true).await.unwrap();
    assert_eq!(client.snapshot().await.status, Status::Paused);
    client
        .save_preferences(Preferences {
            notifications: false,
            autostart: false,
            max_send_kbps: 64,
            max_recv_kbps: 0,
        })
        .await
        .unwrap();
    client
        .set_appearance(Appearance {
            locale: "en".into(),
            theme: "light".into(),
        })
        .await
        .unwrap();
    assert_eq!(
        client
            .set_appearance(Appearance {
                locale: "xx".into(),
                theme: "light".into()
            })
            .await,
        Err(ClientError::InvalidSync)
    );
    assert_eq!(client.snapshot().await.appearance.theme, "light");
    client.move_link("127.0.0.1", gateway.port).await.unwrap();
    let impostor = nas(OTHER);
    assert_eq!(
        client.move_link("127.0.0.1", impostor.port).await,
        Err(ClientError::NotANas)
    );

    let restarted = Client::new(
        scratch.0.clone(),
        Arc::new(Memory::default()),
        Some(Engine::answering(scratch.0.join("engine"), api.port)),
    );
    assert_eq!(restarted.history(10).len(), 1);
    assert_eq!(restarted.snapshot().await.link.unwrap().name, "Studio PC");

    client.disconnect().await.unwrap();
    assert!(client.snapshot().await.link.is_none());
    assert!(client.history(10).is_empty());
    assert_eq!(client.folders("").await, Err(ClientError::NotPaired));
}

#[tokio::test]
async fn an_unreachable_nas_is_counted_from_its_first_failure() {
    let scratch = Scratch::new("client");
    let gateway = nas(CA);
    let secrets = Arc::new(Memory::default());
    secrets.save(CLIENT_KEY).unwrap();
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    let mut settings = linked();
    if let Some(link) = settings.link.as_mut() {
        link.port = port;
    }
    settings::save(&scratch.0, &settings).unwrap();
    let client = Client::new(scratch.0.clone(), secrets, None);
    client.tick().await;
    assert!(client.unreachable_for().await.is_some());
    let snapshot = client.snapshot().await;
    assert_eq!(snapshot.status, Status::Disconnected);
    assert_eq!(
        client
            .probe("127.0.0.1", gateway.port)
            .await
            .unwrap()
            .fingerprint,
        snapshot.link.unwrap().fingerprint
    );
    assert_eq!(
        client
            .connect("127.0.0.1", gateway.port, "x", "a", "b", "PC")
            .await,
        Err(ClientError::TrustChanged)
    );

    let fingerprint = client
        .probe("127.0.0.1", gateway.port)
        .await
        .unwrap()
        .fingerprint;
    let started = std::time::Instant::now();
    assert_eq!(
        client
            .connect("127.0.0.1", gateway.port, &fingerprint, "a", "b", "PC")
            .await,
        Err(ClientError::EngineUnavailable)
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    assert!(client.follow_events(0).await.is_err());
    client.engine_started().await;
}
