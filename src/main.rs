use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Error, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    process::exit,
};

#[derive(Debug)]
struct Header {
    key: String,
    value: String,
}
#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    version: String,
    headers: Vec<Header>,
    body: String,
}

struct HttpResponse {
    version: String,
    status: u32,
    status_message: String,
    headers: Vec<Header>,
    body: String,
}

// const STATUS_MESSAGE: HashMap<i32, &str> = HashMap::from([(200, "OK"), (404, "Not Found")]);

fn parse_request<'a>(mut lines: impl Iterator<Item = &'a str>) -> Result<HttpRequest, Error> {
    let request_line = lines.next().ok_or(Error::new(
        std::io::ErrorKind::InvalidInput,
        "Missing request line",
    ))?;

    let mut request_line_parts = request_line.split_whitespace();

    let method = request_line_parts
        .next()
        .ok_or(Error::new(
            std::io::ErrorKind::InvalidInput,
            "Method missing",
        ))?
        .to_string();
    let path = request_line_parts
        .next()
        .ok_or(Error::new(std::io::ErrorKind::InvalidInput, "Path missing"))?
        .to_string();
    let version = request_line_parts
        .next()
        .ok_or(Error::new(
            std::io::ErrorKind::InvalidInput,
            "Version missing",
        ))?
        .to_string();

    let mut headers: Vec<Header> = vec![];

    while let Some(header) = lines.next() {
        if header.is_empty() {
            break;
        }

        let (k, v) = header.split_once(":").ok_or(Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid header format",
        ))?;

        headers.push(Header {
            key: k.to_string(),
            value: v.trim_start().to_string(),
        })
    }

    Ok(HttpRequest {
        method,
        path,
        version,
        headers,
        body: "".to_string(),
    })
}

fn make_response(request: HttpRequest) -> HttpResponse {
    if (request.path.eq_ignore_ascii_case("/")) {
        HttpResponse {
            version: request.version,
            status: 200,
            status_message: "OK".to_string(),
            headers: vec![],
            body: "".to_string(),
        }
    } else if request.path.contains("/echo/") {
        let body = request.path.strip_prefix("/echo/").unwrap_or_default();
        let mut headers = vec![];

        if (!body.is_empty()) {
            headers.push(Header {
                key: "Content-Length".to_string(),
                value: body.len(),
            })
        }

        HttpResponse {
            version: request.version,
            status: 200,
            status_message: "OK".to_string(),
            headers,
            body: body.to_string(),
        }
    } else {
        HttpResponse {
            version: request.version,
            status: 404,
            status_message: "Not Found".to_string(),
            headers: vec![],
            body: "".to_string(),
        }
    }
}

fn make_response_string(response: HttpResponse) -> String {
    format!(
        "{} {} {}\r\n\r\n",
        response.version, response.status, response.status_message
    )
}

fn handle_connection(mut connection: TcpStream) -> Result<(), Error> {
    println!("Connected to {}", connection.peer_addr()?);

    let mut buf_reader = BufReader::new(&mut connection);
    let mut request_buffer = String::new();

    loop {
        let mut line = String::new();
        buf_reader.read_line(&mut line);

        if line.is_empty() {
            println!("Client closed the connection");
            return Ok(());
        }

        request_buffer.push_str(&line);

        if line == "\r\n" {
            break;
        }
    }
    let mut http_request_iter = request_buffer.lines();
    let mut http_req = parse_request(http_request_iter)?;

    let content_length = http_req
        .headers
        .iter()
        .find(|h| h.key.eq_ignore_ascii_case("Conetent-Length"))
        .and_then(|h| h.value.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = String::new();
    if content_length > 0 {
        let mut body_buf = vec![0; content_length];
        buf_reader.read_exact(&mut body_buf)?;
        body = String::from_utf8(body_buf).unwrap_or_default();
    }

    http_req.body = body;
    let http_res = make_response(http_req);
    let response_string = make_response_string(http_res);
    // println!("{}", response_string);
    let written = connection.write_all(response_string.as_bytes())?;

    println!("Connection close {}", connection.peer_addr()?);
    // drop(connection);
    Ok(())
}

#[allow(unused)]
fn main() {
    const PORT: u16 = 4221;

    let server = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), PORT)).unwrap_or_else(|err| {
        eprintln!("Server: Listen {}", err.to_string());
        exit(1);
    });

    println!("Server listening at {}", server.local_addr().unwrap());

    for connection in server.incoming() {
        match connection {
            Ok(connection) => {
                if let Err(err) = handle_connection(connection) {
                    eprintln!("Some problem with handling connection")
                }
            }
            Err(err) => {
                println!("Could not successfully accept {}", err.to_string())
            }
        }
    }
}
