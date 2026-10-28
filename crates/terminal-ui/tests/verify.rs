use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};

use project_rogue_engine_core::{Command as GameCommand, Engine};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_terminal-ui"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

#[test]
fn headless_verification_and_invalid_inputs() {
    let directory =
        std::env::temp_dir().join(format!("project-rogue-verify-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("run.ron");
    let path_arg = path.to_str().unwrap();
    let mut engine = Engine::default();
    engine.start_recording();
    let empty = engine.stop_recording().unwrap().to_ron().unwrap();
    engine.start_recording();
    for command in [
        GameCommand::Wait,
        GameCommand::UseItem(8),
        GameCommand::Restart,
    ] {
        engine.dispatch(command);
    }
    let recorded = engine.stop_recording().unwrap().to_ron().unwrap();
    for (source, count) in [(&empty, 0), (&recorded, 3)] {
        fs::write(&path, source).unwrap();
        let output = run(&["--verify", path_arg]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!(
                "Verification succeeded: {count} commands reproduced identical results, snapshots, and events.\n"
            )
        );
        assert!(output.stderr.is_empty());
    }

    for (source, error) in [
        ("not RON".to_owned(), "replay parse failed"),
        (
            recorded.replace("version: 1", "version: 99"),
            "unsupported replay version",
        ),
        (
            recorded.replace("map_width: 40", "map_width: 0"),
            "map width and height",
        ),
        (
            recorded.replace("actor_id: \"core:rat\"", "actor_id: \"core:missing\""),
            "unknown actor",
        ),
        (
            recorded.replace("max_health: 10", "max_health: 0"),
            "zero health",
        ),
        (
            recorded.replace("item_id: \"core:potion\"", "item_id: \"core:missing\""),
            "unknown item",
        ),
    ] {
        assert_ne!(source, recorded, "malformed fixture must change replay");
        fs::write(&path, source).unwrap();
        let output = run(&["--verify", path_arg]);
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(error), "{stderr}");
        assert!(!stderr.contains("panicked"), "{stderr}");
        assert!(output.stdout.is_empty());
    }
    fs::remove_dir_all(&directory).unwrap();
    assert!(!Path::new(path_arg).exists());
    let output = run(&["--verify", path_arg]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn rejects_missing_extra_and_unknown_arguments_before_terminal_init() {
    for flag in ["--record", "--replay", "--verify"] {
        for args in [
            vec![flag],
            vec![flag, ""],
            vec![flag, "--verify"],
            vec![flag, "run.ron", "extra"],
        ] {
            let output = run(&args);
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(
                stderr.contains("requires a path") || stderr.contains("unexpected argument"),
                "{stderr}"
            );
            assert!(!stderr.contains("panicked"), "{stderr}");
        }
    }
    let output = run(&["--unknown"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unknown argument")
    );
}
