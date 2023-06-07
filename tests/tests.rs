use std::str::from_utf8;

mod test_helpers {
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
}

#[test]
fn server_closes_after_abort_command() {
    let client_bin =
        test_helpers::get_cargo_bin("check_mate_client").expect("Client binary should be found");
    let server_bin =
        test_helpers::get_cargo_bin("check_mate_server").expect("Server binary should be found");

    let server = std::process::Command::new(server_bin)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Server should start");
    std::thread::sleep(std::time::Duration::from_millis(50));
    let client = std::process::Command::new(client_bin)
        .arg("abort")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Client should start");

    let server_out = server
        .wait_with_output()
        .expect("Server should correctly provide output");
    assert!(server_out.status.success());

    let client_out = client
        .wait_with_output()
        .expect("Client should correctly provide output");
    assert!(client_out.status.success());

    let out = String::from_utf8(server_out.stdout).expect("Server stdout should be available");
    let abort_lines = out
        .lines()
        .filter(|line| *line == "Received abort command")
        .count();
    assert_eq!(
        abort_lines, 1,
        "Server should print information about received abort command"
    );
}
