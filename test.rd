config {
    type = "http";
    host = "127.0.0.1";
    port = 8080;
};

import "E:/projects/rust/cli/target/release/plugin_name.dll" as plg;

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
    val a = plg::is_email_valid("test@test.ru")?;
    val b = plg::is_ip_valid("127.0.0.1")?;
    val c = plg::env_var("HOME")?;
    val d = plg::random(1, 100)?;
    val e = plg::to_uppercase("this is lowercase")?;
    val f = plg::to_lowercase("THIS IS UPPERCASE")?;
    val now = plg::now()?;
    plg::sleep(2)?;
    val after = plg::now()?;

    val res = "a = " + a + "\n" + "b = " + b + "\n" + "c = " + c + "\n" + "d = " + d + "\n" + "e = " + e + "\n" + "f = " + f + "\n\n" + "before = " + now + "\n" + "after = " + after;

    Response.body(res);
    Response.send();
};