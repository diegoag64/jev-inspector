use crate::Mailbox;
use jev_protocol::{Connection, Exchange, PROTOCOL_VERSION, Snapshot};
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct Update {
    pub snapshot: Snapshot,
    pub detail: Option<(u64, Result<Exchange, String>)>,
}

pub(crate) fn connection(path: &PathBuf) -> Result<Connection, String> {
    let file = std::fs::File::open(path)
        .map_err(|_| "Start jev-bridge first, or choose its --connection-file.".to_string())?;
    let mut bytes = vec![];
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read the inspector connection file.")?;
    if bytes.len() > 4096 {
        return Err("Inspector connection file is too large.".into());
    }
    let connection: Connection =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid inspector connection file.")?;
    let url =
        reqwest::Url::parse(&connection.inspector_url).map_err(|_| "Invalid inspector address.")?;
    let loopback = url.host_str().is_some_and(|h| {
        h == "localhost"
            || h.trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    if connection.version != PROTOCOL_VERSION
        || !loopback
        || url.scheme() != "http"
        || url.path() != "/"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Version 1 connects only to a local HTTP inspector origin.".into());
    }
    Ok(connection)
}

pub(crate) fn get<T: serde::de::DeserializeOwned>(
    client: &reqwest::blocking::Client,
    connection: &Connection,
    path: &str,
) -> Result<T, String> {
    let response = client
        .get(format!(
            "{}{path}",
            connection.inspector_url.trim_end_matches('/')
        ))
        .bearer_auth(&connection.token)
        .send()
        .map_err(|_| {
            "Relay unavailable. Start jev-bridge or check its connection file.".to_string()
        })?;
    if !response.status().is_success() {
        return Err(format!(
            "Inspector returned HTTP {}. The relay may have restarted or this exchange was evicted.",
            response.status().as_u16()
        ));
    }
    let mut bytes = vec![];
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Inspector response was interrupted.")?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Inspector response exceeds the local display budget.".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Inspector returned an unsupported response.".into())
}

pub fn start(path: PathBuf, mailbox: Arc<Mutex<Mailbox>>, waker: rune::Waker) {
    std::thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(2))
            .build()
        {
            Ok(client) => client,
            Err(_) => {
                mailbox.lock().expect("mailbox").update =
                    Some(Err("Cannot initialize the inspector HTTP client.".into()));
                waker();
                return;
            }
        };
        let mut previous = None;
        loop {
            let selected = {
                let mut shared = mailbox.lock().expect("mailbox");
                if shared.stop {
                    return;
                }
                if shared.refresh {
                    previous = None;
                    shared.refresh = false;
                }
                shared.selected
            };
            let result = (|| -> Result<Option<Update>, String> {
                let connection = connection(&path)?;
                let snapshot: Snapshot = get(&client, &connection, "/api/requests")?;
                if snapshot.version != PROTOCOL_VERSION {
                    return Err("Inspector protocol version mismatch.".into());
                }
                let signature = (snapshot.instance_id.clone(), snapshot.revision, selected);
                if previous.as_ref() == Some(&signature) {
                    return Ok(None);
                }
                let detail = selected.map(|id| {
                    (
                        id,
                        get(&client, &connection, &format!("/api/requests/{id}")),
                    )
                });
                previous = if detail.as_ref().is_some_and(|(_, value)| value.is_err()) {
                    None
                } else {
                    Some(signature)
                };
                Ok(Some(Update { snapshot, detail }))
            })();
            match result {
                Ok(Some(update)) => {
                    mailbox.lock().expect("mailbox").update = Some(Ok(update));
                    waker();
                }
                Err(error) => {
                    previous = None;
                    mailbox.lock().expect("mailbox").update = Some(Err(error));
                    waker();
                }
                Ok(None) => {}
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}
