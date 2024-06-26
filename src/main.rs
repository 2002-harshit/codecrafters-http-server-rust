use flate2::{write::GzEncoder, Compression};
use http_server_starter_rust::ThreadPool;
use std::{
    env,
    fs::File,
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
struct HttpRequest<'a> {
    method: &'a str,
    path: &'a str,
    version: &'a str,
    headers: Vec<Header>,
    body: Vec<u8>,
}

struct HttpResponse<'a> {
    version: &'a str,
    status: HttpStatus,
    status_message: &'static str,
    headers: Vec<Header>,
    body: Vec<u8>,
}

// enum HttpMethod {
//     GET,
//     POST,
// }

enum HttpStatus {
    OK = 200,
    Created = 201,
    NotFound = 404,
}

fn parse_request<'a>(mut lines: impl Iterator<Item = &'a str>) -> Result<HttpRequest<'a>, Error> {
    let request_line = lines.next().ok_or(Error::new(
        std::io::ErrorKind::InvalidInput,
        "Missing request line",
    ))?;

    let mut request_line_parts = request_line.split_whitespace();

    let method = request_line_parts.next().ok_or(Error::new(
        std::io::ErrorKind::InvalidInput,
        "Method missing",
    ))?;
    // .to_string();
    let path = request_line_parts
        .next()
        .ok_or(Error::new(std::io::ErrorKind::InvalidInput, "Path missing"))?;
    let version = request_line_parts.next().ok_or(Error::new(
        std::io::ErrorKind::InvalidInput,
        "Version missing",
    ))?;

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
        body: vec![],
    })
}

fn get_status_message(status: HttpStatus) -> &'static str {
    match status {
        HttpStatus::OK => "OK",
        HttpStatus::Created => "Created",
        HttpStatus::NotFound => "Not Found",
    }
}

fn make_response<'a>(request: HttpRequest<'a>, dirname: String) -> Result<HttpResponse<'a>, Error> {
    if (request.method.eq_ignore_ascii_case("GET")) {
        if request.path.eq_ignore_ascii_case("/") {
            Ok(HttpResponse {
                version: request.version,
                status: HttpStatus::OK,
                status_message: get_status_message(HttpStatus::OK),
                headers: vec![],
                body: vec![],
            })
        } else if request.path.contains("/echo/") {
            let mut randomstr_bytes = request
                .path
                .strip_prefix("/echo/")
                .unwrap_or_default()
                .as_bytes()
                .to_vec();
            let mut headers = vec![];

            let accepted_encoding = request
                .headers
                .iter()
                .find(|header| header.key.eq_ignore_ascii_case("Accept-Encoding"))
                .and_then(|header| Some(header.value.as_str()));

            if let Some(encoding) = accepted_encoding {
                if (encoding.contains("gzip")) {
                    headers.push(Header {
                        key: "Content-Encoding".to_string(),
                        value: "gzip".to_string(),
                    });
                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(&randomstr_bytes).unwrap();
                    randomstr_bytes = encoder.finish().unwrap();
                }
            }

            headers.push(Header {
                key: "Content-Length".to_string(),
                value: randomstr_bytes.len().to_string(),
            });
            headers.push(Header {
                key: "Content-Type".to_string(),
                value: "text/plain".to_string(),
            });

            Ok(HttpResponse {
                version: request.version,
                status: HttpStatus::OK,
                status_message: get_status_message(HttpStatus::OK),
                headers,
                body: randomstr_bytes,
            })
        } else if request.path.contains("/user-agent") {
            let mut headers = vec![];
            let body = request
                .headers
                .iter()
                .find(|header| header.key.eq_ignore_ascii_case("User-Agent"))
                .and_then(|header| Some(header.value.as_str()))
                .unwrap_or_default()
                .as_bytes()
                .to_vec();

            headers.push(Header {
                key: "Content-Length".to_string(),
                value: body.len().to_string(),
            });
            headers.push(Header {
                key: "Content-Type".to_string(),
                value: "text/plain".to_string(),
            });

            Ok(HttpResponse {
                version: request.version,
                status: HttpStatus::OK,
                status_message: get_status_message(HttpStatus::OK),
                headers,
                body,
            })
        } else if request.path.contains("/files/") {
            let file_name = request.path.strip_prefix("/files/").unwrap();
            let file_path = format!("{}/{}", dirname, file_name);

            match File::open(file_path) {
                Ok(mut file) => {
                    let mut body = Vec::<u8>::new();
                    file.read_to_end(&mut body)?;

                    Ok(HttpResponse {
                        version: request.version,
                        status: HttpStatus::OK,
                        status_message: get_status_message(HttpStatus::OK),
                        headers: vec![
                            Header {
                                key: "Content-Type".to_string(),
                                value: "application/octet-stream".to_string(),
                            },
                            Header {
                                key: "Content-Length".to_string(),
                                value: body.len().to_string(),
                            },
                        ],
                        body,
                    })
                }
                Err(_err) => Ok(HttpResponse {
                    version: request.version,
                    status: HttpStatus::NotFound,
                    status_message: get_status_message(HttpStatus::NotFound),
                    headers: vec![],
                    body: vec![],
                }),
            }
        } else {
            Ok(HttpResponse {
                version: request.version,
                status: HttpStatus::NotFound,
                status_message: get_status_message(HttpStatus::NotFound),
                headers: vec![],
                body: vec![],
            })
        }
    } else if request.method.eq_ignore_ascii_case("POST") {
        if request.path.contains("/files/") {
            let file_name = request.path.strip_prefix("/files/").unwrap();

            let file_path = format!("{}/{}", dirname, file_name);

            // println!("File path: {file_path} body {0}", request.body);

            File::create(file_path)?.write_all(&request.body)?;

            Ok(HttpResponse {
                version: request.version,
                status: HttpStatus::Created,
                status_message: get_status_message(HttpStatus::Created),
                headers: vec![],
                body: vec![],
            })
        } else {
            Ok(HttpResponse {
                version: request.version,
                status: HttpStatus::NotFound,
                status_message: get_status_message(HttpStatus::NotFound),
                headers: vec![],
                body: vec![],
            })
        }
    } else {
        Ok(HttpResponse {
            version: request.version,
            status: HttpStatus::NotFound,
            status_message: get_status_message(HttpStatus::NotFound),
            headers: vec![],
            body: vec![],
        })
    }
}

fn make_response_string(response: HttpResponse) -> Vec<u8> {
    // let mut response_string = format!(
    //     "{} {} {}\r\n",
    //     response.version, response.status, response.status_message
    // );

    // for header in response.headers {
    //     response_string.push_str(&format!("{}: {}\r\n", header.key, header.value))
    // }

    // response_string.push_str("\r\n");
    // let mut response_bytes = response_string.into_bytes();
    // response_bytes.extend(response.body);
    // // response_string.push_str(&format!("\r\n{}", response.body));

    // response_bytes
    let estimated_size = 100
        + response
            .headers
            .iter()
            .map(|h| h.key.len() + h.value.len() + 4)
            .sum::<usize>()
        + response.body.len();
    let mut response_bytes = Vec::with_capacity(estimated_size);
    response_bytes.extend_from_slice(response.version.as_bytes());
    response_bytes.extend_from_slice(b" ");
    response_bytes.extend_from_slice((response.status as u16).to_string().as_bytes());
    response_bytes.extend_from_slice(b" ");
    response_bytes.extend_from_slice(response.status_message.as_bytes());
    response_bytes.extend_from_slice(b"\r\n");

    for header in response.headers {
        response_bytes.extend_from_slice(header.key.as_bytes());
        response_bytes.extend_from_slice(b": ");
        response_bytes.extend_from_slice(header.value.as_bytes());
        response_bytes.extend_from_slice(b"\r\n");
    }

    response_bytes.extend_from_slice(b"\r\n");
    response_bytes.extend(&response.body);

    response_bytes
}

fn handle_connection(mut connection: TcpStream, dirname: String) -> Result<(), Error> {
    println!("Connected to {}", connection.peer_addr()?);

    let mut buf_reader = BufReader::new(&mut connection);
    let mut request_buffer = String::new();

    loop {
        let mut line = String::new();
        buf_reader.read_line(&mut line).unwrap();

        if line.is_empty() {
            println!("Client closed the connection");
            return Ok(());
        }

        request_buffer.push_str(&line);

        if line == "\r\n" {
            break;
        }
    }
    let http_request_iter = request_buffer.lines();
    let mut http_req = parse_request(http_request_iter)?;
    // std::mem::drop(request_buffer);*//! you cannot fucking do this now, because refs to it are used aage like headers
    let content_length = http_req
        .headers
        .iter()
        .find(|h| h.key.eq_ignore_ascii_case("Content-Length"))
        .and_then(|h| h.value.parse::<usize>().ok())
        .unwrap_or(0);

    // let mut body = Vec::<u8>::new();
    if content_length > 0 {
        let mut body_buf = vec![0; content_length];
        buf_reader.read_exact(&mut body_buf)?;
        // body = String::from_utf8(body_buf).unwrap_or_default();
        http_req.body = body_buf;
    }

    // http_req.body = body;
    // println!("{:?}", http_req);
    let http_res = make_response(http_req, dirname)?;
    let response_bytes = make_response_string(http_res);
    connection.write_all(&response_bytes)?;

    println!("Connection close {}", connection.peer_addr()?);
    Ok(())
}

#[allow(unused)]
fn main() {
    let args = env::args().collect::<Vec<String>>();
    let mut dirname = if args.len() == 3 {
        args[2].clone()
    } else {
        "".to_string()
    };
    println!("{:?}", args);
    const PORT: u16 = 4221;

    let server = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), PORT)).unwrap_or_else(|err| {
        eprintln!("Server: Listen {}", err.to_string());
        exit(1);
    });

    let pool = ThreadPool::build(8).unwrap_or_else(|err| {
        eprint!("ThreadPool: {}", err);
        exit(1);
    });

    println!("Server listening at {}", server.local_addr().unwrap());

    for connection in server.incoming() {
        let dirname = dirname.clone();
        match connection {
            Ok(connection) => pool.execute(move || {
                if let Err(err) = handle_connection(connection, dirname) {
                    println!("Ill formed request");
                }
            }),
            Err(err) => {
                println!("Could not successfully accept {}", err.to_string())
            }
        }
    }

    println!("Serve shutting down");
}
