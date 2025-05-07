# Netter

Netter is a tool for quickly and easily launching servers.

## Table of Contents

* [Future](#future)
* [Features](#features)
* [Documentation](#documentation)
* [Installation](#installation)

## Future

* Desktop client (UI);
* Support for other server types (besides HTTP/HTTPS): WebSocket, **gRPC**, TCP/UDP sockets;

## Features

* Custom Route Definition Language (.rd) (hereinafter - RDL) - a config describing possible routes and the logic for handling requests to these routes;
* Error handling in RDL;
* TLS support;
* Own daemon/service (depending on the OS);
* Ability to integrate Rust plugins into RDL.

## Documentation

### Install

To install the daemon or service, use the `install` command:

```powershell
netter install
```

> [!WARN]
> The service executable (`netter_service`) must be in the same directory as the CLI.

### Service-Start

To start the service or daemon, use the `service-start` command:

```powershell
netter service-start
```

### Service-Stop

To stop the service or daemon, use the `service-stop` command:

```powershell
netter service-stop
```

### Service-Status

To get the status of the service or daemon, use the `service-status` command:

```powershell
netter service-status
```

### Uninstall

To remove the service or daemon, use the `uninstall` command:

```powershell
netter uninstall
```

### Ping

To check the connection with the service, use the `ping` command:

```powershell
netter ping
```

### List

To get a list of running servers (including IDs), use the `list` command:

```powershell
netter list
```

### Start

Start a server using the `start` command and the `--config` flag, which takes the path to the .rd server configuration file:

```powershell
netter start --config path/to/file.rd
```

### Stop

Stop a server using the `stop` command and the `-i` (or `--id`) flag, which takes the server ID:

```powershell
netter stop -i id
```

[Route Definition Language Documentation](RDL_DOCUMENTATION_ru.md)

## Installation

### Windows

**Installation from Release Assets:**

* Download the archive for Windows;
* Unpack it in a convenient location;
* Navigate to the directory where you unpacked the archive;
* Run the command `./netter install`;
* If the previous step was successful, run the command `netter service-start`;
* Enjoy!

**Building from Source Code:**

*Prerequisites: Rust must be installed on your device. You can download it either from the official website or using [install_rust.bat](install_rust.bat)*

* Clone the project: `git clone https://github.com/bjfssd757/netter`;
* Navigate to the project directory;
* Run the following commands:

```powershell
cargo build --release; cargo build --release -p netter_service; cd target/release
```

> [!INFO]
> `cargo build --release` will create the CLI executable `netter` in `target/release`;
> `cargo build --release -p netter_service` will create the service executable `netter_service` in `target/release`

* After executing these commands, you will be in the build directory (`target/release`). Execute the following commands:

```powershell
./netter install; ./netter service-start
```

* These commands will create and start the `NetterService` service. To check the CLI connection with the service: `./netter ping`
* **Optional**: Add the path to `netter.exe` to your PATH environment variable.

### Linux

**Installation from Release Assets:**

* Download the archive for Linux;
* Unpack it in a convenient location;
* Navigate to the directory where you unpacked the archive;
* Run the command `./netter install`;
* If the previous step was successful, run the command `netter service-start`;
* Enjoy!

**Building from Source Code:**

*Prerequisites: Rust must be installed on your device. You can download it either from the official website or using [install_rust.sh](install_rust.sh)*

* Clone the project: `git clone https://github.com/bjfssd757/netter`;
* Navigate to the project directory;
* Run the following commands:

```powershell
cargo build --release; cargo build --release -p netter_service; cd target/release
```

> [!INFO]
> `cargo build --release` will create the CLI executable `netter` in `target/release`;
> `cargo build --release -p netter_service` will create the daemon executable `netter_service` in `target/release`

* After executing these commands, you will be in the build directory (`target/release`). Execute the following commands:

```powershell
./netter install; ./netter service-start
```

* These commands will create and start the daemon. To check the CLI connection with the daemon: `./netter ping`
* **Optional**: Add the path to the `netter` executable to your PATH environment variable.
