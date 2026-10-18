//! Download eines Modells – nur nach ausdrücklicher Zustimmung des Nutzers (der Aufrufer zeigt
//! vorher Name, Quelle, Größe und Lizenz an).
//!
//! * Fortschritt und Abbrechen über [`Progress`].
//! * Die Datei entsteht zuerst als `*.part` und wird erst nach erfolgreicher SHA-256-Prüfung
//!   umbenannt; bei Fehler/Abbruch wird die Teil-Datei gelöscht.
//! * Gesperrt, wenn [`disable`] aufgerufen wurde oder `FP_NO_SHELL_OPEN`/`FP_NO_DOWNLOAD` gesetzt
//!   ist (Tests, Stapelbetrieb).

use crate::cancel::Progress;
use crate::error::{AiError, Result};
use crate::models::{self, ModelId};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static DISABLED: AtomicBool = AtomicBool::new(false);

/// Download für diesen Prozess sperren (Tests).
pub fn disable() {
    DISABLED.store(true, Ordering::SeqCst);
}

/// Ist ein Download erlaubt?
pub fn allowed() -> bool {
    !DISABLED.load(Ordering::SeqCst)
        && std::env::var_os("FP_NO_SHELL_OPEN").is_none()
        && std::env::var_os("FP_NO_DOWNLOAD").is_none()
}

/// SHA-256 einer Datei (hex, klein).
pub fn sha256_file(p: &Path, prog: Option<&Progress>) -> Result<String> {
    let mut f = std::fs::File::open(p)?;
    let total = f.metadata().map(|m| m.len()).unwrap_or(0).max(1);
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        done += n as u64;
        if let Some(pr) = prog {
            pr.set(done as f32 / total as f32);
            pr.cancel.check()?;
        }
    }
    Ok(hex(&h.finalize()))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Prüft eine vorhandene Modelldatei gegen die Prüfsumme des Katalogs.
pub fn verify(id: ModelId, prog: Option<&Progress>) -> Result<PathBuf> {
    let m = models::info(id);
    let p = models::find(id).ok_or_else(|| models::missing(id))?;
    let got = sha256_file(&p, prog)?;
    if got != m.sha256 {
        return Err(AiError::Checksum {
            file: p.display().to_string(),
            expected: m.sha256.into(),
            got,
        });
    }
    Ok(p)
}

/// Lädt ein Modell herunter (blockierend; im Hintergrund-Thread aufrufen). Liefert den Pfad.
pub fn download(id: ModelId, prog: &Progress) -> Result<PathBuf> {
    if !allowed() {
        return Err(AiError::DownloadDisabled);
    }
    let m = models::info(id);
    let folder = models::download_folder();
    fp_i18n::profile::guard_write(&folder);
    std::fs::create_dir_all(&folder)?;
    let target = folder.join(m.file);
    let part = folder.join(format!("{}.part", m.file));
    let r = fetch(m.url, &part, m.size, m.sha256, prog);
    match r {
        Ok(()) => {
            if target.exists() {
                let _ = std::fs::remove_file(&target);
            }
            std::fs::rename(&part, &target)?;
            Ok(target)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

fn fetch(url: &str, part: &Path, size: u64, sha: &str, prog: &Progress) -> Result<()> {
    // Windows: SChannel via native-tls; elsewhere rustls (see Cargo.toml), no OpenSSL.
    #[cfg(windows)]
    let provider = ureq::tls::TlsProvider::NativeTls;
    #[cfg(not(windows))]
    let provider = ureq::tls::TlsProvider::Rustls;
    let tls = ureq::tls::TlsConfig::builder().provider(provider).build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_connect(Some(std::time::Duration::from_secs(30)))
        .user_agent("Firepixel")
        .build()
        .into();
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| AiError::Download(e.to_string()))?;
    let mut reader = resp.into_body().into_reader();
    let mut f = std::fs::File::create(part)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    loop {
        prog.cancel.check()?;
        let n = reader
            .read(&mut buf)
            .map_err(|e| AiError::Download(e.to_string()))?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n])?;
        h.update(&buf[..n]);
        done += n as u64;
        prog.set(done as f32 / size.max(1) as f32);
    }
    f.flush()?;
    drop(f);
    if done != size {
        return Err(AiError::Download(fp_i18n::tr!(
            "ai-err-d-download-size",
            done = done.to_string(),
            size = size.to_string()
        )));
    }
    let got = hex(&h.finalize());
    if got != sha {
        return Err(AiError::Checksum {
            file: part.display().to_string(),
            expected: sha.into(),
            got,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_means_no_network() {
        disable();
        let p = Progress::new();
        assert!(matches!(
            download(ModelId::Sam2Tiny, &p),
            Err(AiError::DownloadDisabled)
        ));
    }

    #[test]
    fn sha_of_known_bytes() {
        let dir = std::env::temp_dir().join(format!("fp-ai-sha-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("abc.txt");
        let _ = std::fs::write(&f, b"abc");
        assert_eq!(
            sha256_file(&f, None).unwrap_or_default(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
