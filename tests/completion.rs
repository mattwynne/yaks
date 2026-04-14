use std::time::Duration;

use expectrl::Regex;

fn spawn_bash_with_completion() -> expectrl::Session {
    let yx_bin = env!("CARGO_BIN_EXE_yx");
    let yx_dir = std::path::Path::new(yx_bin)
        .parent()
        .expect("yx binary has a parent directory");

    let completions_script = concat!(env!("CARGO_MANIFEST_DIR"), "/completions/yx.bash");

    let mut session = expectrl::spawn("bash --norc --noprofile").expect("spawn bash");
    session.set_expect_timeout(Some(Duration::from_secs(10)));

    // Wait for the initial prompt
    session.expect(Regex("\\$")).expect("initial prompt");

    // Add yx binary to PATH
    session
        .send_line(&format!("export PATH=\"{}:$PATH\"", yx_dir.display()))
        .expect("set PATH");
    session.expect(Regex("\\$")).expect("prompt after PATH");

    // Disable git hooks
    session
        .send_line("export GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/dev/null'\"")
        .expect("set GIT_CONFIG_PARAMETERS");
    session
        .expect(Regex("\\$"))
        .expect("prompt after git config");

    // Source the completion script
    session
        .send_line(&format!("source \"{}\"", completions_script))
        .expect("source completions");
    session.expect(Regex("\\$")).expect("prompt after sourcing");

    // Show completions immediately on first Tab
    session
        .send_line("bind 'set show-all-if-ambiguous on'")
        .expect("set show-all-if-ambiguous");
    session.expect(Regex("\\$")).expect("prompt after bind");

    session
}

#[test]
fn tab_completion_shows_commands() {
    let mut session = spawn_bash_with_completion();

    // Type "yx " then press Tab to trigger completion
    session.send("yx ").expect("send yx prefix");
    session.send("\t").expect("send tab");

    // Wait for completion output
    std::thread::sleep(Duration::from_secs(2));

    // Press Enter to get back to a prompt so we can read output
    session.send_line("").expect("send enter");

    // Capture everything up to the next prompt
    let output = session.expect(Regex("\\$")).expect("final prompt");
    let output_str = String::from_utf8_lossy(output.as_bytes());

    assert!(
        output_str.contains("add"),
        "Expected 'add' in completion output, got: {}",
        output_str
    );
    assert!(
        output_str.contains("done"),
        "Expected 'done' in completion output, got: {}",
        output_str
    );

    // Internal test helpers must NOT appear
    assert!(
        !output_str.contains("git_checks"),
        "Internal 'git_checks' must not appear, got: {}",
        output_str
    );

    // Clean up: send exit and drop the session
    session.send_line("exit").expect("send exit");
}
