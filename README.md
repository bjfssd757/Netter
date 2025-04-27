# Netter

Netter is a CLI tool for quickly and easily launching servers.

## Table of Contents

* [Future](#future)
* [Functionality](#functionality)
* [Documentation](#documentation)
* [Installation](#installation)

## Future

* Support for complex server structures and routing;
* Support for different types of servers: HTTP, gRPC, TCP/UDP sockets;
* Support for SSL/TLS

## Functionality

* Creating a server on web sockets (websockets);
* Stopping any server launched via netter

## Documentation

Launching the server is done via the command:

``` powershell
netter start
```

Stopping the server via the command:

```powershell
netter stop
```

[Route Definition Language Documentation](RDL_DOCUMENTATION_en.md) <!-- Assuming the RDL doc will also be translated -->

### Start

The `start` command accepts the following parameters:

* **--type**: server type: **websocket**, **tcp**, **udp**, **http**, **grpc**:

``` powershell
netter start --websocket
```

* **--host**: server address:

``` powershell
netter start --websocket --host 127.0.0.1
```

* **--port**: server port:

``` powershell
netter start --websocket --host 127.0.0.1 --port 808
```

* **--protect**: whether to protect or not (default is no. Also no if the flag is absent):

```powershell
netter start --websocket --host 127.0.0.1 --port 8080 --protect
```

For the type parameter and server protection status, you don't need to specify anything other than the flag itself.

### Stop

The `stop` command will stop any running server:

```powershell
netter stop
```

#### How does it work?

> [!NOTE]
> When starting the server (`netter start`), a server state file is created, which specifies the host, **pid**, port, and the presence of the `protect` flag. This file helps to maintain the existence of the running server itself and manage it in the process, as each new command you use = running the code again. The running server will continue to work because it is built on asynchronous operations.
> Terminating the server is aided by the presence of the `pid` parameter in the state file, which indicates the server process ID in the system. After using the `stop` command, netter "kills" the process (stops it).

## Installation

### Windows

Prerequisite: MSVC installed. If you don't have it yet, you can download it using the [guide](https://learn.microsoft.com/en-us/cpp/build/vscpp-step-0-installation?view=msvc-170)

> [!WARN]
> If you have Qt6 installed via the MSYS2 system (or another) for a different compiler (e.g., mingw) and you see an error when running `netter client`, you will have to manually download Qt for MSVC or move (or delete) your current Qt and run `netter client` again.

**Rust**:

* Run the `install_rust.bat` script;

Next, you have 2 options: install netter directly via cargo:

```powershell
cargo install netter
```

or build the release version of the project from source:

* Go to the project root;
* Run `cargo build --release`;
* (**OPTIONAL**): Add the path "path/to/netter/target/release" to your PATH.

Then you can use netter directly (if you didn't complete the last step, only in the project folder):

```powershell
netter client
```

or any other netter command.

**Desktop GUI**:

* Run `netter client`;

Then everything will happen automatically. It will start searching for MSVC, Qt6, Cmake, Ninja. If any of these (except MSVC) are not found, the [script](setup_dependencies.py) will download them automatically.

### Linux

**Rust**:

* Run the `install_rust.sh` script

Next, you have 2 options: install netter directly via cargo:

```powershell
cargo install netter
```

or build the release version of the project from source:

* Go to the project root;
* Run `cargo build --release`;
* (**OPTIONAL**): Add the path "path/to/netter/target/release" to your PATH.

**Desktop GUI**:

* Run `netter client`;

Then everything will happen automatically. It will start searching for MSVC, Qt6, Cmake, Ninja. If any of these (except MSVC) are not found, the [script](setup_dependencies.py) will download them automatically.
