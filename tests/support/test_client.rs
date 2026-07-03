#![allow(dead_code)]

use std::process::{Command, Child};
use std::thread;
use std::time::Duration;

pub struct TestClient {
    child: Child
}

impl TestClient {
    pub fn start(server: &str, mode: &str, target: Option<&str>) -> Self {
        let mut cmd = Command::new("./../target/debug/s5t");
        cmd
            .arg("--server").arg(server)
            .arg("--mode").arg(mode);

        if let Some(target) = target {
            cmd.arg("--target").arg(target);
        }

        let child: Child = cmd.spawn().unwrap();

        thread::sleep(Duration::from_millis(200));

        Self { child }
    }

    pub fn run(server: &str, target: &str, auth: Option<(String, String)>, data: Option<&str>, headers: Option<&str>, xor: Option<u8>) -> String {
        let mut cmd = Command::new("./../target/debug/s5t");
        cmd
            .arg("--server").arg(server)
            .arg("--target").arg(target);

        if let Some((user, pass)) = &auth {
            cmd.arg("--auth").arg(format!("{user}:{pass}"));
        }

        if let Some(data) = &data {
            cmd.arg("--data").arg(data);
        }
        if let Some(headers) = &headers {
            cmd.arg("--headers").arg(headers);
        }

        if let Some(xor) = &xor {
            cmd.arg("--xor").arg(xor.to_string());
        }

        let output = cmd.output().unwrap();

        String::from_utf8_lossy(&output.stdout).to_string()
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}