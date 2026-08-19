use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::io::AsyncWriteExt;
use tracing::warn;

use crate::error::{MusicError, Result};

const NETSCAPE_HEADERS: [&str; 2] =
    ["netscape http cookie file", "http cookie file"];

const HTTP_ONLY_PREFIX: &str = "#HttpOnly_";
const LOGIN_COOKIE: &str = "LOGIN_INFO";
const SID_COOKIES: [&str; 3] = ["SAPISID", "__Secure-3PAPISID", "__Secure-1PAPISID"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JarStatus {
    Authenticated { expires: Option<i64> },
    Anonymous,
}

impl JarStatus {
    #[must_use]
    pub const fn is_authenticated(self) -> bool {
        matches!(self, Self::Authenticated { .. })
    }

    #[must_use]
    pub const fn expires_in(self, now_unix: i64) -> Option<i64> {
        match self {
            Self::Authenticated { expires: Some(at) } => {
                Some(at.saturating_sub(now_unix))
            },
            Self::Authenticated { expires: None } | Self::Anonymous => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cookie<'a> {
    pub domain: &'a str,
    pub name: &'a str,
    pub expires: Option<i64>,
    pub http_only: bool,
}

#[derive(Debug)]
pub struct CookieJar {
    path: PathBuf,
    status: JarStatus,
}

impl CookieJar {
    pub fn open(path: PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(&path).map_err(|e| {
            MusicError::Internal(format!(
                "could not read the YouTube cookie file at {}: {e}",
                path.display()
            ))
        })?;

        if !has_netscape_header(&contents) {
            return Err(MusicError::Internal(format!(
                "{} is not a Netscape cookie file (its first line must be `# \
                 Netscape HTTP Cookie File`); export it with a cookies.txt \
                 browser extension rather than copying it out of devtools",
                path.display()
            )));
        }

        Ok(Self { path, status: jar_status(&contents) })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub const fn status(&self) -> JarStatus {
        self.status
    }

    pub async fn lease(&self) -> Result<CookieLease> {
        let contents = tokio::fs::read(&self.path).await.map_err(|e| {
            MusicError::Resolve(format!(
                "could not read the YouTube cookie file at {}: {e}",
                self.path.display()
            ))
        })?;

        let path = lease_path();
        let arg = path
            .to_str()
            .ok_or_else(|| {
                MusicError::Resolve(format!(
                    "the temporary cookie path {} is not valid UTF-8",
                    path.display()
                ))
            })?
            .to_owned();

        write_private(&path, &contents).await?;

        Ok(CookieLease { path, arg })
    }
}

#[derive(Debug)]
pub struct CookieLease {
    path: PathBuf,
    arg: String,
}

impl CookieLease {
    #[must_use]
    pub fn arg(&self) -> &str {
        &self.arg
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for CookieLease {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {},
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => warn!(
                path = %self.path.display(),
                "could not remove the temporary YouTube cookie copy: {e}"
            ),
        }
    }
}

#[must_use]
pub fn has_netscape_header(contents: &str) -> bool {
    contents
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .and_then(|line| line.strip_prefix('#'))
        .map(|rest| rest.trim().to_ascii_lowercase())
        .is_some_and(|rest| {
            NETSCAPE_HEADERS.iter().any(|header| rest.starts_with(header))
        })
}

#[must_use]
pub fn parse_netscape(contents: &str) -> Vec<Cookie<'_>> {
    contents.lines().filter_map(parse_line).collect()
}

#[must_use]
pub fn jar_status(contents: &str) -> JarStatus {
    let cookies = parse_netscape(contents);

    let youtube = || cookies.iter().filter(|c| is_youtube_domain(c.domain));

    let login = youtube().find(|c| c.name == LOGIN_COOKIE);
    let sids: Vec<&Cookie<'_>> =
        youtube().filter(|c| SID_COOKIES.contains(&c.name)).collect();

    let (Some(login), Some(_)) = (login, sids.first()) else {
        return JarStatus::Anonymous;
    };

    let expires =
        std::iter::once(login).chain(sids).filter_map(|cookie| cookie.expires).min();

    JarStatus::Authenticated { expires }
}

#[must_use]
pub fn cookie_warning(stderr: &str) -> Option<&str> {
    stderr.lines().map(str::trim).find(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("cookie")
            && (lower.contains("no longer valid")
                || lower.contains("rotated")
                || lower.contains("expired")
                || lower.contains("not a valid netscape"))
    })
}

fn parse_line(line: &str) -> Option<Cookie<'_>> {
    let (line, http_only) = line
        .strip_prefix(HTTP_ONLY_PREFIX)
        .map_or((line, false), |rest| (rest, true));

    let line = line.trim_end_matches(['\r', '\n']);
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let mut fields = line.split('\t');
    let domain = fields.next()?;
    let expires = fields.nth(3)?;
    let name = fields.next()?;

    Some(Cookie {
        domain,
        name,
        expires: expires.parse().ok().filter(|at| *at > 0),
        http_only,
    })
}

fn is_youtube_domain(domain: &str) -> bool {
    matches!(domain.trim_start_matches('.'), "youtube.com" | "www.youtube.com")
}

fn lease_path() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);

    let seq = NEXT.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();

    std::env::temp_dir().join(format!("zayden-yt-cookies-{pid}-{seq}.txt"))
}

async fn write_private(path: &Path, contents: &[u8]) -> Result<()> {
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    options.mode(0o600);

    let mut file = options.open(path).await.map_err(|e| {
        MusicError::Resolve(format!(
            "could not create the temporary cookie copy at {}: {e}",
            path.display()
        ))
    })?;

    file.write_all(contents).await.map_err(|e| {
        MusicError::Resolve(format!(
            "could not write the temporary cookie copy at {}: {e}",
            path.display()
        ))
    })?;

    file.flush().await.map_err(|e| {
        MusicError::Resolve(format!(
            "could not flush the temporary cookie copy at {}: {e}",
            path.display()
        ))
    })
}
