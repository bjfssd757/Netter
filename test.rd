route "/user" GET {
    val action = "active";
    if (action == "active") {
        response.body("TRUE");
    } else {
        response.body("FALSE");
    }
    response.send();
};