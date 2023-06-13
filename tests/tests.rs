mod helpers;
use helpers::port::get_port_number;
use helpers::seekable::Seekable;
use helpers::subprocess::Subprocess;

#[test]
fn server_closes_after_abort_command() {
    let port = get_port_number();
    let mut server = Subprocess::start_server("server", port);
    let mut client = Subprocess::start_client("client", port, &["abort"]);

    assert!(client.wait_and_get_output(true).is_empty());
    let server_out = server.wait_and_get_output(true);
    server_out.lines().seek("Received abort command");
}

#[test]
fn server_logs_client_name() {
    let port = get_port_number();
    let mut server = Subprocess::start_server("server", port);
    let mut client = Subprocess::start_client("client", port, &["abort", "-n", "Aborter"]);

    assert!(client.wait_and_get_output(true).is_empty());

    let server_out = server.wait_and_get_output(true);
    server_out
        .lines()
        .seek("Name set to Aborter")
        .seek("Received abort command");
}

#[test]
fn read_messages_with_single_client_works() {
    let port = get_port_number();
    let _server = Subprocess::start_server("server", port);
    let _client_watcher = Subprocess::start_client(
        "client_watcher",
        port,
        &[
            "watch",
            "echo",
            "\n\n\n \nsome nice error\nsecond line ignored",
        ],
    );

    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut client_reader = Subprocess::start_client("client_reader", port, &["read"]);
    let client_reader_out = client_reader.wait_and_get_output(true);
    assert_eq!(client_reader_out, "some nice error\n");
}

#[test]
fn read_messages_with_multiple_clients_works() {
    let port = get_port_number();
    let _server = Subprocess::start_server("server", port);
    let _client_watcher1 = Subprocess::start_client(
        "client_watcher1",
        port,
        &[
            "watch",
            "echo",
            "\n\n\n \nsome nice error\nsecond line ignored",
        ],
    );
    let _client_watcher2 = Subprocess::start_client(
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
    let mut client_reader = Subprocess::start_client("client_reader", port, &["read"]);
    let client_reader_out = client_reader.wait_and_get_output(true);

    let lines: Vec<&str> = client_reader_out.lines().collect();
    assert!(lines.contains(&"some nice error")); // TODO this does not check that no other lines are printed.
    assert!(lines.contains(&"some other error"));
}
