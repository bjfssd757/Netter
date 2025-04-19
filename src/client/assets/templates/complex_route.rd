route "/api/resource/{id}" GET {
    val resourceId = request.get_params("id");
    val resource = Database.get(resourceId);
    
    if (resource != null) {
        // Resource found
        response.body(resource);
    } else {
        // Resource not found
        response.status(404).body("Resource not found");
    }
    
    response.send();
};