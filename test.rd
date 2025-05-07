config {
    type = "http";
    host = "127.0.0.1";
    port = 9097;
};

route "/" GET {
    val body = Request.body()?;
    if (body == "empty") {
        Response.body("Body is empty!");
        Response.status(400);
        Response.send();
    };
    Response.body(body + "!empty");
    Response.send(); // 200 OK
} onError(e) {
    Response.status(500);
    Response.body(e);
    Response.send();
};