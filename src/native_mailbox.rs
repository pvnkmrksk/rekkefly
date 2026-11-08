//! Opt-in, reconnectable local control of the existing native window/engine.
//! Requests carry an endpoint instance identity and expiry. No network listener.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_BYTES: u64 = 65_536;
pub struct Mailbox {
    directory: PathBuf,
    instance: String,
    replies: BTreeMap<String, (u64, Value)>,
}
fn now_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .map_err(|e| e.to_string())
}
fn atomic(path: &Path, value: &Value) -> Result<(), String> {
    let temp = path.with_extension("writing");
    fs::write(&temp, serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}
impl Mailbox {
    pub fn new(directory: PathBuf, instance: String, backend: &str) -> Result<Self, String> {
        if !directory.is_absolute()
            || instance.len() != 32
            || !instance.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("control directory must be absolute and control instance must be 32 hexadecimal characters".into());
        }
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let descriptor = directory.join("endpoint.json");
        if descriptor.exists() {
            let old: Value =
                serde_json::from_slice(&fs::read(&descriptor).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if old["instance"].as_str() != Some(instance.as_str()) {
                return Err("control directory belongs to another tracker instance; choose a fresh directory".into());
            }
        }
        atomic(
            &descriptor,
            &json!({"protocolVersion":1,"instance":instance,"backend":backend,"processId":std::process::id(),
            "startedUtcMilliseconds":now_ms()?,"capabilities":["native-gui-control","per-take-finalization", "owned-take-stop-v1"],
            "commands":["status","record_start","record_stop","start","pause"],
            "semantics":"Commands may queue native worker changes. Inspect recording state and the exact take finalization receipt; queued or inactive is not completion."}),
        )?;
        Ok(Self {
            directory,
            instance,
            replies: BTreeMap::new(),
        })
    }
    pub fn poll(
        &mut self,
        mut execute: impl FnMut(&str, &Value) -> Result<Value, String>,
    ) -> Result<(), String> {
        let current = now_ms()?;
        self.replies.retain(|_, (expiry, _)| *expiry >= current);
        let mut requests: Vec<_> = fs::read_dir(&self.directory)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                    n.strip_suffix(".request.json").is_some_and(|id| {
                        id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
                    })
                })
            })
            .collect();
        requests.sort();
        for path in requests.into_iter().take(8) {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let id = name.trim_end_matches(".request.json");
            if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            let response_path = self.directory.join(format!("{id}.response.json"));
            if let Some((_, cached)) = self.replies.get(id) {
                atomic(&response_path, cached)?;
                let _ = fs::remove_file(&path);
                continue;
            }
            let outcome = (|| {
                if self.replies.len() >= 4096 {
                    return Err(
                        "control request history is full; wait for expired requests to clear"
                            .into(),
                    );
                }
                if fs::metadata(&path).map_err(|e| e.to_string())?.len() > MAX_BYTES {
                    return Err("request exceeds 64 KiB".into());
                }
                let req: Value =
                    serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                if req["id"].as_str() != Some(id)
                    || req["instance"].as_str() != Some(self.instance.as_str())
                {
                    return Err("request does not match this endpoint instance".into());
                }
                let expires = req["expiresUtcMilliseconds"]
                    .as_u64()
                    .ok_or("request expiry is required")?;
                let now = now_ms()?;
                if expires < now || expires > now.saturating_add(120_000) {
                    return Err("request expired or expiry exceeds two minutes".into());
                }
                let command = req["command"].as_str().ok_or("command is required")?;
                if !matches!(
                    command,
                    "status" | "record_start" | "record_stop" | "start" | "pause"
                ) {
                    return Err("unsupported recording-control command".into());
                }
                let args = req.get("args").cloned().unwrap_or_else(|| json!({}));
                if !args.is_object() {
                    return Err("args must be an object".into());
                }
                execute(command, &args)
            })();
            let response = match outcome {
                Ok(value) => json!({"id":id,"instance":self.instance,"ok":true,"result":value}),
                Err(error) => json!({"id":id,"instance":self.instance,"ok":false,"error":error}),
            };
            self.replies.insert(
                id.to_owned(),
                (current.saturating_add(120_000), response.clone()),
            );
            atomic(&response_path, &response)?;
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
