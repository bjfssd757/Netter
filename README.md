# Netter

Netter is a CLI tool for quickly and easily launching servers.

## Table of Contents

* [Future](#future)
* [Features](#features)
* [Documentation](#documentation)
* [Installation](#installation)

## Future

* Support for complex server structures and routing;
* Support for different types of servers: HTTP, gRPC, TCP/UDP sockets;
* Support for SSL/TLS.

## Features

* Creating a server on web sockets (websockets);
* Stopping any server launched via netter.

## Documentation

Launching a server is done using the command:

```powershell
netter start
```

Stopping a server is done using the command:

```powershell
netter stop
```

### Start

The start command accepts the following parameters:

* **--type**: server type: **websocket**, **tcp**, **udp**, **http**, **grpc**:

```powershell
netter start --websocket
```

* **--host**: server address:

```powershell
netter start --websocket --host 127.0.0.1
```

* **--port**: server port:

```powershell
netter start --websocket --host 127.0.0.1 --port 808
```

* **--protect**: whether to protect or not (default is no. If the flag is absent, it is also no):

```powershell
netter start --websocket --host 127.0.0.1 --port 8080 --protect
```

For the type parameter and server protection status, you do not need to specify anything other than the flag itself.

### Stop

The stop command will stop any running server:

```powershell
netter stop
```

#### How does it work?

> [!NOTE]
> When starting a server (netter start), a server state file is created, which specifies the host, **pid**, port, and protection status. This file helps maintain the existence of a running server and manage it during the process, as each new command you use = code execution from scratch. The running server will continue to work because it is built on asynchronous operations.\
> The presence of the pid parameter in the state file helps terminate the server. It indicates the process ID of the server in the system. After using the stop command, netter "kills" the process (stops it).

## Installation

In case of errors or questions about installing Rust, you can refer to the documentation in the [Rust book](https://rust-lang.github.io/book/ch01-01-installation.html).

### Windows

* To install Rust on Windows, you need to go to [this link](https://www.rust-lang.org/tools/install) and download the language from there.

* Then you need to install netter:

```bash
cargo install netter
```

### Linux

* Install Rust:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
```

* Install netter:

```bash
cargo install netter
```
