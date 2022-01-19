use crate::{handler::Params, Error};

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum RoutePart {
    PathComponent(String),
    Param(String),
    Leader,
}

#[derive(Debug, Clone, PartialOrd)]
pub struct Path(Vec<RoutePart>);

impl Eq for Path {}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl Path {
    pub(crate) fn new(path: String) -> Self {
        let mut parts = Self::default();

        for arg in path.split("/") {
            if arg.starts_with(":") {
                // is param
                parts.push(RoutePart::Param(arg.trim_start_matches(":").to_string()));
            } else if arg == "" {
                // skip empties. this will push additional leaders if there is an duplicate slash
                // (e.g.: `//one/two`), which will fail on matching; we don't want to support this
                // syntax in the router.
                parts.push(RoutePart::Leader);
            } else {
                // is not param
                parts.push(RoutePart::PathComponent(arg.to_string()));
            }
        }

        parts
    }

    pub(crate) fn push(&mut self, arg: RoutePart) -> Self {
        self.0.push(arg);
        self.clone()
    }

    #[allow(dead_code)]
    pub(crate) fn params(&self) -> Vec<String> {
        let mut params = Vec::new();
        for arg in self.0.clone() {
            match arg {
                RoutePart::Param(p) => params.push(p),
                _ => {}
            }
        }

        params
    }

    #[allow(dead_code)]
    pub(crate) fn extract(&self, provided: String) -> Result<Params, Error> {
        let parts: Vec<String> = provided
            .split("/")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let mut params = Params::default();

        if parts.len() != self.0.len() {
            return Err(Error::new("invalid parameters"));
        }

        let mut i = 0;

        for part in self.0.clone() {
            match part {
                RoutePart::Param(p) => params.insert(p, parts[i].clone()),
                RoutePart::PathComponent(part) => {
                    if part != parts[i] {
                        return Err(Error::new("invalid path for parameter extraction"));
                    }

                    None
                }
                RoutePart::Leader => None,
            };

            i += 1
        }

        Ok(params)
    }

    pub(crate) fn matches(&self, s: String) -> bool {
        self.eq(&Self::new(s))
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        if other.0.len() != self.0.len() {
            return false;
        }

        let mut i = 0;
        let mut leader_seen = false;
        for arg in other.0.clone() {
            let res = match self.0[i].clone() {
                RoutePart::PathComponent(_) => self.0[i] == arg,
                RoutePart::Param(_param) => {
                    // FIXME advanced parameter shit here later
                    true
                }
                RoutePart::Leader => {
                    if leader_seen {
                        false
                    } else {
                        leader_seen = true;
                        true
                    }
                }
            };

            if !res {
                return false;
            }

            i += 1;
        }

        true
    }
}

impl Default for Path {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl ToString for Path {
    fn to_string(&self) -> String {
        let mut s = Vec::new();

        for part in self.0.clone() {
            s.push(match part {
                RoutePart::PathComponent(pc) => pc.to_string(),
                RoutePart::Param(param) => {
                    format!(":{}", param)
                }
                RoutePart::Leader => "".to_string(),
            });
        }

        if s.len() < 2 {
            return "/".to_string();
        }

        s.join("/")
    }
}

mod tests {
    #[test]
    fn test_path() {
        use super::Path;
        use std::collections::BTreeMap;

        let path = Path::new("/abc/def/ghi".to_string());
        assert!(path.matches("/abc/def/ghi".to_string()));
        assert!(!path.matches("//abc/def/ghi".to_string()));
        assert!(!path.matches("/def/ghi".to_string()));
        assert!(path.params().is_empty());

        let path = Path::new("/abc/:def/:ghi/jkl".to_string());
        assert!(!path.matches("/abc/def/ghi".to_string()));
        assert!(path.matches("/abc/def/ghi/jkl".to_string()));
        assert!(path.matches("/abc/ghi/def/jkl".to_string()));
        assert!(path.matches("/abc/wooble/wakka/jkl".to_string()));
        assert!(!path.matches("/nope/ghi/def/jkl".to_string()));
        assert!(!path.matches("/abc/ghi/def/nope".to_string()));
        assert_eq!(path.params().len(), 2);

        let mut bt = BTreeMap::new();
        bt.insert("def".to_string(), "wooble".to_string());
        bt.insert("ghi".to_string(), "wakka".to_string());

        assert_eq!(
            path.extract("/abc/wooble/wakka/jkl".to_string()).unwrap(),
            bt
        );
        assert!(path.extract("/wooble/wakka/jkl".to_string()).is_err());
        assert!(path.extract("/def/wooble/wakka/jkl".to_string()).is_err());

        assert_eq!(
            Path::new("/abc/:wooble/:wakka/jkl".to_string()).to_string(),
            "/abc/:wooble/:wakka/jkl".to_string()
        );

        assert_eq!(Path::default().to_string(), "/".to_string());
    }
}
