# Netter

Netter - это инструмент для быстрого и простого запуска серверов

## Оглавление

* [будущее](#будущее)
* [Функционал](#функционал)
* [Документация](#документация)
* [Установка](#установка)

## Будущее

* Десктоп клиент (UI);
* Поддержка иных типов (кроме HTTP/HTTPS) серверов: WebSocket, **gRPC**, TCP/UDP сокетов;
* Возможность интеграции плагинов на Rust в RDL.

## Функционал

* Кастомный язык Route Definition Language (.rd) (далее - RDL) - конфиг, описывающий возможные маршруты и логику обработки запросов на эти маршруты;
* Обработка ошибок в RDL;
* Поддержка TLS;
* Свой демон/служба (зависит от ОС);

## Документация

### Install

Для установки демона или службы используется команда install:

```powershell
netter install
```

> [!WARN]
> Исполняемый файл сервиса (`netter_service`) должен быть в той же директории, что и CLI

### Service-Start

Для запуска службы или демона используется команда service-start:

```powershell
netter service-start
```

### Service-Stop

Для остановки службы или демона используется команда service-stop:

```powershell
netter service-stop
```

### Service-Status

Для получения статуса службы или демона используется команда service-status:

```powershell
netter service-status
```

### Uninstall

Для удаления службы или демона используется команда uninstall:

```powershell
netter uninstall
```

### Ping

Для проверки соединения со службой используется команда ping:

```powershell
netter ping
```

### List

Для получения списка запущенных серверов (включая id) используется команда list:

```powershell
netter list
```

### Start

Запуск сервера происходит через команду start и флаг --config, который принимает путь к файлу кофигурации сервера .rd:

```powershell
netter start --config path/to/file.rd
```

### Stop

Отключение сервера через команду stop и флаг -i (или --id), который принимает id сервера:

```powershell
netter stop -i id
```

[Документация по Route Definition Language](RDL_DOCUMENTATION_ru.md)

## Установка

### Windows

**Установка из Assets релиза:**

* Скачайте архив под windows;
* Распокуйте в удобном для вас месте;
* Перейдите в директорию, куда вы распоковали архив;
* Запустите команду `./netter install`;
* Если всё прошло успешно на прошлом этапе, запустите команду `netter service-start`;
* Наслаждайтесь!

**Самостоятельная сборка из исходного кода:**

*Предварительные требования: наличие языка Rust на вашем устройстве. Скачать его можно либо с официального сайта, либо с помощью [install_rust.bat](install_rust.bat)*

* Склонируйте проект: `git clone https://github.com/bjfssd757/netter`;
* Перейдите в директорию проекта;
* Запустите следующие команды:

```powershell
cargo build --release; cargo build --release -p netter_service; cd target/release
```

> [!INFO]
> `cargo build --release` создаст в target/release исполняемый файл CLI `netter`;
> `cargo build --release -p netter_service` создаст в target/release исполняемый файл службы `netter_service`

* После выполнения этих команды, вы окажитесь в директории сборки (target/release). Выполните следующие команды:

```powershell
./netter install; ./netter service-start
```

* Эти команды создадут и запустят службу `NetterService`. Проверить связь CLI со службой: `./netter ping`
* **Опционально**: Добавьте путь к netter.exe в PATH

### Linux

**Установка из Assets релиза:**

* Скачайте архив под linux;
* Распокуйте в удобном для вас месте;
* Перейдите в директорию, куда вы распоковали архив;
* Запустите команду `./netter install`;
* Если всё прошло успешно на прошлом этапе, запустите команду `netter service-start`;
* Наслаждайтесь!

**Самостоятельная сборка из исходного кода:**

*Предварительные требования: наличие языка Rust на вашем устройстве. Скачать его можно либо с официального сайта, либо с помощью [install_rust.sh](install_rust.sh)*

* Склонируйте проект: `git clone https://github.com/bjfssd757/netter`;
* Перейдите в директорию проекта;
* Запустите следующие команды:

```powershell
cargo build --release; cargo build --release -p netter_service; cd target/release
```

> [!INFO]
> `cargo build --release` создаст в target/release исполняемый файл CLI `netter`;
> `cargo build --release -p netter_service` создаст в target/release исполняемый файл демона `netter_service`

* После выполнения этих команды, вы окажитесь в директории сборки (target/release). Выполните следующие команды:

```powershell
./netter install; ./netter service-start
```

* Эти команды создадут и запустят демона. Проверить связь CLI с демоном: `./netter ping`
* **Опционально**: Добавьте путь к netter.exe в PATH
