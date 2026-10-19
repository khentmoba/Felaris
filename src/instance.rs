use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    time::Duration,
};

#[derive(Serialize, Deserialize)]
struct Endpoint {
    port: u16,
    token: String,
}

#[derive(Serialize, Deserialize)]
struct Request {
    token: String,
    urls: Vec<String>,
}

pub struct Instance {
    _lock: File,
    listener: TcpListener,
    token: String,
}

impl Instance {
    pub fn acquire(
        directory: &Path,
        urls: &[String],
        updating: bool,
    ) -> Result<Option<Self>, String> {
        use std::hash::BuildHasher;
        std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("instance.lock"))
            .map_err(|e| e.to_string())?;
        for attempt in 0..100 {
            match lock.try_lock() {
                Ok(()) => {
                    let listener =
                        TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
                    let token = format!(
                        "{:x}{:x}",
                        std::collections::hash_map::RandomState::new().hash_one("instance"),
                        std::collections::hash_map::RandomState::new().hash_one("token")
                    );
                    let endpoint = Endpoint {
                        port: listener.local_addr().map_err(|e| e.to_string())?.port(),
                        token: token.clone(),
                    };
                    crate::profile::write_atomic(
                        &directory.join("instance.json"),
                        &serde_json::to_vec(&endpoint).unwrap(),
                    )
                    .map_err(|e| e.to_string())?;
                    return Ok(Some(Self {
                        _lock: lock,
                        listener,
                        token,
                    }));
                }
                Err(std::fs::TryLockError::WouldBlock) => {
                    if !updating
                        && let Ok(bytes) = std::fs::read(directory.join("instance.json"))
                        && let Ok(endpoint) = serde_json::from_slice::<Endpoint>(&bytes)
                        && let Ok(mut stream) = TcpStream::connect_timeout(
                            &([127, 0, 0, 1], endpoint.port).into(),
                            Duration::from_secs(1),
                        )
                    {
                        stream
                            .set_write_timeout(Some(Duration::from_secs(1)))
                            .map_err(|e| e.to_string())?;
                        let request = Request {
                            token: endpoint.token,
                            urls: urls.to_vec(),
                        };
                        let mut bytes = serde_json::to_vec(&request).unwrap();
                        bytes.push(b'\n');
                        stream.write_all(&bytes).map_err(|e| e.to_string())?;
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .map_err(|e| e.to_string())?;
                        let mut ack = [0];
                        if stream.read_exact(&mut ack).is_ok() && ack == [1] {
                            return Ok(None);
                        }
                    }
                    if attempt < 99 {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
                Err(error) => return Err(error.to_string()),
            }
        }
        Err(
            "Felaris is already running but could not receive the link. Close it and try again."
                .into(),
        )
    }

    pub fn listen(&self, receive: impl Fn(Vec<String>) + Send + 'static) -> Result<(), String> {
        let listener = self.listener.try_clone().map_err(|e| e.to_string())?;
        let token = self.token.clone();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                let mut line = String::new();
                if BufReader::new((&mut stream).take(16_384))
                    .read_line(&mut line)
                    .is_ok()
                    && let Ok(request) = serde_json::from_str::<Request>(&line)
                    && request.token == token
                    && request
                        .urls
                        .iter()
                        .all(|url| crate::profile::allowed_url(url))
                {
                    receive(request.urls);
                    let _ = stream.write_all(&[1]);
                }
            }
        });
        Ok(())
    }
}
