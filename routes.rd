route "/api/users" GET {
    val users = Database.get_all();
    response.body(users).send();
};

route "/api/user/{id}" GET {
    val user_id = request.get_params("id");
    response.body(Database.get(user_id));
    response.send();
};

route "/api/user/{id}" POST {
    val user_id = request.get_params("id");
    
    if (Database.check()) {
        response.body("User updated successfully");
    } else {
        Database.add(
            user_id,
            "new_user",
            "hashed_password"
        );
        response.body("User created successfully");
    }
    
    response.send();
};

route "/api/admin/{action}" POST {
    val action = request.get_params("action");
    
    if (action == "create") {
        Database.add(
            "admin",
            "admin_user",
            "admin_pass"
        );
        response.body("Admin created");
    } else if (action == "delete") {
        response.body("Admin deleted");
    } else {
        response.body("Unknown admin action");
    }
    
    response.send();
};

route "/api/complex" GET {
    /* 
     * Это многострочный комментарий
    */

    // А это просто комментарий

    val status = "active";
    val data = Database.get_all();
    
    if (status == "active") {
        response.body(data);
    } else {
        response.body("Service unavailable");
    }
    
    response.send();
};