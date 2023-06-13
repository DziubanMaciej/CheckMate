use crate::test_helpers::get_child_output;
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

    pub fn start_server(port: u16) -> std::process::Child {
        let server_bin = get_cargo_bin("check_mate_server").expect("Server binary should be found");

        let server = std::process::Command::new(server_bin)
            .arg("-p")
            .arg(port.to_string())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Server should start");
        std::thread::sleep(std::time::Duration::from_millis(50));

        server
    }

    pub fn start_client(port: u16, args: &[&str]) -> std::process::Child {
        let client_bin = get_cargo_bin("check_mate_client").expect("Client binary should be found");

        let port = port.to_string();
        let mut port_args = vec!["-p", &port];
        if args.len() > 0 && args[0] == "watch" && !args.contains(&"--") {
            port_args.insert(0, "--");
        }

        let client = std::process::Command::new(client_bin)
            .args(args)
            .args(port_args)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Client should start");
        client
    }

    pub fn get_child_output(child_name: &str, child: std::process::Child) -> String {
        let out = child
            .wait_with_output()
            .unwrap_or_else(|_| panic!("{child_name} should correctly provide output"));
        assert!(out.status.success());
        String::from_utf8(out.stdout).expect("Server stdout should be available")
    }
}

#[test]
fn server_closes_after_abort_command() {
    let port = test_helpers::get_port_number();
    let server = test_helpers::start_server(port);
    let client = test_helpers::start_client(port, &["abort"]);

    assert!(get_child_output("client", client).is_empty());

    let server_out = get_child_output("server", server);
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
    let server = test_helpers::start_server(port);
    let client = test_helpers::start_client(port, &["abort", "-n", "Aborter"]);

    assert!(get_child_output("client", client).is_empty());

    let server_out = get_child_output("server", server);
    server_out
        .lines()
        .seek("Name set to Aborter")
        .seek("Received abort command");
}

#[test]
fn read_messages_with_single_client_works() {
    let port = test_helpers::get_port_number();
    let mut server = test_helpers::start_server(port);
    let mut client_watcher = test_helpers::start_client(
        port,
        &[
            "watch",
            "echo",
            "\n\n\n \nsome nice error\nsecond line ignored",
        ],
    );

    std::thread::sleep(std::time::Duration::from_millis(50));
    let client_reader = test_helpers::start_client(port, &["read"]);

    let client_reader_out = get_child_output("client_reader", client_reader);
    server.kill().expect("Server should be killable");
    client_watcher.kill().expect("Client should be killable");

    assert_eq!(client_reader_out, "some nice error\n");
}

#[ignore]
#[test]
fn read_messages_with_multiple_clients_works() {
    let port = test_helpers::get_port_number();
    let mut server = test_helpers::start_server(port);
    let mut client_watcher1 = test_helpers::start_client(
        port,
        &[
            "watch",
            "echo",
            "\n\n\n \nsome nice error\nsecond line ignored",
        ],
    );
    let mut client_watcher2 = test_helpers::start_client(
        port,
        &[
            "watch",
            "echo",
            "\n\n\n \nsome other error\nsecond line ignored",
        ],
    );

    std::thread::sleep(std::time::Duration::from_millis(50));
    let client_reader = test_helpers::start_client(port, &["read"]);

    let client_reader_out = get_child_output("client_reader", client_reader);
    server.kill().expect("Server should be killable");
    client_watcher1.kill().expect("Client should be killable");
    client_watcher2.kill().expect("Client should be killable");

    assert_eq!(client_reader_out, "some nice error\n");
}
