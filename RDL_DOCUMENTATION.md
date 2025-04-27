# Route Definition Language

## Table of Contents

[Introduction](#introduction)\
[Route Declaration](#route-declaration)\
[Variables, Types, Errors](#variables-types-errors)\
[Global Configuration](#global-configuration)\
[Error Interceptors](#error-interceptors)\
[Objects and Functions](#objects-and-functions)\
[Errors](#errors)

## Introduction

Language Features:

- Interpreted. The parser, lexer, and interpreter are written in Rust;
- Case-sensitive;
- Ignores whitespace;
- `;` is mandatory;
- If an `if` conditional statement does not have an `else` or `if else` condition after the block ends, a `;` is mandatory: `if (a == "string") {  };`.

*The language is still under development! Stay tuned for updates!*

## Route Declaration

To declare a route, use the keyword `route`. Then comes the route that the server will handle (case-sensitive!). Next is the type of requests it expects.

``` rd
route "/users" GET {};
```

## Variables, Types, Errors

- Variables can be declared using the keywords `var` or `val`:

```rd
val a = 1;
```

- The language has string typing - any value that a function can return or a value entered directly (`val a = 2;`) is a string. That is, in the example above, `a` contains not the numeric type 1, but the string **"1"**;

- You have the ability to handle errors in two ways: a local handler (error handling for a specific block) or a global handler (if there is no local handler, errors go there). The local handler has priority (i.e., if both global and local handlers exist, the error will go to the local one).

Local handler:

```rd
route "/user/{id}" GET {
    val user = Request.get_params("id")?;
    // If the get_params function ends with an error (there is no {id} in the path), the code will not proceed further, but will go to the handling block
    Response.body("Hello, " + user);
    Response.send();
} onError(e) { // (e) - the variable where the error is placed (error is a string)
    Response.status(500);
    Response.body("Error on get request params: " + e);
    Response.send();
}; // ';' is mandatory after each block (exception - the end of an "else" block)!
```

Global handler:

```rd
global_error_handler(e) {
    Response.status(500);
    Response.body("Server-side error: " + e);
    Response.headers("Content-Type", "text/html");
    Response.send();
};

route "/user" GET {
    val body = Request.body()?;
    // If an error is caught (the request has no body), execution will jump to global_error_handler
    Response.body(body);
    Response.send();
};
```

## Global Configuration

There are 2 configurations you can set up: tls (connection security settings) and global_error_handler (global error handling).

TLS:

```rd
tls {
    enabled = true;
    key = path/to/key;
    cert = path/to/cert;
}; // If there are no errors in this block, all connections will go through https, not http
```

Global handler:

```rd
global_error_handler { // we do not save the error and cannot use it
    Response.status(500);
    Response.send();
};
```

## Error Interceptors

There are 2 ways to catch an error: using the `?` operator (catches the error, stops code execution, and goes to the handler) and `!!` (ignores a potential error. If it exists, it will cause an emergency code termination (panic)).

`?`:

```rd
val body = Request.body()?;
```

`!!`:

```rd
val body = Request.body()!!;
```

If no operator is set, execution proceeds as `!!`.

## Objects and Functions

Objects:

- **Database**: This object provides access to database functions;
- **Request**: This object provides access to request handling functions;
- **Response**: This object provides access to response configuration functions.

### Functions

**Database**:

- **get_all()**: Get all records from the database. [Errors](#database);
- **get(id)**: Get a record by id. [Errors](#database);
- **check(id)**: Check for the existence of a record by id. [Errors](#database).

**Request**:

- **body()**: Get the request body. [Errors](#request);
- **get_params()**: Get parameters from the route (insertions {name_param} in the path: `route "/user/{id}`). [Errors](#request);
- **headers()**: Get headers from the route. [Errors](#request).

**Response**:

- **status()**: Set the response status;
- **body()**: Set the response body;
- **headers()**: Set the response headers;
- **send()**: Send the assembled response;

## Errors

### Database

- **get_all()**: No records in the database;
- **get(id)**: Record with `id` not found in the database;
- **check(id)**: Record with `id` not found in the database;

### Request

- **body()**: The request has no body;
- **get_params()**: The route path has no parameter with the specified `id`;
- **headers()**: The request has no headers;

## Conclusion

If any of these functions work incorrectly, be sure to report the bug in an issue!
