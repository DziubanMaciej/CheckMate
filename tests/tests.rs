mod helpers;
use helpers::collection_counter::CountableCollection;
use helpers::port::get_port_number;
use helpers::seekable::Seekable;
use helpers::subprocess::Subprocess;

#[test]
fn server_closes_after_abort_command() {
    let port = get_port_number();
    let mut server = Subprocess::start_server("server", port, &[]);
    let mut client = Subprocess::start_client("client", port, &["abort"]);

    assert!(client.wait_and_get_output(true).is_empty());
    let server_out = server.wait_and_get_output(true);
    server_out.lines().seek("Received abort command");
}

#[test]
fn server_logs_client_name() {
    let port = get_port_number();
    let mut server = Subprocess::start_server("server", port, &[]);
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
    let _server = Subprocess::start_server("server", port, &[]);
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
fn client_reconnects_when_server_restarts() {
    // TODO this test may fail sporadically due to the sleep being to short. I should make it smarter...

    let port = get_port_number();
    let _client_watcher = Subprocess::start_client(
        "client_watcher",
        port,
        &["watch", "echo", "My fail", "--", "-c", "0", "-w", "0"],
    );

    for i in 0..2 {
        let mut server = Subprocess::start_server(&format!("server{i}"), port, &[]);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let server_out = server.kill_and_get_output();
        server_out
            .lines()
            .seek("Client <Unknown> has error: My fail");
    }
}

#[test]
fn read_messages_with_names_works() {
    let port = get_port_number();
    let _server = Subprocess::start_server("server", port, &[]);
    let _client_watcher1 =
        Subprocess::start_client("client_watcher1", port, &["watch", "echo", "error1"]);
    let _client_watcher2 = Subprocess::start_client(
        "client_watcher2",
        port,
        &["watch", "echo", "error2", "--", "-n", "client2"],
    );

    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut client_reader = Subprocess::start_client("client_reader", port, &["read", "-i", "1"]);
    let client_reader_out = client_reader.wait_and_get_output(true);

    client_reader_out
        .lines()
        .to_collection_counter()
        .contains("<Unknown>: error1", 1)
        .contains("client2: error2", 1)
        .nothing_else();
}

#[test]
fn read_messages_with_multiple_clients_works() {
    let port = get_port_number();
    let _server = Subprocess::start_server("server", port, &[]);
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

    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut client_reader = Subprocess::start_client("client_reader", port, &["read"]);
    let client_reader_out = client_reader.wait_and_get_output(true);
    client_reader_out
        .lines()
        .to_collection_counter()
        .contains("some nice error", 1)
        .contains("some other error", 1)
        .nothing_else();
}

#[test]
fn refreshing_by_name_works() {
    let port = get_port_number();

    // Start server with log_every_status flag, so we'll be able to see updates of Watcher2 after refreshing it.
    let mut server = Subprocess::start_server("server", port, &["-e", "1"]);

    // Two watchers are working with very high watch interval, meaning they should only
    // send status to server once.
    let mut _client_watcher1 = Subprocess::start_client(
        "client_watcher1",
        port,
        &[
            "watch", "echo", "Error", "--", "-n", "Watcher1", "-w", "5000",
        ],
    );
    let mut _client_watcher2 = Subprocess::start_client(
        "client_watcher2",
        port,
        &[
            "watch", "echo", "Error", "--", "-n", "Watcher2", "-w", "5000",
        ],
    );
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Refresh one of the watchers to cause the second status report to server
    let mut client_refresher =
        Subprocess::start_client("client_refresher", port, &["refresh", "Watcher2"]);
    client_refresher.wait_and_get_output(true);
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Server should see only one report from Watcher1, but two reports from Watcher2, since
    // it has been explicitly refreshed.
    _client_watcher1.kill_and_get_output();
    _client_watcher2.kill_and_get_output();
    let server_out = server.kill_and_get_output();
    server_out
        .lines()
        .to_collection_counter()
        .contains("Name set to Watcher1", 1)
        .contains("Name set to Watcher2", 1)
        .contains("Client Watcher1 has error: Error", 1)
        .contains("Client Watcher2 has error: Error", 2)
        .nothing_else();
}

#[test]
fn refreshing_all_works() {
    let port = get_port_number();

    // Start server with log_every_status flag, so we'll be able to see updates of watchers after refreshing them.
    let mut server = Subprocess::start_server("server", port, &["-e", "1"]);

    // Two watchers are working with very high watch interval, meaning they should only
    // send status to server once.
    let mut _client_watcher1 = Subprocess::start_client(
        "client_watcher1",
        port,
        &[
            "watch", "echo", "Error", "--", "-n", "Watcher1", "-w", "5000",
        ],
    );
    let mut _client_watcher2 = Subprocess::start_client(
        "client_watcher2",
        port,
        &[
            "watch", "echo", "Error", "--", "-n", "Watcher2", "-w", "5000",
        ],
    );
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Refresh both watchers
    let mut client_refresher = Subprocess::start_client("client_refresher", port, &["refresh_all"]);
    client_refresher.wait_and_get_output(true);
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Server should see only one report from Watcher1, but two reports from Watcher2, since
    // it has been explicitly refreshed.
    _client_watcher1.kill_and_get_output();
    _client_watcher2.kill_and_get_output();
    let server_out = server.kill_and_get_output();
    server_out
        .lines()
        .to_collection_counter()
        .contains("Name set to Watcher1", 1)
        .contains("Name set to Watcher2", 1)
        .contains("Client Watcher1 has error: Error", 2)
        .contains("Client Watcher2 has error: Error", 2)
        .nothing_else();
}
