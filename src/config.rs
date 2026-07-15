use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub panels: Vec<String>,
    pub pomodoro: PomodoroCfg,
    pub theme: ThemeCfg,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct PomodoroCfg {
    pub focus_min: u64,
    pub break_min: u64,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct ThemeCfg {
    // optional hex overrides, e.g. accent = "#cba6f7"
    pub accent: Option<String>,
    pub green: Option<String>,
    pub red: Option<String>,
    pub yellow: Option<String>,
    pub blue: Option<String>,
    pub peach: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            panels: ["habits", "calendar", "ideas", "todos", "pomodoro", "stats"]
                .map(String::from)
                .to_vec(),
            pomodoro: PomodoroCfg::default(),
            theme: ThemeCfg::default(),
        }
    }
}

impl Default for PomodoroCfg {
    fn default() -> Self {
        Self {
            focus_min: 25,
            break_min: 5,
        }
    }
}

/// Sanitize a loaded config: ensure panels is not empty.
/// Returns (sanitized config, optional warning).
fn sanitize(mut c: Config) -> (Config, Option<String>) {
    if c.panels.is_empty() {
        c.panels = Config::default().panels;
        (
            c,
            Some("config.toml: panels empty, using default panel order".into()),
        )
    } else {
        (c, None)
    }
}

/// Load config from <config_dir>/config.toml; fall back to defaults on any failure.
/// Returns (config, warning) — warning is shown in the status line.
pub fn load() -> (Config, Option<String>) {
    let Some(dirs) = directories::ProjectDirs::from("", "", "productivo") else {
        return (Config::default(), None);
    };
    let path = dirs.config_dir().join("config.toml");
    match std::fs::read_to_string(&path) {
        Ok(s) => match toml::from_str(&s) {
            Ok(c) => sanitize(c),
            Err(e) => (
                Config::default(),
                Some(format!("config.toml invalid, using defaults: {e}")),
            ),
        },
        Err(_) => (Config::default(), None), // no file = defaults, not an error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_six_panels() {
        let c = Config::default();
        assert_eq!(c.panels.len(), 6);
        assert_eq!(c.pomodoro.focus_min, 25);
    }

    #[test]
    fn partial_toml_fills_defaults() {
        let c: Config =
            toml::from_str("panels = [\"todos\", \"stats\"]\n[pomodoro]\nfocus_min = 50\n")
                .unwrap();
        assert_eq!(c.panels, vec!["todos", "stats"]);
        assert_eq!(c.pomodoro.focus_min, 50);
        assert_eq!(c.pomodoro.break_min, 5);
    }

    #[test]
    fn sanitize_empty_panels() {
        let c: Config = toml::from_str("panels = []").unwrap();
        assert_eq!(c.panels.len(), 0);
        let (sanitized, warning) = sanitize(c);
        assert_eq!(sanitized.panels.len(), 6);
        assert_eq!(sanitized.panels, Config::default().panels);
        assert_eq!(
            warning,
            Some("config.toml: panels empty, using default panel order".into())
        );
    }
}
