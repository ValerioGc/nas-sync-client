use super::*;

use crate::test_support::{http, reply, Scratch, NAS_ID, PC_ID};

#[test]
fn the_computer_holds_the_opposite_side_of_the_nas() {
    assert_eq!(folder_kind("pc-to-nas"), "sendonly");
    assert_eq!(folder_kind("nas-to-pc"), "receiveonly");
    assert_eq!(folder_kind("two-way"), "sendreceive");
}

#[test]
fn the_first_configuration_keeps_the_engine_on_the_local_network_and_the_loopback() {
    let config = first_config(22123);
    for setting in [
        "tcp://127.0.0.1:22123",
        "<globalAnnounceEnabled>false",
        "<localAnnounceEnabled>false",
        "<relaysEnabled>false",
        "<natEnabled>false",
        "<urAccepted>-1",
        "<crashReportingEnabled>false",
        "<autoUpgradeIntervalH>0",
    ] {
        assert!(config.contains(setting), "{setting}");
    }
}

#[test]
fn the_key_is_made_once_and_kept() {
    let scratch = Scratch::new("engine");
    let key = key_in(&scratch.0).unwrap();
    assert_eq!(key.len(), 32);
    assert_eq!(key_in(&scratch.0).unwrap(), key);
    fs::write(scratch.0.join("api-key"), "short").unwrap();
    assert_ne!(key_in(&scratch.0).unwrap(), "short");
    let engine = Engine::new(PathBuf::from("missing"), scratch.0.join("home")).unwrap();
    assert!(!engine.is_installed());
    assert!(!engine.is_running());
    assert!(scratch.0.join("home/config.xml").exists());
}

#[test]
fn the_program_is_beside_the_application_unless_development_says_otherwise() {
    assert!(program().to_string_lossy().contains("syncthing"));
}

#[tokio::test]
async fn the_engine_is_configured_through_its_api_with_its_key() {
    let server = http(
        |request| match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/rest/system/status") => reply(200, &format!(r#"{{"myID":"{PC_ID}"}}"#)),
            ("GET", "/rest/config/devices") => reply(
                200,
                &format!(r#"[{{"deviceID":"{PC_ID}"}},{{"deviceID":"{NAS_ID}"}}]"#),
            ),
            ("GET", "/rest/config/folders") => reply(200, r#"[{"id":"f-1"}]"#),
            ("GET", "/rest/db/status?folder=f-1") => {
                reply(200, r#"{"state":"syncing","globalBytes":10,"needBytes":4}"#)
            }
            ("GET", "/rest/system/connections") => reply(
                200,
                &format!(r#"{{"connections":{{"{NAS_ID}":{{"connected":true}}}}}}"#),
            ),
            ("GET", path) if path.starts_with("/rest/events") => reply(200, r#"[{"id":7}]"#),
            ("DELETE", "/rest/config/folders/broken") => reply(500, "nope"),
            _ => reply(200, ""),
        },
    );
    let scratch = Scratch::new("engine");
    let engine = Engine::answering(scratch.0.join("home"), server.port);
    assert_eq!(engine.device_id().await.unwrap(), PC_ID);
    engine.configure(100, 0).await.unwrap();
    engine
        .set_nas(NAS_ID, "tcp://192.168.1.15:22000", true)
        .await
        .unwrap();
    let folder = Folder {
        id: "f-2".into(),
        label: "Photos".into(),
        path: scratch.0.join("Photos"),
        kind: "receiveonly",
        device: NAS_ID.into(),
        paused: false,
    };
    engine.put_folder(&folder).await.unwrap();
    assert!(scratch.0.join("Photos").is_dir());
    assert_eq!(engine.folders().await.unwrap(), ["f-1"]);
    assert_eq!(engine.devices().await.unwrap(), [PC_ID, NAS_ID]);
    assert_eq!(engine.folder_status("f-1").await.unwrap().need_bytes, 4);
    assert!(engine.connected(NAS_ID).await);
    assert!(!engine.connected(PC_ID).await);
    assert_eq!(engine.events(0).await.unwrap().len(), 1);
    engine.remove_folder("f-1").await.unwrap();
    engine.remove_device(NAS_ID).await.unwrap();
    assert!(engine.remove_folder("broken").await.is_err());

    let seen = server.seen.lock().unwrap().clone();
    assert!(seen.iter().all(|request| request
        .header("x-api-key")
        .is_some_and(|key| key.len() == 32)));
    let options = seen
        .iter()
        .find(|request| request.method == "PATCH")
        .unwrap();
    assert!(options.body.contains(r#""relaysEnabled":false"#));
    assert!(options.body.contains(r#""maxSendKbps":100"#));
    let nas = seen
        .iter()
        .find(|request| {
            request.path == format!("/rest/config/devices/{NAS_ID}") && request.method == "PUT"
        })
        .unwrap();
    assert!(nas.body.contains(r#""paused":true"#));
    assert!(nas.body.contains("tcp://192.168.1.15:22000"));
}

#[tokio::test]
async fn an_engine_left_running_is_asked_to_shut_down_before_a_new_one_starts() {
    let server = http(
        |request| match (request.method.as_str(), request.path.as_str()) {
            ("POST", "/rest/system/shutdown") => reply(200, ""),
            _ => reply(404, ""),
        },
    );
    let scratch = Scratch::new("engine-left");
    let engine = Engine::new(PathBuf::from("missing"), scratch.0.join("home")).unwrap();

    engine.stop_left_running().await;
    assert!(server.seen.lock().unwrap().is_empty());

    fs::write(scratch.0.join("home/api-port"), server.port.to_string()).unwrap();
    engine.stop_left_running().await;
    let seen = server.seen.lock().unwrap().clone();
    assert_eq!(seen[0].path, "/rest/system/shutdown");
    assert!(seen[0]
        .header("x-api-key")
        .is_some_and(|key| key.len() == 32));
    assert!(!scratch.0.join("home/api-port").exists());
}

#[tokio::test]
async fn the_output_of_the_engine_goes_to_its_log() {
    let scratch = Scratch::new("engine-log");
    let engine = Engine::new(PathBuf::from("missing"), scratch.0.join("home")).unwrap();
    let _ = engine.output();
    assert!(scratch.0.join("home/engine.log").is_file());
}

#[tokio::test]
async fn an_engine_that_was_stopped_stays_stopped() {
    let scratch = Scratch::new("engine");
    let engine = Engine::new(PathBuf::from("missing"), scratch.0.join("home")).unwrap();
    engine.stop();
    tokio::time::timeout(Duration::from_secs(5), engine.clone().supervise())
        .await
        .unwrap();
    assert!(!engine.is_running());
}

#[tokio::test]
#[ignore = "needs the Syncthing binary"]
async fn real_engine_starts_local_only_and_takes_the_nas_and_a_folder() {
    const NAS: &str = "MFZWI3D-BONSGYC-YLTMRWG-C43ENR5-QXGZDMM-FZWI3DP-BONSGYY-LTMRWAD";
    let scratch = Scratch::new("real-engine");
    let engine = Engine::new(program(), scratch.0.join("home")).unwrap();
    assert!(engine.is_installed());
    tokio::spawn(engine.clone().supervise());
    tokio::time::timeout(Duration::from_secs(60), engine.started.notified())
        .await
        .unwrap();
    let own = engine.device_id().await.unwrap();
    engine.configure(0, 0).await.unwrap();
    engine
        .set_nas(NAS, "tcp://127.0.0.1:22999", false)
        .await
        .unwrap();
    let folder = Folder {
        id: "abcde-12345".into(),
        label: "Docs".into(),
        path: scratch.0.join("Docs"),
        kind: "receiveonly",
        device: NAS.into(),
        paused: false,
    };
    engine.put_folder(&folder).await.unwrap();
    assert_eq!(engine.folders().await.unwrap(), ["abcde-12345"]);
    assert!(engine.devices().await.unwrap().contains(&own));
    let options = engine.get("/rest/config/options").await.unwrap();
    assert_eq!(options["relaysEnabled"], false);
    assert_eq!(options["globalAnnounceEnabled"], false);
    assert_eq!(options["localAnnounceEnabled"], false);
    assert!(options["listenAddresses"][0]
        .as_str()
        .unwrap()
        .starts_with("tcp://127.0.0.1:"));
    assert!(!engine.connected(NAS).await);
    assert!(engine.folder_status("abcde-12345").await.is_ok());
    engine.remove_folder("abcde-12345").await.unwrap();
    engine.stop();
}
