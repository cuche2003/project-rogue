#![forbid(unsafe_code)]

use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn main() -> io::Result<()> {
    let terminal_ui = std::env::current_exe()?
        .with_file_name(format!("terminal-ui{}", std::env::consts::EXE_SUFFIX));
    let mut child = Command::new(terminal_ui)
        .arg("--agent")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut input = child.stdin.take().expect("agent stdin is piped");
    let output = child.stdout.take().expect("agent stdout is piped");
    let mut responses = BufReader::new(output);
    let mut stdout = io::stdout().lock();

    for line in BufReader::new(io::stdin().lock()).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        writeln!(input, "{line}")?;
        input.flush()?;

        let mut response = String::new();
        responses.read_line(&mut response)?;
        stdout.write_all(response.as_bytes())?;
        stdout.flush()?;

        if line.trim() == "quit" {
            break;
        }
    }
    drop(input);
    child.wait()?;
    Ok(())
}
