use std::fmt::Display;

mod test_helpers {
    use std::sync::atomic::{AtomicU16, Ordering};

    static PORT_NUMBER: AtomicU16 = AtomicU16::new(check_mate_common::DEFAULT_PORT);

    pub fn get_port_number() -> u16 {
        PORT_NUMBER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn get_cargo_bin(name: &str) -> Option<std::path::PathBuf> {
        fn target_dir() -> std::path::PathBuf {
            let mut path = std::env::current_exe().unwrap();
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path
        }

        let env_var = format!("CARGO_BIN_EXE_{}", name);
        let path = std::env::var_os(&env_var)
            .map(|p| p.into())
            .unwrap_or_else(|| {
                target_dir().join(format!("{}{}", name, std::env::consts::EXE_SUFFIX))
            });
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    pub struct Subprocess {
        name: String,
        child: Option<std::process::Child>,
    }

    impl Subprocess {
        pub fn start_server(name: &str, port: u16) -> Subprocess {
            let server_bin =
                get_cargo_bin("check_mate_server").expect("Server binary should be found");

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
            let client_bin =
                get_cargo_bin("check_mate_client").expect("Client binary should be found");

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
    }

    impl Drop for Subprocess {
        fn drop(&mut self) {
            if let Some(ref mut child) = &mut self.child {
                if child.kill().is_err() {
                    panic!("{} shoud be killable", self.name);
                }
            }
        }
    }
}

#[test]
fn server_closes_after_abort_command() {
    let port = test_helpers::get_port_number();
    let mut server = test_helpers::Subprocess::start_server("server", port);
    let mut client = test_helpers::Subprocess::start_client("client", port, &["abort"]);

    assert!(client.wait_and_get_output(true).is_empty());
    let server_out = server.wait_and_get_output(true);
    server_out.lines().seek("Received abort command");
}

trait Seekable<T> {
    fn seek(&mut self, arg: T) -> &mut Self;
}

impl<T> Seekable<<T as Iterator>::Item> for T
where
    T: Iterator,
    <T as Iterator>::Item: Display + PartialEq,
{
    fn seek(&mut self, arg: <T as Iterator>::Item) -> &mut Self {
        loop {
            let element = match self.next() {
                Some(x) => x,
                None => panic!("Could not find element \"{arg}\""),
            };
            if element == arg {
                break;
            }
        }
        self
    }
}

#[test]
fn server_logs_client_name() {
    let port = test_helpers::get_port_number();
    let mut server = test_helpers::Subprocess::start_server("server", port);
    let mut client =
        test_helpers::Subprocess::start_client("client", port, &["abort", "-n", "Aborter"]);

    assert!(client.wait_and_get_output(true).is_empty());

    let server_out = server.wait_and_get_output(true);
    server_out
        .lines()
        .seek("Name set to Aborter")
        .seek("Received abort command");
}

#[test]
fn read_messages_with_single_client_works() {
    let port = test_helpers::get_port_number();
    let _server = test_helpers::Subprocess::start_server("server", port);
    let _client_watcher = test_helpers::Subprocess::start_client(
        "client_watcher",
        port,
        &[
            "watch",
            "echo",
            "\n\n\n \nsome nice error\nsecond line ignored",
        ],
    );

    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut client_reader =
        test_helpers::Subprocess::start_client("client_reader", port, &["read"]);
    let client_reader_out = client_reader.wait_and_get_output(true);
    assert_eq!(client_reader_out, "some nice error\n");
}

#[test]
fn read_messages_with_multiple_clients_works() {
    let port = test_helpers::get_port_number();
    let _server = test_helpers::Subprocess::start_server("server", port);
    let _client_watcher1 = test_helpers::Subprocess::start_client(
        "client_watcher1",
        port,
        &[
            "watch",
            "echo",
            "\n\n\n \nsome nice error\nsecond line ignored",
        ],
    );
    let _client_watcher2 = test_helpers::Subprocess::start_client(
        "client_watcher2",
        port,
        &[
            "watch",
            "echo",
            "\n\n\n \nsome other error\nsecond line ignored",
        ],
    );

    println!("PORT: {port}");
    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut client_reader =
        test_helpers::Subprocess::start_client("client_reader", port, &["read"]);
    let client_reader_out = client_reader.wait_and_get_output(true);

    let lines : Vec<&str> = client_reader_out.lines().collect();
    assert!(lines.contains(&"some nice error")); // TODO this does not check that no other lines are printed.
    assert!(lines.contains(&"some other error"));
}
