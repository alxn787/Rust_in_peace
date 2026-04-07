use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

const READ_CHUNK: usize = 1024;
const MAX_HEADER_BYTES: usize = 65536;

fn main() {
    println!("Logs from your program will appear here!");

    let args: Vec<String> = std::env::args().collect();
    let directory = args
        .iter()
        .position(|a| a == "--directory")
        .map(|i| args[i + 1].clone())
        .unwrap_or_else(|| ".".to_string());

    println!("directory: {:?}", directory);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming().flatten() {
        let dir = directory.clone();
        thread::spawn(move || handle_connection(stream, dir));
    }
}

struct HttpRequest {
    method: String,
    path: String,
    headers: String,
    body: Vec<u8>,
}

impl HttpRequest {
    fn read(stream: &mut TcpStream) -> io::Result<Self> {
        let mut buf = Vec::new();
        let mut chunk = [0u8; READ_CHUNK];

        let header_end = loop {
            let n = stream.read(&mut chunk)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "connection closed before headers",
                ));
            }
            buf.extend_from_slice(&chunk[..n]);

            if let Some(pos) = double_crlf_at(&buf) {
                break pos;
            }
            if buf.len() > MAX_HEADER_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "headers too large",
                ));
            }
        };

        let headers = String::from_utf8_lossy(&buf[..header_end]).to_string();
        let body_len = header_value(&headers, "Content-Length")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let body_start = header_end + 4;

        while buf.len() < body_start + body_len {
            let n = stream.read(&mut chunk)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "connection closed before full body",
                ));
            }
            buf.extend_from_slice(&chunk[..n]);
        }

        let body = buf[body_start..body_start + body_len].to_vec();
        let (method, path) = parse_request_line(&headers);

        Ok(Self {
            method,
            path,
            headers,
            body,
        })
    }
}

fn handle_connection(mut stream: TcpStream, directory: String) {
    let req = match HttpRequest::read(&mut stream) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("read error: {}", e);
            return;
        }
    };

    println!(
        "Request: {} {}",
        req.method, req.path
    );

    let response = dispatch(&req, &directory);
    let _ = stream.write_all(response.as_bytes());
}

fn dispatch(req: &HttpRequest, files_root: &str) -> String {
    if req.method == "POST" && req.path.starts_with("/files/") {
        return save_uploaded_file(files_root, &req.path["/files/".len()..], &req.body);
    }

    match req.path.as_str() {
        "/" => "HTTP/1.1 200 OK\r\n\r\n".to_string(),
        p if p.starts_with("/echo/") => echo_response(req, &p["/echo/".len()..]),
        "/user-agent" => text_ok(header_value(&req.headers, "User-Agent").unwrap_or("")),
        p if p.starts_with("/files/") => serve_file(files_root, &p["/files/".len()..]),
        _ => "HTTP/1.1 404 Not Found\r\n\r\n".to_string(),
    }
}

fn save_uploaded_file(files_root: &str, filename: &str, body: &[u8]) -> String {
    let path = path_under_root(files_root, filename);
    println!("POST file: {:?}", path);

    match std::fs::write(&path, body) {
        Ok(()) => "HTTP/1.1 201 Created\r\n\r\n".to_string(),
        Err(e) => {
            eprintln!("write error: {}", e);
            "HTTP/1.1 500 Internal Server Error\r\n\r\n".to_string()
        }
    }
}

fn serve_file(files_root: &str, filename: &str) -> String {
    let path = path_under_root(files_root, filename);
    println!("GET file: {:?}", path);

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return "HTTP/1.1 404 Not Found\r\n\r\n".to_string(),
    };

    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
        content.len(),
        content
    )
}

fn text_ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

fn echo_response(req: &HttpRequest, body: &str) -> String {
    if accepts_gzip(&req.headers) {
        return format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
    }

    text_ok(body)
}

fn path_under_root(root: &str, filename: &str) -> String {
    format!("{}/{}", root.trim_end_matches('/'), filename)
}

fn double_crlf_at(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_request_line(headers: &str) -> (String, String) {
    let first = headers.lines().next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    (method, path)
}

fn header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    for line in headers.lines() {
        let line = line.trim_end_matches('\r');
        let Some((left, value)) = line.split_once(':') else {
            continue;
        };

        if left.trim().eq_ignore_ascii_case(name) {
            return Some(value.trim());
        }
    }

    None
}

fn accepts_gzip(headers: &str) -> bool {
    header_value(headers, "Accept-Encoding")
        .map(|value| value.split(',').any(|encoding| encoding.trim() == "gzip"))
        .unwrap_or(false)
}
