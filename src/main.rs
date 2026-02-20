#[allow(unused_imports)]
use std::net::TcpListener;
use std::io::Write;
use std::io::Read;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // TODO: Uncomment the code below to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = [0; 1024];
                let bytes_read = stream.read(&mut buffer).unwrap();
                println!("Request: {}", String::from_utf8_lossy(&buffer[..bytes_read]));
                let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                let path = request.split_whitespace().nth(1).unwrap();
                let response = match path {
                    "/" => "HTTP/1.1 200 OK\r\n\r\n",
                    "/echo" => "HTTP/1.1 200 OK\r\n\r\n",
                    _ => "HTTP/1.1 404 Not Found\r\n\r\n",
                };
                println!("Path: {}", path);

                stream.write_all(response.as_bytes()).unwrap();
            }
            Err(e) => println!("error: {}", e),
        }
    }
}
