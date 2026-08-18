#![allow(dead_code)]

use std::process::{Child, Command};

pub struct TestClient {
    child: Child
}

impl TestClient {
    pub fn start(server: &str, target: &str, auth: Option<(String, String)>, data: Option<&str>, headers: Option<&str>, xor: Option<u8>) -> String {
        let mut cmd = Command::new("./../target/debug/s5t");
        cmd
            .arg("--mode").arg("socks5")
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