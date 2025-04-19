route "/api/resource/{id}" GET {
    val resourceId = request.get_params("id");
    val resource = Database.get(resourceId);
    
    if (resource == "") {
        // Resource not found
        response.body("Resource not found");
    } else {
        // Resource found
        response.body(resource);
    }
    
    response.send();
};