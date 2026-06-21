use async_channel::Sender;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;

pub fn socket_path() -> String {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        format!("{}/autoclicker.sock", runtime_dir)
    } else {
        "/tmp/autoclicker.sock".to_string()
    }
}

/// Called by CLI subcommands (toggle/start/stop) to talk to the running GUI.
pub fn send_command(cmd: &str) -> Result<(), String> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|_| {
        "Cannot connect to autoclicker — is the GUI running?\n\
Start it first with:  autoclicker"
.to_string()
    })?;

    stream
    .write_all(cmd.as_bytes())
    .map_err(|e| format!("Failed to send command: {}", e))?;

    let mut response = String::new();
    stream
    .read_to_string(&mut response)
    .map_err(|e| format!("Failed to read response: {}", e))?;

    if response.starts_with("OK") {
        Ok(())
    } else {
        Err(response)
    }
}

/// Spawn a background thread that listens on a Unix socket for hotkey commands.
/// Commands are forwarded to the GUI via an async channel.
pub fn start_listener(tx: Sender<String>) {
    let path = socket_path();

    // Clean up any stale socket from a previous crash
    let _ = std::fs::remove_file(&path);

    thread::spawn(move || {
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
                  Err(e) => {
                      eprintln!("Failed to bind IPC socket at {}: {}", path, e);
                      return;
                  }
        };

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buf = [0u8; 64];
                    let n = match stream.read(&mut buf) {
                        Ok(0) => continue, // connection closed
                  Ok(n) => n,
                  Err(_) => continue,
                    };

                    let cmd = String::from_utf8_lossy(&buf[..n]).trim().to_string();

                    let response = match cmd.as_str() {
                        "toggle" | "start" | "stop" => {
                            let _ = tx.send_blocking(cmd.clone());
                            "OK\n".to_string()
                        }
                        _ => "ERROR: unknown command\n".to_string(),
                    };

                    let _ = stream.write_all(response.as_bytes());
                }
                Err(_) => continue,
            }
        }
    });
}
