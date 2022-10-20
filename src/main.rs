use httparse;
use std::io::Result;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Request {
    pub method: String,
    pub path: String,
    pub version: u8,
    pub headers: Vec<Header>,
}

struct Header {
    pub name: String,
    pub value: String,
}

struct Response {
    pub status_code: u16,
    pub status_message: String,
    pub headers: Vec<Header>,
    pub body: String,
}

impl Response {
    fn new(status_code: u16, status_message: &str, headers: Vec<Header>, body: String) -> Self {
        Self {
            status_code,
            status_message: String::from(status_message),
            headers,
            body,
        }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        format!(
            "HTTP/1.1 {} {}\r\n{}\r\n{}",
            self.status_code,
            self.status_message,
            self.headers_string(),
            self.body
        )
        .into()
    }

    fn headers_string(&self) -> String {
        let mut s = String::with_capacity(&self.headers.len() * 2);
        s.push_str("Content-Length: ");
        s.push_str(&self.body.len().to_string());
        s.push_str("\r\n");
        s.push_str("Content-Type: text/plain");
        s.push_str("\r\n");
        for header in &self.headers {
            s.push_str(&header.name);
            s.push_str(": ");
            s.push_str(&header.value);
            s.push_str("\r\n");
        }
        return s;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Listening on port 8080");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {
                let _n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                // parse http request
                let mut headers = [httparse::EMPTY_HEADER; 32];
                let mut req = httparse::Request::new(&mut headers);
                loop {
                    match req.parse(&buf) {
                        Ok(r) => {
                            if r.is_partial() {
                                continue;
                            }
                            if r.is_complete() {
                                break;
                            }
                        }
                        Err(e) => {
                            let response = Response::new(
                                400,
                                "Bad Request",
                                vec![],
                                format!("failed to parse http; err = {:?}", e),
                            );
                            if let Err(er) = socket.write_all(&response.to_bytes()).await {
                                eprintln!("failed to write to socket; err = {:?}", er);
                            }
                            return;
                        }
                    }
                }

                let response = Response::new(200, "OK", vec![], String::from("hello"));
                if let Err(e) = socket.write_all(&response.to_bytes()).await {
                    // if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}
