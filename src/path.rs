use crate::{handler::Params, Error};

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum RoutePart {
    PathComponent(&'static str),
    Param(&'static str),
    Leader,
}

#[derive(Debug, Clone, PartialOrd)]
pub struct Path(Vec<RoutePart>);

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for Path {}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl Path {
    pub(crate) fn new(path: &'static str) -> Self {
        let mut parts = Self::default();

        for arg in path.split("/") {
            if arg.starts_with(":") {
                // is param
                parts.push(RoutePart::Param(arg.trim_start_matches(":")));
            } else if arg == "" { // skip empties
            } else {
                // is not param
                parts.push(RoutePart::PathComponent(arg));
            }
        }

        parts
    }

    pub(crate) fn push(&mut self, arg: RoutePart) -> Self {
        self.0.push(arg);
        self.clone()
    }

    #[allow(dead_code)]
    pub(crate) fn params(&self) -> Vec<&str> {
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
    pub(crate) fn extract(&self, provided: &'static str) -> Result<Params, Error> {
        let parts: Vec<&str> = provided.split("/").collect();
        let mut params = Params::default();

        if parts.len() != self.0.len() {
            return Err(Error::new("invalid parameters"));
        }

        let mut i = 0;

        for part in self.0.clone() {
            match part {
                RoutePart::Param(p) => params.insert(p, parts[i]),
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

    pub(crate) fn matches(&self, path: &'static str) -> bool {
        let parts: Vec<&str> = path.split("/").collect();

        if parts.len() != self.0.len() {
            return false;
        }

        let mut i = 0;
        for arg in parts {
            if arg != "" {
                let res = match self.0[i] {
                    RoutePart::PathComponent(pc) => pc == arg,
                    RoutePart::Param(_param) => {
                        // FIXME advanced parameter shit here later
                        true
                    }
                    RoutePart::Leader => true,
                };

                if !res {
                    return res;
                }
            }

            i += 1;
        }

        true
    }
}

impl Default for Path {
    fn default() -> Self {
        Self(vec![RoutePart::Leader])
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

        let path = Path::new("/abc/def/ghi");
        assert!(path.matches("/abc/def/ghi"));
        assert!(!path.matches("//abc/def/ghi"));
        assert!(!path.matches("/def/ghi"));
        assert!(path.params().is_empty());

        let path = Path::new("/abc/:def/:ghi/jkl");
        assert!(!path.matches("/abc/def/ghi"));
        assert!(path.matches("/abc/def/ghi/jkl"));
        assert!(path.matches("/abc/ghi/def/jkl"));
        assert!(path.matches("/abc/wooble/wakka/jkl"));
        assert!(!path.matches("/nope/ghi/def/jkl"));
        assert!(!path.matches("/abc/ghi/def/nope"));

        let mut bt = BTreeMap::new();
        bt.insert("def", "wooble");
        bt.insert("ghi", "wakka");

        assert_eq!(path.extract("/abc/wooble/wakka/jkl").unwrap(), bt);
        assert!(path.extract("/wooble/wakka/jkl").is_err());
        assert!(path.extract("/def/wooble/wakka/jkl").is_err());

        assert_eq!(
            Path::new("/abc/:wooble/:wakka/jkl").to_string(),
            "/abc/:wooble/:wakka/jkl".to_string()
        );

        assert_eq!(Path::default().to_string(), "/".to_string());
    }
}
