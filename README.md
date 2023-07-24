# CheckMate

*CheckMate* is an application used for monitoring output of custom commands on your PC. The commands can be any valid commands runnable by the operating system. *CheckMate* works in a client-server fashion. The server allows clients to connect to it via TCP and inform about their statuses. Each client periodically runs a command, analyzes its output and detects errors. The status of the command is sent to the server, which accumulates statuses from all clients. The server can be queried for known errors at any time.



# Usage
An example of a *CheckMate* server working with three clients. We first create a simple script, so we have a command to run. By default, first line printed by the command is considered as an error. If command doesn't print anything to its stdout, it's considered to have succeeded. There are also different modes, such as checking the command's error code. See `-m` argument in help message for more details.
```bash
$ echo '#!/bin/sh' > check_dir.sh
$ echo '[ $(find "$1" -maxdepth 1 | wc -l) -gt 30 ] && echo "Too many files $2"' >> check_dir.sh
$ chmod +x check_dir.sh
```

Clients periodically (by default every second) check, whether selected directories contain an acceptable number of files. The server accumulates statuses from all clients. For every client the status is either a success or an error string, such as `Too many files in home directory`.
```bash
# Start client instances in background
$ check_mate_client watch ./check_dir.sh $HOME           "in home directory"      2>/dev/null &
$ check_mate_client watch ./check_dir.sh $HOME/Downloads "in downloads directory" 2>/dev/null &
$ check_mate_client watch ./check_dir.sh $HOME/Desktop   "on deskop"              2>/dev/null &

# Start the server
$ check_mate_server
```

At any time we can ask the server about current statuses of the clients. Example output for when there are too many files in `$HOME` and `$HOME/Downloads` below. Statuses from different clients are separated by an empty line. This output can be used for anything, e.g. to display some diagnostic information on screen.
```bash
$ check_mate_client read
Too many files in home directory

Too many files in downloads directory
```

Let's say that we've just downloaded a couple of files and we want to rerun the check for downloads directory immediately, instead of waiting for the client to rerun its command automatically. We can force a refresh.
```bash
$ check_mate_client refresh_all
```

As the name suggests, this will refresh all of the clients. Clients can also be individually refreshed by name. But first they have to be named. Just add `-n` argument to the client definition. Note that additional `--` argument is also neccessary to serve as a separator between command arguments and *CheckMate* arguments.
```bash
$ check_mate_client watch check_dir.sh $HOME/Downloads "in downloads directory" -- -n DownloadsChecker
```

Then the individual client can be refreshed.
```bash
$ check_mate_client refresh DownloadsChecker
```

For a complete list of features, like configuring command interval, TCP port used for communication, format of status reporting and more, refer to the help messages for client and server binaries.
```bash
$ check_mate_client -h
$ check_mate_server -h
```



# Installing
## Arch Linux:
```bash
yay -S check_mate-bin
```

## Windows:
Currently unsupported.

## Cargo:
Currently unsupported.



# Building from source
Standard building procedure for Rust projects:
1. Install and configure Rust environment
2. Download the repository
3. Enter the repository
4. Call `cargo build --release`
5. Compiled binaries will be in `target/release` directory



# TODO
1. Add support for Windows.
2. Distribute Windows releases on Chocolatey.
3. Publish on crates.io.
4. Improve integration tests. There are some sleeps.
5. Improve logs in server console. Currently they are quite incomplete. Also, multiline errors should be trimmed to one line.
