route "/api/resource" POST {
    val requestData = request.get_body();
    Database.add(requestData);
    response.body("Resource created successfully").send();
};