use crate::helpers::paths::get_cargo_bin;

pub struct Subprocess {
    name: String,
    child: Option<std::process::Child>,
}

impl Subprocess {
    pub fn start_server(name: &str, port: u16) -> Subprocess {
        let server_bin = get_cargo_bin("check_mate_server").expect("Server binary should be found");

        let child = std::process::Command::new(server_bin)
            .arg("-p")
            .arg(port.to_string())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Server should start");
        std::thread::sleep(std::time::Duration::from_millis(50));

        Subprocess {
            child: Some(child),
            name: name.to_owned(),
        }
    }

    pub fn start_client(name: &str, port: u16, args: &[&str]) -> Subprocess {
        let client_bin = get_cargo_bin("check_mate_client").expect("Client binary should be found");

        let port = port.to_string();
        let mut port_args = vec!["-p", &port];
        if args.len() > 0 && args[0] == "watch" && !args.contains(&"--") {
            port_args.insert(0, "--");
        }

        let child = std::process::Command::new(client_bin)
            .args(args)
            .args(port_args)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Client should start");

        Subprocess {
            child: Some(child),
            name: name.to_owned(),
        }
    }

    pub fn wait_and_get_output(&mut self, require_success: bool) -> String {
        let out = self
            .child
            .take()
            .expect(&format!("{} should not be moved out", self.name))
            .wait_with_output()
            .unwrap_or_else(|_| panic!("{} should correctly provide output", self.name));
        if require_success {
            assert!(out.status.success(), "{} should return success", self.name);
        }
        String::from_utf8(out.stdout).expect("Server stdout should be available")
    }

    pub fn kill(&mut self) {
        match &mut self.child {
            Some(child) => {
                if child.kill().is_err() {
                    panic!("{} shoud be killable", self.name);
                }
            }
            None => panic!("{} has already been killed", self.name),
        }
    }
}

impl Drop for Subprocess {
    fn drop(&mut self) {
        if self.child.is_some() {
            self.kill();
        }
    }
}
