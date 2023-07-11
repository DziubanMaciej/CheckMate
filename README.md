# CheckMate

*CheckMate* is a client-server application used for monitoring output of custom commands on your PC. The commands can be any valid commands runnable in native OS shell, which makes this a very flexible solution.

The server allows multiple clients to connect to it and set their statuses. Each client will periodically run a command and analyze its stdout. The first non-empty line in command's output is considered as an error status. If stdout is empty, it is considered as a success status. In both cases, a notification is sent to the server, which accumulates statuses from all clients. Then it can be asked to return all the statuses acquired from its clients.

# Usage
An example of a *CheckMate* server working with three clients. Clients are periodically (by default every second) checking whether some directories contain an acceptable number of files
```bash
$ check_mate_server &
$ check_mate_client watch_shell '[ $(find $HOME -maxdepth 1 | wc -l) -gt 30 ] && echo "Too many files in home"' &
$ check_mate_client watch_shell '[ $(find $HOME/Downloads -maxdepth 1 | wc -l) -gt 20 ] && echo "Too many files in downloads directory"' &
$ check_mate_client watch_shell '[ $(find $HOME/Deskop -maxdepth 1 | wc -l) -gt 10 ] && echo "Too many files on the desktop"' &
```

At any time we can ask the server about current statuses of the clients. Example output for when there are too many files in `$HOME` and `$HOME/Downloads`:
```bash
$ check_mate_client read
Too many files in home
Too many files in downloads directory
```

This output can be used to display some diagnostic information on screen. Let's say that we just installed some new program and we want to rerun the check for `$HOME` instead of waiting for the interval to elapse. We can schedule a refresh:
```bash
$ check_mate_client refresh_all
```

This will refresh all clients. Clients can also be refreshed by name. But first they have to be named. Just add `-n` argument to the client definition:
```bash
$ check_mate_client watch_shell '[ $(find $HOME -maxdepth 1 | wc -l) -gt 30 ] && echo "Too many files in home"' -n HomeChecker &
```

Then the individual client can be refreshed:
```bash
$ check_mate_client refresh HomeChecker
```

For more features, like configuring command interval, TCP port used for communication, format of status reporting and more, refer to the help messages for client and server binaries:
```bash
$ check_mate_client -h
$ check_mate_server -h
```

# Installing


# Building from source
Standard building procedure for Rust projects:
    1. Install and configure Rust environment
    2. Download the repository
    3. Enter the repository
    4. Call `cargo build`
    5. Compiled binaries will be in `target/release` directory




# TODO
Add watch_shell
Add release
Think of a name in crates.io. CheckMate is taken
Learn how to install crates
Upload a crate
Make better integration tests
