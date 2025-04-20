tls {
    enabled = true;
    cert_path = "E:/projects/rust/cli/tls/cert.pem";
    key_path = "E:/projects/rust/cli/tls/key.pem";
};

route "/test" GET {
    response.body("TLS работает!");
    response.send();
};