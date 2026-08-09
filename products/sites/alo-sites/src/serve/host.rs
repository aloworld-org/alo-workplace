//! Host header → public site host, the resolution step that scopes a public
//! request to one tenant's site (`docs/design/sites.md`, Tenancy). Strict on
//! purpose: only `<label>.<SITES_DOMAIN>` where the label passes the store's
//! subdomain rules resolves — nested labels, the apex itself, IP literals,
//! and malformed authorities fall through to the generic not-found. A valid
//! host outside the configured apex is a custom-domain lookup key.

/// A validated public routing key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// One built-in `<label>.<SITES_DOMAIN>` host.
    Subdomain { label: String, host: String },
    /// One canonical custom host.
    Custom { host: String },
}

impl Scope {
    /// The canonical public host, without a port or trailing root dot.
    #[must_use]
    pub fn host(&self) -> &str {
        match self {
            Self::Subdomain { host, .. } | Self::Custom { host } => host,
        }
    }
}

/// Extracts the site subdomain from a request's Host header value, given the
/// configured apex (already lowercase). Ports and a trailing FQDN dot are
/// ignored; matching is case-insensitive; anything that is not exactly one
/// valid subdomain label under the apex is `None`.
pub fn subdomain(host: &str, sites_domain: &str) -> Option<String> {
    let host = canonical(host)?;
    let label = host.strip_suffix(sites_domain)?.strip_suffix('.')?;
    // The store's rules also exclude `.`, so `a.b.<apex>` can never match.
    alo_store::validate_subdomain(label).ok()?;
    Some(label.to_owned())
}

/// Canonicalizes a request authority to a safe DNS host. Ports and a trailing
/// FQDN dot are ignored; schemes, IP literals and malformed DNS names fail.
pub fn canonical(host: &str) -> Option<String> {
    let host = host.trim();
    // An IPv6 authority (`[::1]:8081`) is never a site host.
    if host.starts_with('[') {
        return None;
    }
    let host = match host.rsplit_once(':') {
        Some((name, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    };
    let host = host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase();
    alo_store::normalize_site_domain(&host).ok()
}

/// Classifies a validated authority as a built-in subdomain or custom host.
pub fn scope(host: &str, sites_domain: &str) -> Option<Scope> {
    let host = canonical(host)?;
    if let Some(label) = subdomain(&host, sites_domain) {
        Some(Scope::Subdomain { label, host })
    } else {
        Some(Scope::Custom { host })
    }
}

#[cfg(test)]
mod tests {
    use super::{Scope, canonical, scope, subdomain};

    const APEX: &str = "alosites.test";

    #[test]
    fn resolves_exactly_one_valid_label_under_the_apex() {
        assert_eq!(
            subdomain("acme.alosites.test", APEX).as_deref(),
            Some("acme")
        );
        assert_eq!(
            subdomain("ACME.AloSites.Test", APEX).as_deref(),
            Some("acme"),
            "host matching is case-insensitive"
        );
        assert_eq!(
            subdomain("acme.alosites.test:8081", APEX).as_deref(),
            Some("acme"),
            "a port is ignored"
        );
        assert_eq!(
            subdomain("acme.alosites.test.", APEX).as_deref(),
            Some("acme"),
            "a trailing FQDN dot is ignored"
        );
    }

    #[test]
    fn everything_else_falls_through() {
        // The apex itself, nested labels, other domains, lookalike suffixes.
        assert_eq!(subdomain("alosites.test", APEX), None);
        assert_eq!(subdomain("a.b.alosites.test", APEX), None);
        assert_eq!(subdomain("acme.example.com", APEX), None);
        assert_eq!(subdomain("acme.evilalosites.test", APEX), None);
        // Labels the store would never have admitted.
        assert_eq!(subdomain("-x-.alosites.test", APEX), None);
        assert_eq!(
            subdomain("ab.alosites.test", APEX),
            None,
            "below min length"
        );
        // Degenerate authorities.
        assert_eq!(subdomain("", APEX), None);
        assert_eq!(subdomain("[::1]:8081", APEX), None);
        assert_eq!(subdomain("127.0.0.1:8081", APEX), None);
        assert_eq!(subdomain(".alosites.test", APEX), None);
    }

    #[test]
    fn canonicalizes_and_classifies_custom_hosts() {
        assert_eq!(
            canonical("WWW.Example.COM.:443").as_deref(),
            Some("www.example.com")
        );
        assert_eq!(canonical("https://example.com"), None);
        assert_eq!(canonical("127.0.0.1:8081"), None);
        assert_eq!(
            scope("acme.alosites.test", APEX),
            Some(Scope::Subdomain {
                label: "acme".to_owned(),
                host: "acme.alosites.test".to_owned(),
            })
        );
        assert_eq!(
            scope("www.example.com", APEX),
            Some(Scope::Custom {
                host: "www.example.com".to_owned(),
            })
        );
    }
}
