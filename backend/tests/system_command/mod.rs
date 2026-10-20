use super::*;

fn fixture(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "system_command::tests::command_fixture",
            "--nocapture",
        ])
        .env("NAS_FINDER_COMMAND_FIXTURE", mode);
    command
}

#[test]
fn command_fixture() {
    use std::io::Write;
    match std::env::var("NAS_FINDER_COMMAND_FIXTURE").as_deref() {
        Ok("sleep") => thread::sleep(Duration::from_secs(5)),
        Ok("large") => std::io::stdout().write_all(&vec![b'x'; 65537]).unwrap(),
        Ok("fail") => std::process::exit(1),
        Ok("success") => println!("interface-output"),
        _ => {}
    }
}

#[test]
fn collects_successful_output_and_rejects_failed_or_oversized_output() {
    let bytes = output(&mut fixture("success"), Duration::from_secs(2)).unwrap();
    assert!(String::from_utf8(bytes)
        .unwrap()
        .contains("interface-output"));
    assert!(output(&mut fixture("fail"), Duration::from_secs(2)).is_none());
    assert!(output(&mut fixture("large"), Duration::from_secs(2)).is_none());
}

#[test]
fn terminates_a_process_that_exceeds_the_deadline() {
    let started = Instant::now();
    assert!(output(&mut fixture("sleep"), Duration::from_millis(100)).is_none());
    assert!(started.elapsed() < Duration::from_secs(2));
}
