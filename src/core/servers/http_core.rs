use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{body::{self, Bytes}, header::HeaderName, server::conn::http1, service::service_fn, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use crate::{core::config_parser::load_config, state};

#[derive(Debug)]
pub struct Routes {
    method: String,
    path: String,
    response: Resp,
}

#[derive(Debug)]
pub struct Resp {
    body: String,
    headers: Vec<(String, String)>,
    status: u16,
}

#[derive(Debug)]
pub struct Server {
    addr: Vec<u16>,
    port: u16,
    routes: Vec<Routes>,
}

pub trait HTTP {
    fn new(addr: Vec<u16>, port: u16, routes: Routes) -> Self;
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    addr: Vec<u16>,
    port: u16,
    routes: Vec<RouteConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct RouteConfig {
    method: String,
    path: String,
    response: RespConfig,
}

#[derive(Serialize, Deserialize)]
pub struct RespConfig {
    body: String,
    headers: Vec<(String, String)>,
    status: u16,
}

impl Server {
    pub fn configure(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Read config data");

        let config = load_config(file_path)?;

        let routes = config.routes.into_iter().map(|route| Routes {
            method: route.method,
            path: route.path,
            response: Resp {
                body: route.response.body,
                headers: route.response.headers,
                status: route.response.status,
            },
        }).collect();

        Ok(Server {
            addr: config.host.to_string()
                .split('.')
                .map(|s| s.parse::<u16>().unwrap())
                .collect(),
            port: config.port,
            routes,
        })
    }
}

async fn handler(req: Request<body::Incoming>)
            -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let routes = Server::configure("routes.json")
                .unwrap();
    println!("routes: {:?}", &routes);
    for route in routes.routes {
        if req.method().as_str() == route.method && req.uri().path() == route.path {
            let mut response = Response::new(full(route.response.body.clone()));
            *response.status_mut() = StatusCode::from_u16(route.response.status).unwrap();
            for (key, value) in &route.response.headers {
                response.headers_mut().insert(
                    key.parse::<HeaderName>().unwrap(),
                    value.parse().unwrap(),
                );
            }
            return Ok(response);
        }
    }

    let mut not_found = Response::new(empty());
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}

impl HTTP for Server {

    fn new(addr: Vec<u16>, port: u16, routes: Routes) -> Self {
        Server {
            addr,
            port,
            routes: vec![routes],
        }
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!(
            "{}:{}",
            self.addr.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("."),
            self.port
        );
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        println!("Server started on {}", addr);

        state::save_state(String::from("HTTP"), self.addr.clone().iter().map(|n| n.to_string()).collect::<Vec<_>>().join("."), self.port.clone())
            .map_err(|_| "Failed to save state")?;

        loop {
            let (socket, _) = listener.accept().await?;
            let io = hyper_util::rt::TokioIo::new(socket);
            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(handler))
                    .await
                {
                    println!("Error serving connection: {}", err);
                }
            });
        }
    }
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}