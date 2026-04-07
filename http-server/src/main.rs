use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

fn main() {
    println!("Logs from your program will appear here!");

    let args: Vec<String> = std::env::args().collect();
    
    // let directory = if let Some(directory_index) = directory_index {
    //     args[directory_index + 1].clone()
    // } else {
    //     ".".to_string()
    // };
    // println!("directory: {:?}", directory);

    let directory = match args.iter().position(|a| a== "--directory") {
        Some(index) => args[index + 1].clone(),
        None => ".".to_string(),
    };
    println!("directory: {:?}", directory);


    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    
    for stream in listener.incoming() {

        match stream {
            Ok(stream) => {
                let dir = directory.clone();
                thread::spawn(move || {
                    handle_connection(stream, dir);
                });
            }
            Err(e) => println!("error: {}", e),
        }
    }
}

fn handle_connection(mut stream: TcpStream, directory: String) {
                        let mut buffer = [0; 1024];
                        let bytes_read = stream.read(&mut buffer).unwrap();
                        println!("Request: {}", String::from_utf8_lossy(&buffer[..bytes_read]));
                        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                        let path = request.split_whitespace().nth(1).unwrap();

                        let response = match path {
                            "/" => "HTTP/1.1 200 OK\r\n\r\n".to_string(),
                            p if p.starts_with("/echo/") => {
                                let echo_str = &p[6..];
                                format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                                    echo_str.len(),
                                    echo_str
                                )
                            }
                            "/user-agent" => {
                                let user_agent = request
                                    .lines()
                                    .find(|line| line.starts_with("User-Agent: "))
                                    .map(|line| &line[12..])
                                    .unwrap_or("");
                                format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                                    user_agent.len(),
                                    user_agent
                                )
                            }
                            p if p.starts_with("/files/") => {
                                let file_path = format!("{}/{}", directory.trim_end_matches('/'), &p[7..]);
                                let file = File::open(file_path);
                                match file {
                                    Ok(mut file) => {
                                        let mut content = String::new();
                                        file.read_to_string(&mut content).unwrap();
                                        format!(
                                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                            content.len(),
                                            content
                                        )
                                    }
                                    Err(_e) => {
                                        format!(
                                            "HTTP/1.1 404 Not Found\r\n\r\n"
                                        )
                                    }
                                }
                            }
                            _ => "HTTP/1.1 404 Not Found\r\n\r\n".to_string(),
                        };
    stream.write_all(response.as_bytes()).unwrap();
}
