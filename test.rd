config {
    type = "http";
    host = "127.0.0.1";
    port = 9090;
};

import "E:/projects/rust/cli/target/release/netter_plugins_hub.dll" as hub;

route "/" GET {
    val a = 1;
    val b = 2;
    val google = hub::add_numbers(a, b)?;
    log_error("!!!!! Выполнение после ?");
    Response.body(google);
    Response.send();
} onError(e) {
    Response.status(500);
    Response.body(e);
    Response.send();
};