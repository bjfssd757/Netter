config {
    type = "http";
    host = "127.0.0.1";
    port = 8080;
};

import "path/to/plugin_name.dll" as plg;

global_error_handler(error) {
    Response.status(500);
    Response.headers("Content-Type", "application/json");
    Response.body("{\"error\": \"" + error + "\"}");
    Response.send();
};

route "/test" GET {
    val a = 2;
    val b = 15 - 3;
    while (a != 5) {
        a += 1;
        b -= 2;
    };
    Response.body("a = " + a + "b = " + b);
    Response.send();
};

route "/test2" GET {
    val mix = ["hello", "hey", 1, 2];
    val body = "";
    for (i in mix) {
        body += i;
        body += " ";
    };
    body += "\n";
    body += array_length(mix);
    body += "\n\n";
    body += array_contains(mix, 1);
    Response.body(body);
    Response.send();
};

route "/test3" GET {
    val num = plg::add_numbers(12, 3);
    Response.body(num);
    Response.send();
};