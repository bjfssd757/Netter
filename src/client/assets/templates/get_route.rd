route "/api/resource" GET {
    val data = Database.get_all();
    response.body(data).send();
};