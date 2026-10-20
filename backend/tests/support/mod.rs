use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    server::WebPkiClientVerifier,
    RootCertStore, ServerConfig, ServerConnection, StreamOwned,
};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

pub const CA: &str = include_str!("../tls/fixtures/ca.pem");
pub const OTHER: &str = include_str!("../tls/fixtures/other.pem");
pub const SERVER: &str = include_str!("../tls/fixtures/server.pem");
pub const SERVER_KEY: &str = include_str!("../tls/fixtures/server.key");
pub const CLIENT: &str = include_str!("../tls/fixtures/client.pem");
pub const CLIENT_KEY: &str = include_str!("../tls/fixtures/client-pkcs8.key");
pub const PC_ID: &str = "PCPCPCP-PCPCPCP-PCPCPCP-PCPCPCP-PCPCPCP-PCPCPCP-PCPCPCP-PCPCPCP";
pub const NAS_ID: &str = "NASNASN-NASNASN-NASNASN-NASNASN-NASNASN-NASNASN-NASNASN-NASNASN";

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,

    pub certified: bool,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

pub struct Reply {
    pub status: u16,
    pub headers: Vec<(&'static str, String)>,
    pub body: String,
}

pub fn reply(status: u16, body: &str) -> Reply {
    Reply {
        status,
        headers: vec![("Content-Type", "application/json".to_owned())],
        body: body.to_owned(),
    }
}

pub type Handler = Arc<dyn Fn(&Request) -> Reply + Send + Sync>;

pub struct Server {
    pub port: u16,

    pub seen: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    pub fn calls(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|request| format!("{} {}", request.method, request.path))
            .collect()
    }
}

fn read_request(stream: &mut impl Read, certified: bool) -> Option<Request> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next()?.to_owned(), parts.next()?.to_owned());
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let (name, value) = header.split_once(':')?;
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some(Request {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
        certified,
    })
}

fn write_reply(stream: &mut (impl Write + ?Sized), reply: &Reply) {
    let mut text = format!(
        "HTTP/1.1 {} X\r\nConnection: close\r\nContent-Length: {}\r\n",
        reply.status,
        reply.body.len()
    );
    for (name, value) in &reply.headers {
        text.push_str(&format!("{name}: {value}\r\n"));
    }
    text.push_str("\r\n");
    text.push_str(&reply.body);
    let _ = stream.write_all(text.as_bytes());
    let _ = stream.flush();
}

fn serve(handler: Handler, tls: Option<Arc<ServerConfig>>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen: Arc<Mutex<Vec<Request>>> = Arc::default();
    let log = seen.clone();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (handler, log, tls) = (handler.clone(), log.clone(), tls.clone());
            thread::spawn(move || handle(stream, handler, log, tls));
        }
    });
    Server { port, seen }
}

fn handle(
    stream: TcpStream,
    handler: Handler,
    log: Arc<Mutex<Vec<Request>>>,
    tls: Option<Arc<ServerConfig>>,
) {
    let answer = |request: Option<Request>, stream: &mut dyn Write| {
        if let Some(request) = request {
            let reply = handler(&request);
            log.lock().unwrap().push(request);
            write_reply(stream, &reply);
        }
    };
    match tls {
        Some(config) => {
            let mut stream = StreamOwned::new(ServerConnection::new(config).unwrap(), stream);
            let mut first = [0u8; 0];
            let _ = stream.read(&mut first);
            while stream.conn.is_handshaking() {
                if stream.conn.complete_io(&mut stream.sock).is_err() {
                    return;
                }
            }
            let certified = stream.conn.peer_certificates().is_some();
            let request = read_request(&mut stream, certified);
            answer(request, &mut stream);
            stream.conn.send_close_notify();
            let _ = stream.conn.complete_io(&mut stream.sock);
        }
        None => {
            let mut stream = stream;
            let request = read_request(&mut stream, false);
            answer(request, &mut stream);
        }
    }
}

pub fn https(handler: impl Fn(&Request) -> Reply + Send + Sync + 'static) -> Server {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut roots = RootCertStore::empty();
    for certificate in CertificateDer::pem_slice_iter(CA.as_bytes()) {
        roots.add(certificate.unwrap()).unwrap();
    }
    let verifier = WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider.clone())
        .allow_unauthenticated()
        .build()
        .unwrap();
    let chain: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(SERVER.as_bytes())
        .chain(CertificateDer::pem_slice_iter(CA.as_bytes()))
        .map(Result::unwrap)
        .collect();
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            chain,
            PrivateKeyDer::from_pem_slice(SERVER_KEY.as_bytes()).unwrap(),
        )
        .unwrap();
    serve(Arc::new(handler), Some(Arc::new(config)))
}

pub fn http(handler: impl Fn(&Request) -> Reply + Send + Sync + 'static) -> Server {
    serve(Arc::new(handler), None)
}

pub struct Scratch(pub std::path::PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "nas-sync-client-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
