//! Normalization for arena base URLs used by outbound WASI HTTP.

/// Prepares the arena base URL for outbound WASI HTTP.
///
/// - Adds `http://` when missing (relative URLs break `wasi:http` outbound).
/// - Strips a trailing `/` so `{base}/message` is a single slash path.
/// - Rewrites the host `localhost` (any ASCII case) to **`127.0.0.1`**. `arena-stub` binds IPv4
///   only; resolving `localhost` can yield **`::1`**, so requests never reach port 3000.
pub fn normalize_arena_base_url(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .trim_end_matches('/');
    let with_scheme = if trimmed
        .get(..7)
        .is_some_and(|p| p.eq_ignore_ascii_case("http://"))
        || trimmed
            .get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case("https://"))
    {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };

    let with_scheme = normalize_http_scheme_prefix_case(with_scheme);
    replace_localhost_host_with_loopback(&with_scheme)
}

fn normalize_http_scheme_prefix_case(s: String) -> String {
    if s.len() >= 8
        && s.get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case("https://"))
        && !s.starts_with("https://")
    {
        let mut out = String::with_capacity(s.len());
        out.push_str("https://");
        out.push_str(
            s.get(8..)
                .unwrap_or(""),
        );
        return out;
    }
    if s.len() >= 7
        && s.get(..7)
            .is_some_and(|p| p.eq_ignore_ascii_case("http://"))
        && !s.starts_with("http://")
    {
        let mut out = String::with_capacity(s.len());
        out.push_str("http://");
        out.push_str(
            s.get(7..)
                .unwrap_or(""),
        );
        return out;
    }
    s
}

fn replace_localhost_host_with_loopback(s: &str) -> String {
    let Some(scheme_sep) = s.find("://") else {
        return s.to_string();
    };
    let host_start = scheme_sep.saturating_add(3);
    let rest = s
        .get(host_start..)
        .unwrap_or("");
    let host_len = rest
        .find([':', '/', '?', '#'])
        .unwrap_or(rest.len());
    let host = rest
        .get(..host_len)
        .unwrap_or("");
    if !host.eq_ignore_ascii_case("localhost") {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    out.push_str(
        s.get(..host_start)
            .unwrap_or(s),
    );
    out.push_str("127.0.0.1");
    out.push_str(
        rest.get(host_len..)
            .unwrap_or(""),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::normalize_arena_base_url;

    #[test]
    fn adds_scheme_and_strips_slash() {
        assert_eq!(normalize_arena_base_url(" 127.0.0.1:3000/ "), "http://127.0.0.1:3000");
    }

    #[test]
    fn localhost_becomes_loopback_ipv4() {
        assert_eq!(normalize_arena_base_url("http://localhost:3000"), "http://127.0.0.1:3000");
        assert_eq!(normalize_arena_base_url("HTTP://LOCALHOST:3000/"), "http://127.0.0.1:3000");
    }

    #[test]
    fn leaves_non_localhost_unchanged() {
        assert_eq!(normalize_arena_base_url("http://127.0.0.1:3000"), "http://127.0.0.1:3000");
    }
}
