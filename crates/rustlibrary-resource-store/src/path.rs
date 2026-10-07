use crate::{CasePolicy, Error, Limits, Result};

pub(crate) fn normalize(path: &str, policy: CasePolicy, limits: Limits) -> Result<String> {
    if path.len() > limits.max_path_bytes {
        return Err(Error::LimitExceeded("path bytes"));
    }
    if path.is_empty() {
        return Ok(String::new());
    }
    if path.chars().any(|c| c.is_control() || "\\:%?#".contains(c))
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(Error::InvalidPath);
    }
    if path.split('/').count() > limits.max_depth {
        return Err(Error::LimitExceeded("path depth"));
    }
    Ok(match policy {
        CasePolicy::Sensitive => path.to_owned(),
        CasePolicy::AsciiInsensitive => path.to_ascii_lowercase(),
    })
}

pub(crate) fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

pub(crate) fn below(path: &str, directory: &str) -> bool {
    !path.is_empty()
        && (directory.is_empty()
            || path
                .strip_prefix(directory)
                .is_some_and(|s| s.starts_with('/')))
}
