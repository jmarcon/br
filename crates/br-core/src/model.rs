use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub config_version: u32,
    #[serde(default)]
    pub general: General,
    #[serde(default, rename = "browsers")]
    pub browsers: Vec<BrowserTarget>,
    #[serde(default, rename = "filters")]
    pub filters: Vec<Filter>,
    #[serde(default, rename = "rules")]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DefaultAction {
    #[default]
    Ask,
    OpenWith(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct General {
    #[serde(default)]
    pub default_action: DefaultActionSerde,
    #[serde(default)]
    pub picker_timeout_ms: u64,
    #[serde(default = "default_picker_position")]
    pub picker_position: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_picker_background")]
    pub picker_background: String,
    #[serde(default = "default_picker_background_color")]
    pub picker_background_color: String,
    #[serde(default)]
    pub picker_background_image: Option<String>,
    #[serde(default = "default_picker_window_opacity")]
    pub picker_window_opacity: f32,
    #[serde(default = "default_picker_acrylic")]
    pub picker_acrylic: bool,
    #[serde(default = "default_picker_icon_size")]
    pub picker_icon_size: f32,
    #[serde(default = "default_picker_padding")]
    pub picker_padding: f32,
    #[serde(default = "default_picker_width")]
    pub picker_width: f32,
    #[serde(default = "default_picker_height")]
    pub picker_height: f32,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub start_on_login: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for General {
    fn default() -> Self {
        General {
            default_action: DefaultActionSerde::default(),
            picker_timeout_ms: 0,
            picker_position: default_picker_position(),
            theme: default_theme(),
            picker_background: default_picker_background(),
            picker_background_color: default_picker_background_color(),
            picker_background_image: None,
            picker_window_opacity: default_picker_window_opacity(),
            picker_acrylic: default_picker_acrylic(),
            picker_icon_size: default_picker_icon_size(),
            picker_padding: default_picker_padding(),
            picker_width: default_picker_width(),
            picker_height: default_picker_height(),
            language: default_language(),
            start_on_login: false,
            log_level: default_log_level(),
        }
    }
}

fn default_picker_position() -> String {
    "cursor".to_string()
}
fn default_theme() -> String {
    "system".to_string()
}
fn default_picker_background() -> String {
    "bubbles".to_string()
}
fn default_picker_background_color() -> String {
    "#1453aa".to_string()
}
fn default_picker_window_opacity() -> f32 {
    1.0
}
fn default_picker_acrylic() -> bool {
    false
}
fn default_picker_icon_size() -> f32 {
    72.0
}
fn default_picker_padding() -> f32 {
    20.0
}
fn default_picker_width() -> f32 {
    720.0
}
fn default_picker_height() -> f32 {
    460.0
}
fn default_language() -> String {
    "en".to_string()
}
fn default_log_level() -> String {
    "warn".to_string()
}

/// `default_action` is stored as a plain string in TOML: "ask" or "open_with:<id>".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DefaultActionSerde(pub DefaultAction);

impl Default for DefaultActionSerde {
    fn default() -> Self {
        DefaultActionSerde(DefaultAction::Ask)
    }
}

impl TryFrom<String> for DefaultActionSerde {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "ask" {
            Ok(DefaultActionSerde(DefaultAction::Ask))
        } else if let Some(id) = value.strip_prefix("open_with:") {
            Ok(DefaultActionSerde(DefaultAction::OpenWith(id.to_string())))
        } else {
            Err(format!(
                "invalid default_action '{value}': expected 'ask' or 'open_with:<id>'"
            ))
        }
    }
}

impl From<DefaultActionSerde> for String {
    fn from(value: DefaultActionSerde) -> Self {
        match value.0 {
            DefaultAction::Ask => "ask".to_string(),
            DefaultAction::OpenWith(id) => format!("open_with:{id}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTarget {
    pub id: String,
    pub name: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default = "default_executable")]
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub profile_dir: Option<String>,
    #[serde(default)]
    pub profile_name: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub hidden: bool,
}

fn default_kind() -> String {
    "generic".to_string()
}
fn default_executable() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Filter {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub strip_query_params: Vec<String>,
    #[serde(default)]
    pub upgrade_http_to_https: bool,
    #[serde(default)]
    pub exceptions: Vec<String>,
    #[serde(default)]
    pub rewrite_pattern: Option<String>,
    #[serde(default)]
    pub rewrite_replacement: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Rule {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i64,
    #[serde(rename = "match")]
    pub match_: MatchCondition,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchCondition {
    #[serde(default)]
    pub url_pattern: Vec<String>,
    #[serde(default)]
    pub host: Vec<String>,
    #[serde(default)]
    pub path: Vec<String>,
    #[serde(default)]
    pub query_param: Vec<QueryParamMatch>,
    #[serde(default)]
    pub source_app: Vec<String>,
    #[serde(default)]
    pub modifier_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum QueryParamMatch {
    Name(String),
    Condition {
        name: String,
        #[serde(default)]
        value: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Action {
    #[serde(default)]
    pub open_with: Option<String>,
    #[serde(default)]
    pub open_with_all: Vec<String>,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub ask: bool,
    #[serde(default)]
    pub block: bool,
}

/// The result of evaluating filters + rules for a given URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingDecision {
    OpenWith { target: String, private: bool },
    OpenWithAll { targets: Vec<String>, private: bool },
    AskUser,
    Block,
}

/// Context surrounding a routing request (e.g. the app that originated the click).
#[derive(Debug, Clone, Default)]
pub struct RoutingContext {
    pub source_app: Option<String>,
    pub modifier_keys: Vec<String>,
}

/// Explains which rule (if any) produced a [`RoutingDecision`], for `br rules test`.
#[derive(Debug, Clone)]
pub struct RoutingExplanation {
    pub normalized_url: String,
    pub matched_rule: Option<String>,
    pub decision: RoutingDecision,
}
