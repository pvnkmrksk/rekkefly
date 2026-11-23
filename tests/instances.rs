use rekkefly::{
    config::Config,
    engine::{Engine, control_command},
    native_mailbox::Mailbox,
    tracker::Measurement,
};
use serde_json::{Value, json};
use std::{
    fs,
    net::UdpSocket,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
fn sender_address() -> String {
    let s = UdpSocket::bind("127.0.0.1:0").unwrap();
    s.local_addr().unwrap().to_string()
}
#[test]
fn simultaneous_instances_send_independent_configured_streams() {
    let a = UdpSocket::bind("127.0.0.1:0").unwrap();
    let b = UdpSocket::bind("127.0.0.1:0").unwrap();
    a.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    b.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut ca = Config::demo();
    ca.output.bind = sender_address();
    ca.output.target = a.local_addr().unwrap().to_string();
    ca.instance = "a".into();
    let mut cb = ca.clone();
    cb.instance = "b".into();
    cb.output.bind = sender_address();
    cb.output.target = b.local_addr().unwrap().to_string();
    assert_ne!(ca.output.bind, cb.output.bind);
    let ea = Engine::start(ca.clone(), PathBuf::from("unused-a.toml"), Some(4)).unwrap();
    let eb = Engine::start(cb.clone(), PathBuf::from("unused-b.toml"), Some(4)).unwrap();
    let mut buffer = [0; 8192];
    let (n, peer) = a.recv_from(&mut buffer).unwrap();
    let ma: Measurement = serde_json::from_slice(&buffer[..n]).unwrap();
    assert_eq!(ma.instance, "a");
    assert_eq!(peer.to_string(), ca.output.bind);
    let (n, peer) = b.recv_from(&mut buffer).unwrap();
    let mb: Measurement = serde_json::from_slice(&buffer[..n]).unwrap();
    assert_eq!(mb.instance, "b");
    assert_eq!(peer.to_string(), cb.output.bind);
    ea.wait();
    eb.wait();
}
#[test]
fn conflicting_sender_port_fails_with_a_clear_error() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut config = Config::demo();
    config.output.bind = socket.local_addr().unwrap().to_string();
    let error = match Engine::start(config, PathBuf::from("unused.toml"), Some(1)) {
        Ok(_) => panic!("Port conflict was accepted"),
        Err(e) => e,
    };
    assert!(format!("{error:#}").contains("unique sender port"));
}
#[test]
fn background_is_saved_and_reloaded_without_global_state() {
    let dir = tempfile::tempdir().unwrap();
    let temp = dir.path().to_path_buf();
    let path = temp.join("rig.toml");
    let mut c = Config::demo();
    c.output.enabled = false;
    c.tracking.background_path = "reference.png".into();
    let engine = Engine::start(c.clone(), path.clone(), None).unwrap();
    engine.demo_background().unwrap();
    let start = std::time::Instant::now();
    while !engine.state().background_loaded && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(engine.state().background_loaded);
    engine.wait();
    assert!(temp.join("reference.png").is_file());
    c.tracking.subtract_background = true;
    let engine = Engine::start(c, path, Some(2)).unwrap();
    let start = std::time::Instant::now();
    while !engine.is_finished() && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(10));
    }
    let state = engine.state();
    assert!(state.error.is_none(), "{:?}", state.error);
    assert!(state.latest.unwrap().measurement.background_subtracted);
    engine.wait();
}
#[test]
fn threshold_and_enable_changes_apply_without_restarting_capture() {
    let mut c = Config::demo();
    c.output.enabled = false;
    let engine = Engine::start(c.clone(), PathBuf::from("unused-live.toml"), None).unwrap();
    let wait = |condition: &dyn Fn(&rekkefly::engine::State) -> bool| {
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            let state = engine.state();
            assert!(state.error.is_none(), "{:?}", state.error);
            if condition(&state) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("Live update was not observed");
    };
    wait(&|s| {
        s.latest
            .as_ref()
            .is_some_and(|v| v.measurement.right.angle_rad.is_some())
    });
    c.tracking.right.threshold = 1.0;
    engine.update(&c).unwrap();
    wait(&|s| {
        s.latest
            .as_ref()
            .is_some_and(|v| v.measurement.right.enabled && v.measurement.right.angle_rad.is_none())
    });
    c.tracking.right.threshold = 0.0;
    engine.update(&c).unwrap();
    wait(&|s| {
        s.latest
            .as_ref()
            .is_some_and(|v| v.measurement.right.angle_rad.is_some())
    });
    c.tracking.right.enabled = false;
    engine.update(&c).unwrap();
    wait(&|s| {
        s.latest.as_ref().is_some_and(|v| {
            !v.measurement.right.enabled && v.measurement.left_minus_right_rad.is_none()
        })
    });
    engine.wait();
}

#[test]
fn video_recording_can_start_stop_and_restart_without_reopening_capture() {
    let directory = tempfile::tempdir().unwrap();
    let mut config = Config::demo();
    config.output.enabled = false;
    config.recording.directory = "recordings".into();
    let engine = Engine::start(config, directory.path().join("rig.toml"), None).unwrap();
    let wait = |predicate: &dyn Fn(&rekkefly::engine::State) -> bool| {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let state = engine.state();
            assert!(state.error.is_none(), "{:?}", state.error);
            if predicate(&state) {
                return state;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for recorder state"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    let before = wait(&|s| s.frames >= 3).frames;
    engine.record(true).unwrap();
    let first = wait(&|s| s.recording.active && s.recording.written >= 4);
    assert!(first.frames > before);
    engine.record(false).unwrap();
    let stopped = wait(&|s| {
        !s.recording.active
            && s.recording.written >= 4
            && s.recording.written == s.recording.accepted
    });
    assert_eq!(stopped.recording.accepted, stopped.recording.written);
    engine.record(true).unwrap();
    let second = wait(&|s| {
        s.recording.active
            && s.recording.written >= 3
            && s.recording.directory != first.recording.directory
    });
    assert!(second.frames > first.frames);
    engine.stop();
    let final_state = wait(&|s| s.finished);
    assert_eq!(
        final_state.recording.accepted,
        final_state.recording.written
    );
    assert!(final_state.recording.error.is_none());
    engine.wait();
    assert_eq!(
        directory
            .path()
            .join("recordings")
            .read_dir()
            .unwrap()
            .count(),
        2
    );
}

#[test]
fn experiment_mailbox_stops_only_its_take_and_proves_finalization() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = Config::demo();
    config.output.enabled = false;
    let engine = Engine::start(config, temp.path().join("rig.toml"), None).unwrap();
    let directory = temp.path().join("control");
    let instance = "00000000000000000000000000000001".to_owned();
    let mut mailbox = Mailbox::new(directory.clone(), instance.clone(), "rekkefly").unwrap();
    let mut sequence = 0;
    let mut call = |command: &str, args: Value| {
        sequence += 1;
        let id = format!("{sequence:032x}");
        let request = directory.join(format!("{id}.request.json"));
        let response = directory.join(format!("{id}.response.json"));
        let expires = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            + 5000;
        fs::write(
            request,
            json!({
                "id": id, "instance": instance, "command": command, "args": args,
                "expiresUtcMilliseconds": expires,
            })
            .to_string(),
        )
        .unwrap();
        mailbox
            .poll(|command, args| {
                control_command(&engine, command, args).map_err(|e| format!("{e:#}"))
            })
            .unwrap();
        let reply: Value = serde_json::from_slice(&fs::read(&response).unwrap()).unwrap();
        fs::remove_file(response).unwrap();
        assert_eq!(reply["ok"], true, "{reply}");
        assert_eq!(reply["instance"], instance);
        assert_eq!(reply["result"]["queued"], command != "status");
        reply["result"].clone()
    };
    let wait = |call: &mut dyn FnMut(&str, Value) -> Value, ready: &dyn Fn(&Value) -> bool| {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let status = call("status", json!({}));
            assert_eq!(status["ended"], false, "{status}");
            assert!(status["error"].is_null(), "{status}");
            if ready(&status) {
                break status;
            }
            assert!(Instant::now() < deadline, "Last status: {status}");
            std::thread::sleep(Duration::from_millis(10));
        }
    };

    call("pause", json!({}));
    wait(&mut call, &|status| status["paused"] == true);
    let root = temp.path().join("take");
    call(
        "record_start",
        json!({"directory": root, "record_options": {"raw": true, "debug": false}, "recording_tuning": {"encoding": "mjpeg"}}),
    );
    let armed = wait(&mut call, &|status| status["recording"]["active"] == true);
    assert_eq!(armed["recording"]["accepted"], 0);
    let take = Path::new(armed["recording"]["directory"].as_str().unwrap());
    assert_eq!(take.parent(), Some(root.as_path()));
    let receipt_path = take.join("completion.json");
    assert!(!receipt_path.exists());

    call("start", json!({}));
    wait(&mut call, &|status| {
        status["recording"]["accepted"].as_u64().unwrap() >= 3
    });
    let wrong_stop = "00000000000000000000000000000002";
    call(
        "record_stop",
        json!({"expected_directory": temp.path().join("other-take"), "stop_request_id": wrong_stop}),
    );
    let still_active = wait(&mut call, &|status| status["command_error"].is_string());
    assert_eq!(still_active["recording"]["active"], true);
    assert!(!receipt_path.exists());

    let stop = "00000000000000000000000000000003";
    call(
        "record_stop",
        json!({"expected_directory": take, "stop_request_id": stop}),
    );
    let finalized = wait(&mut call, &|status| {
        status["recording"]["finalized"] == true
    });
    let receipt: Value = serde_json::from_slice(&fs::read(receipt_path).unwrap()).unwrap();
    assert_eq!(receipt["state"], "complete");
    assert_eq!(receipt["control_stop_id"], stop);
    assert_eq!(receipt["capture_pause_count"], 0);
    assert_eq!(receipt["writersDrained"], true);
    assert_eq!(receipt["accepted"], receipt["written"]);
    assert!(receipt["accepted"].as_u64().unwrap() >= 3);
    assert!(take.join("video-0000.avi").is_file());
    let frames = finalized["frames"].as_u64().unwrap();
    wait(&mut call, &|status| {
        status["frames"].as_u64().unwrap() > frames
    });
    engine.wait();
}
