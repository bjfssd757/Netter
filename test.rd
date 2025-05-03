config {
    type = "http";
    host = "127.0.0.1";
    port = 9091;
};

global_error_handler(e) {
    log_error("Глобальная ошибка: " + e);
    val status = 500;
    Response.status(status);
    Response.body("Сервер обнаружил ошибку: " + e);
    Response.send();
};

route "/{id}" GET {
    val body = Request.body()?;
    log_error("Выполнение после ?");
    Response.body(body + "- тело пустое");
    Response.send();
};

route "/error/{id}" GET {
    val body = Request.body()?;
    val id = Request.get_params("id");
    val user = Database.get(id)?;
    log_error("выполнение после ?");
    Response.status(200);
    Response.body(body);
    Response.send();
} onError(e) {
    Response.body("body is empty or error in database!");
    Response.status(500);
    Response.send();
};

route "/api/complex/{id}" GET {
    val id = Request.get_params("id");
    if (id == "0") {
        val stat = 400;
        Response.body("ID не может быть 0");
        Response.status(400);
        Response.send();
    } else {
        val user = Database.get(id)?;
        val check = Database.check()!!;
        Response.body("Пользователь: " + user + ", Статус: " + check);
        Response.status("200");
        Response.send();
    }
} onError(e) {
    Response.body("Ошибка обработки запроса: " + e);
    Response.status("422");
    Response.send();
};