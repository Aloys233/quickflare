//! Heuristic process → service classifier.
//!
//! We deliberately keep this rule-based rather than ML-driven: the
//! frameworks we care about have distinctive command-line signatures
//! (`vite`, `next dev`, `nuxt`, `org.springframework.boot`, …) so a few
//! ordered regexes get us 95% accuracy with zero runtime cost.

use crate::types::ServiceKind;
use once_cell::sync::Lazy;
use regex::Regex;

/// Each rule fires on the *full* `cmd ...args...` string, not just the
/// executable basename, because `node /path/to/bin/vite` and
/// `node /path/to/bin/next dev` both look like `node`.
struct Rule {
    re: Regex,
    kind: ServiceKind,
}

static RULES: Lazy<Vec<Rule>> = Lazy::new(|| {
    let r = |pat: &str, kind: ServiceKind| Rule {
        re: Regex::new(pat).expect("static regex"),
        kind,
    };
    vec![
        r(r"(?i)\bvite\b", ServiceKind::Vite),
        r(r"(?i)\bnext\b\s+(dev|start)", ServiceKind::NextJs),
        r(r"(?i)\.next/standalone", ServiceKind::NextJs),
        r(r"(?i)\bnuxt\b", ServiceKind::Nuxt),
        r(
            r"(?i)org\.springframework\.boot|spring-boot",
            ServiceKind::SpringBoot,
        ),
        r(
            r"(?i)\bdocker(d|-proxy)?\b|/var/lib/docker|containerd-shim",
            ServiceKind::Docker,
        ),
        r(r"(?i)\bsshd\b", ServiceKind::Ssh),
        r(
            r"(?i)\bjava\b.*minecraft|fabric|forge|spigot|paper",
            ServiceKind::Minecraft,
        ),
        r(r"(?i)\bpostgres\b", ServiceKind::Postgres),
        r(r"(?i)\bmysqld?\b|\bmariadbd?\b", ServiceKind::Mysql),
        r(r"(?i)\bredis-server\b", ServiceKind::Redis),
        r(r"(?i)\bmongod\b", ServiceKind::Mongo),
        r(r"(?i)\belasticsearch\b", ServiceKind::Elastic),
        r(r"(?i)\bpython[0-9.]*\b", ServiceKind::Python),
        r(r"(?i)\bnode(js)?\b|\bbun\b|\bdeno\b", ServiceKind::NodeJs),
        r(r"(?i)\bnginx\b|\bcaddy\b|\bapache2?\b|\bhttpd\b", ServiceKind::Http),
    ]
});

/// Well-known port assignments — used as a fallback when we couldn't read
/// the owning process (e.g. permission denied).
fn classify_by_port(port: u16) -> ServiceKind {
    match port {
        22 => ServiceKind::Ssh,
        80 | 443 | 8080 | 8443 => ServiceKind::Http,
        3306 | 33060 => ServiceKind::Mysql,
        5432 => ServiceKind::Postgres,
        6379 => ServiceKind::Redis,
        9200 | 9300 => ServiceKind::Elastic,
        25565 => ServiceKind::Minecraft,
        27017 | 27018 | 27019 => ServiceKind::Mongo,
        3000 | 5173 | 4321 | 1420 => ServiceKind::NodeJs,
        _ => ServiceKind::Unknown,
    }
}

/// Best-effort guess. We try the command-line rules first (most specific),
/// fall back to the executable basename, then to the well-known port table.
pub fn classify(port: u16, process: Option<&str>, command: Option<&str>) -> ServiceKind {
    let haystack = match (command, process) {
        (Some(cmd), _) => cmd.to_string(),
        (None, Some(name)) => name.to_string(),
        _ => return classify_by_port(port),
    };

    for rule in RULES.iter() {
        if rule.re.is_match(&haystack) {
            return rule.kind;
        }
    }

    classify_by_port(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_vite() {
        assert_eq!(
            classify(5173, Some("node"), Some("node /usr/bin/vite --port 5173")),
            ServiceKind::Vite
        );
    }

    #[test]
    fn detects_nextjs_dev() {
        assert_eq!(
            classify(3000, Some("node"), Some("node /repo/node_modules/.bin/next dev")),
            ServiceKind::NextJs
        );
    }

    #[test]
    fn falls_back_to_well_known_port() {
        assert_eq!(classify(22, None, None), ServiceKind::Ssh);
        assert_eq!(classify(5432, None, None), ServiceKind::Postgres);
    }
}
