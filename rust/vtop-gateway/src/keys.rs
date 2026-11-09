//! The gateway's own Noise keys: a new pair every Monday 00:00 IST, the
//! one before kept for a week, older ones deleted. Kept in
//! `<data dir>/udp-keys.json` (owner-only), so a restart keeps them. Only
//! the public halves leave the gateway, posted to the edge ([`Announce`]).

use std::path::{Path, PathBuf};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::{Deserialize, Serialize};

pub const WEEK_SECS: u64 = 7 * 24 * 3600;
/// A Monday 00:00 IST (Sunday 18:30 UTC); keys turn over a whole number of
/// weeks from it.
const MONDAY_IST: u64 = 1_704_652_200;
const FILE: &str = "udp-keys.json";

/// The Monday 00:00 IST at or before `now`.
pub fn week_start(now: u64) -> u64 {
    now - now.saturating_sub(MONDAY_IST) % WEEK_SECS
}

/// One key pair, made for the week starting at `from`. No `Debug`: it
/// holds the private key.
#[derive(Clone, Serialize, Deserialize)]
pub struct Key {
    pub from: u64,
    /// Hex.
    private: String,
}

impl Key {
    pub fn private(&self) -> [u8; 32] {
        vtop_wire::unhex(&self.private)
            .and_then(|bytes| bytes.try_into().ok())
            .expect("checked on load")
    }

    pub fn public(&self) -> [u8; 32] {
        crate::public_key(&self.private())
    }

    /// The gateway stops taking it a week after the next key comes.
    pub fn until(&self) -> u64 {
        self.from + 2 * WEEK_SECS
    }

    /// A short name for logs and the edge: the public key's first 8 bytes.
    pub fn id(&self) -> String {
        vtop_wire::hex(&self.public()[..8])
    }
}

/// The keys in `dir`, newest first, after making this week's and dropping
/// those older than last week's. Saved when anything changed.
pub fn rotate(dir: &Path, now: u64) -> Result<Vec<Key>, String> {
    let path = dir.join(FILE);
    let mut keys: Vec<Key> = match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("{} does not read: {e}", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    let before = keys.len();
    let week = week_start(now);
    keys.retain(|key| {
        key.from + WEEK_SECS >= week
            && key.from <= week
            && vtop_wire::unhex(&key.private).is_some_and(|bytes| bytes.len() == 32)
    });
    let mut changed = keys.len() != before;
    if !keys.iter().any(|key| key.from == week) {
        let (private, _) = vtop_wire::generate_keypair();
        keys.push(Key {
            from: week,
            private: vtop_wire::hex(&private),
        });
        changed = true;
    }
    keys.sort_by_key(|key| std::cmp::Reverse(key.from));
    if changed {
        save(dir, &path, &keys)?;
    }
    Ok(keys)
}

fn save(dir: &Path, path: &PathBuf, keys: &[Key]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let temp = dir.join(format!("{FILE}.tmp"));
    let json = serde_json::to_vec(keys).map_err(|e| e.to_string())?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let write = |options: &std::fs::OpenOptions| -> std::io::Result<()> {
        use std::io::Write as _;
        let mut file = options.open(&temp)?;
        file.write_all(&json)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)
    };
    write(&options).map_err(|e| format!("cannot save {}: {e}", path.display()))
}

/// What the gateway posts to the edge's `/internal/udp-keys`.
#[derive(Debug, Serialize)]
pub struct Announce {
    pub host: String,
    pub ports: Vec<u16>,
    pub keys: Vec<PublicKey>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKey {
    pub id: String,
    /// The X25519 public key, base64url without padding.
    pub public_key: String,
    pub from: u64,
    pub until: u64,
}

impl Announce {
    pub fn new(host: &str, ports: &[u16], keys: &[Key]) -> Self {
        Self {
            host: host.to_string(),
            ports: ports.to_vec(),
            keys: keys
                .iter()
                .map(|key| PublicKey {
                    id: key.id(),
                    public_key: URL_SAFE_NO_PAD.encode(key.public()),
                    from: key.from,
                    until: key.until(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mon 2 Nov 2026 00:00 IST.
    const MONDAY: u64 = 1_793_557_800;

    fn dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("vtop-gateway-keys-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn weeks_start_on_monday_ist() {
        assert_eq!(week_start(MONDAY), MONDAY);
        assert_eq!(week_start(MONDAY + 3 * 86_400), MONDAY);
        assert_eq!(week_start(MONDAY - 1), MONDAY - WEEK_SECS);
    }

    #[test]
    fn keeps_last_weeks_key_a_week_and_then_deletes_it() {
        let dir = dir("rotate");
        let first = rotate(&dir, MONDAY + 60).unwrap();
        assert_eq!(first.len(), 1);
        // A restart in the same week keeps the same key.
        let again = rotate(&dir, MONDAY + 86_400).unwrap();
        assert_eq!(again[0].private(), first[0].private());

        let next = rotate(&dir, MONDAY + WEEK_SECS).unwrap();
        assert_eq!(next.len(), 2);
        assert_eq!(next[0].from, MONDAY + WEEK_SECS);
        assert_eq!(next[1].private(), first[0].private());

        let later = rotate(&dir, MONDAY + 2 * WEEK_SECS + 5).unwrap();
        assert_eq!(later.len(), 2);
        assert!(later.iter().all(|key| key.private() != first[0].private()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn announces_only_public_keys() {
        let dir = dir("announce");
        let keys = rotate(&dir, MONDAY).unwrap();
        let body = serde_json::to_value(Announce::new("udp.example", &[3478], &keys)).unwrap();
        let key = &body["keys"][0];
        assert_eq!(body["host"], "udp.example");
        assert_eq!(body["ports"], serde_json::json!([3478]));
        assert_eq!(
            (key["from"].as_u64(), key["until"].as_u64()),
            (Some(MONDAY), Some(MONDAY + 2 * WEEK_SECS))
        );
        let public = URL_SAFE_NO_PAD
            .decode(key["publicKey"].as_str().unwrap())
            .unwrap();
        assert_eq!(public, keys[0].public());
        assert!(!body
            .to_string()
            .contains(&vtop_wire::hex(&keys[0].private())));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
