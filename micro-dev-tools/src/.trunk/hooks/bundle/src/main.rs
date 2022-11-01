use fs_extra::dir::CopyOptions;
use regex::{Match, Regex};
use std::{
    borrow::Cow,
    collections::HashMap,
    env, fmt,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use yaml_rust::*;

const ANY_PATH_NAME: &str = "_";

const LOG_COLOR_ERROR: &str = "91"; //bright red
const LOG_COLOR_INFO: &str = "94"; //bright blue

#[macro_export]
macro_rules! log {
    (error; $($arg:tt)*) => {
        eprint!("\x1b[90m[bundle] \x1b[{}mERROR\x1b[0m ", LOG_COLOR_ERROR);
        eprintln!($($arg)*);
    };
    (info; $($arg:tt)*) => {
        eprint!("\x1b[90m[bundle] \x1b[{}mINFO\x1b[0m ", LOG_COLOR_INFO);
        eprintln!($($arg)*);
    };
}

fn main() {
    let bundle_config = read_bundle_config().unwrap();
    log!(info; "bundle targets in {}", &bundle_config.targets_dir);
    let targets = targets(&bundle_config);
    for target in targets {
        log!(info; "target : {}", &target.as_os_str().to_string_lossy());
        match parse_config(&bundle_config, &target) {
            Ok(c) => {
                build(&bundle_config, &c);
            }
            Err(e) => {
                log!(error; "{}", e);
            }
        }
    }
}

fn read_bundle_config() -> Result<BundleConfig, &'static str> {
    let profile = env::var("TRUNK_PROFILE").unwrap();
    let stage = env::var("TRUNK_STAGING_DIR").unwrap().stripped(r"\\?\").to_string();
    let html_file = env::var("TRUNK_HTML_FILE").unwrap().stripped(r"\\?\").to_string();
    let source_dir = env::var("TRUNK_SOURCE_DIR").unwrap().stripped(r"\\?\").to_string();
    let dist_dir = env::var("TRUNK_DIST_DIR").unwrap().stripped(r"\\?\").to_string();
    let public_url = env::var("TRUNK_PUBLIC_URL").unwrap();

    let args: HashMap<_, _> = env::args().skip(1).step_by(2).zip(env::args().skip(2).step_by(2)).collect();
    let targets_dir = args.get("-t").or(args.get("--target")).unwrap().clone();
    let build_config_path = args.get("--config-path").map_or(".build/build.yml", String::as_str).to_string();
    let default_deploy_dir = args.get("--deploy-dir").map_or(stage.as_str(), String::as_str).to_string();

    let config = BundleConfig {
        profile,
        stage,
        html_file,
        source_dir,
        dist_dir,
        public_url,
        build_config_path,
        targets_dir,
        default_deploy_dir,
    };
    return Ok(config);

    trait StrippedExtension {
        fn stripped<'a>(&'a self, prefix: &Self) -> &'a Self;
    }

    impl StrippedExtension for str {
        fn stripped<'a>(&'a self, prefix: &str) -> &'a Self {
            self.strip_prefix(prefix).unwrap_or(self)
        }
    }
}

fn build(bundle_config: &BundleConfig, config: &BuildConfig) {
    log!(info; "build : {}", &config.target.as_os_str().to_string_lossy());

    let succeeded = config
        .paths
        .iter()
        .filter(|(k, _)| is_match(k, &bundle_config.profile))
        .fold(true, |prev, (n, p)| prev && execute_path(n, p));

    if !succeeded || !copy_artifacts(config) {
        log!(error; "failed to build : {}", &config.target.as_os_str().to_string_lossy());
    }

    fn is_match(pattern: &str, profile: &str) -> bool {
        match pattern {
            ANY_PATH_NAME => true,
            _ => profile == pattern,
        }
    }

    fn execute_path(name: &str, path: &Vec<Step>) -> bool {
        log!(info; "execute build path : {}", name);
        let result = path.iter().fold(true, |prev_is_succeeded, step| {
            if !prev_is_succeeded {
                return false;
            }
            execute_step(step)
        });

        if !result {
            log!(error; "faild to execute build path : {}", name);
        }

        result
    }

    fn execute_step(step: &Step) -> bool {
        let mut cmd = Command::new(&step.exec);
        cmd.current_dir(&step.working_dir).args(&step.args);
        log!(info; "run : {}", &step.label);
        let result = cmd.output();
        match result {
            Err(err) => {
                log!(error; "faild to run build step {} : {}", &step.label, err);
                false
            }
            Ok(output) => {
                println!("{}", String::from_utf8_lossy(&output.stdout).as_ref());
                if !output.stderr.is_empty() {
                    println!("{}", String::from_utf8_lossy(&output.stderr).as_ref());
                };

                let success = output.status.success();
                if !success {
                    log!(error; "faild to run build step. {}", &output.status);
                }
                success
            }
        }
    }
}

fn copy_artifacts(config: &BuildConfig) -> bool {
    log!(info; "copy artifacts from {} to {}", config.dist.as_os_str().to_string_lossy(), config.deploy.as_os_str().to_string_lossy());
    let entries = if let Ok(entries) = fs::read_dir(config.dist.as_path()) {
        entries
    } else {
        log!(error; "{} was not exist", config.dist.as_os_str().to_string_lossy());
        return false;
    };
    let mut options = CopyOptions::new();
    options.overwrite = true;
    let items = entries.filter_map(|e| e.ok().map(|e| e.path())).collect::<Vec<_>>();
    let result = fs::create_dir_all(config.deploy.as_path());
    if let Err(err) = result.as_ref() {
        log!(error; "cannot create output dir {}", err);
        return false;
    }

    let result = fs_extra::copy_items(&items, &config.deploy, &options);

    if let Err(err) = result.as_ref() {
        log!(error; "cannot copy {}", err);
        return false;
    }
    return true;
}

fn split_to_args<'a>(args: &'a str) -> Vec<&'a str> {
    let mut stack = Vec::new();
    enum State {
        Initial,
        Plain { offset: usize },
        Quoted { offset: usize, quote: char },
        QuoteEnd { offset: usize },
        QuoteEscape { offset: usize, quote: char },
    }
    let mut state = State::Initial;
    for (i, c) in args.char_indices() {
        state = match state {
            State::Initial => {
                if c.is_whitespace() {
                    State::Initial
                } else if matches!(c, '"' | '\'') {
                    State::Quoted { offset: i, quote: c }
                } else {
                    State::Plain { offset: i }
                }
            }

            State::Plain { offset } => {
                if c.is_whitespace() {
                    stack.push(&args[offset..i]);
                    State::Initial
                } else if matches!(c, '"' | '\'') {
                    stack.push(&args[offset..i]);
                    State::Quoted { offset, quote: c }
                } else {
                    State::Plain { offset }
                }
            }

            State::Quoted { offset, quote } => {
                if c == '\\' {
                    State::QuoteEscape { offset, quote }
                } else if c == quote {
                    State::QuoteEnd { offset }
                } else {
                    State::Quoted { offset, quote }
                }
            }

            State::QuoteEnd { offset } => {
                stack.push(&args[offset..i]);
                if c.is_whitespace() {
                    State::Initial
                } else if matches!(c, '"' | '\'') {
                    State::Quoted { offset: i, quote: c }
                } else {
                    State::Plain { offset: i }
                }
            }

            State::QuoteEscape { offset, quote } => State::Quoted { offset, quote },
        }
    }

    match state {
        State::Plain { offset } => {
            stack.push(&args[offset..]);
        }
        State::Quoted { offset, .. } => {
            stack.push(&args[offset..]);
        }
        State::QuoteEnd { offset } => {
            stack.push(&args[offset..]);
        }
        State::QuoteEscape { offset, .. } => {
            stack.push(&args[offset..]);
        }
        _ => {}
    }

    return stack;
}

fn targets(bundle_config: &BundleConfig) -> Vec<PathBuf> {
    let dir = if let Ok(dir) = fs::read_dir(&bundle_config.targets_dir) {
        dir
    } else {
        return vec![];
    };

    return dir
        .filter_map(|e| {
            let e = e.ok()?;
            let path = e.path();
            let config = path.join(&*bundle_config.build_config_path);
            if config.try_exists().unwrap_or(false) {
                return Some(vec![path]);
            } else {
                return Some(targets(bundle_config));
            }
        })
        .flatten()
        .collect::<Vec<_>>();
}

fn parse_config(bundle_config: &BundleConfig, target: &impl AsRef<Path>) -> Result<BuildConfig, BuildConfigError> {
    let config_source = fs::read_to_string(target.as_ref().join(&bundle_config.build_config_path)).map_err(BuildConfigError::from)?;
    let mut config_docs = YamlLoader::load_from_str(&config_source).map_err(BuildConfigError::from)?;
    if config_docs.len() <= 0 {
        return Err(BuildConfigError::ReadBuildConfig {
            msg: String::from("config is empty"),
        });
    }
    let mut config_doc = config_docs.remove(0);

    let mut stage = BuildConfigStage {
        target: target.as_ref().to_path_buf(),
        dist: None,
        deploy: None,
        paths: HashMap::new(),
    };

    let mut context = ParseContext {
        pattern: Regex::new(r"\$\{\{(.*?)}}").unwrap(),
        stage: &mut stage,
        bundle_config,
    };

    context.read(&mut config_doc)?;

    Ok(stage.complete())
}

fn stringify(doc: &Yaml) -> Option<String> {
    let mut buf = String::new();
    write_to_string(doc, &mut buf).ok().map(move |_| buf)
}
fn write_to_string(doc: &Yaml, buffer: &mut String) -> Result<(), ()> {
    use std::fmt::Write;
    match doc {
        Yaml::Real(s) => {
            write!(buffer, "{}", s).map_err(|_| ())?;
            Ok(())
        }
        Yaml::Integer(i) => {
            write!(buffer, "{}", i).map_err(|_| ())?;
            Ok(())
        }
        Yaml::String(s) => {
            write!(buffer, "{}", s).map_err(|_| ())?;
            Ok(())
        }
        Yaml::Boolean(f) => {
            write!(buffer, "{}", f).map_err(|_| ())?;
            Ok(())
        }
        _ => Err(()),
    }
}
fn as_vec_mut(doc: &mut Yaml) -> Option<&mut Vec<Yaml>> {
    if let Yaml::Array(arr) = doc {
        Some(arr)
    } else {
        None
    }
}
fn as_hash_mut(doc: &mut Yaml) -> Option<&mut yaml_rust::yaml::Hash> {
    if let Yaml::Hash(hash) = doc {
        Some(hash)
    } else {
        None
    }
}

struct ParseContext<'a> {
    pattern: Regex,
    bundle_config: &'a BundleConfig,
    stage: &'a mut BuildConfigStage,
}

impl<'a> ParseContext<'a> {
    fn access(&self, config_doc: *mut Yaml, path: &str) -> Result<*mut Yaml, BuildConfigError> {
        let entry = unsafe { raw_access(config_doc, path)? };

        if let Yaml::String(s) = unsafe { &mut *entry } {
            let matches = self.pattern.find_iter(s);
            let scanned = matches.scan(Ok(0), |last_match, m| {
                if last_match.is_err() {
                    return None;
                }
                let subpath = Self::as_path_part(&m);
                let result = self.resolve(config_doc, subpath);
                if result.is_err() {
                    *last_match = Err(());
                } else {
                    *last_match = Ok(m.end());
                }

                Some(result.map(|e| (m.start(), m.end(), subpath, e)))
            });
            let mut collected = scanned.collect::<Result<Vec<_>, _>>()?;
            if collected.len() == 1 && collected[0].0 == 0 {
                unsafe {
                    *entry = collected.remove(0).3.into_yaml();
                }
            }

            let mut buffer = String::with_capacity(s.len());
            let mut last_match = 0;
            for (start, end, reference, result) in collected {
                buffer.push_str(&s[last_match..start]);
                result
                    .write_to_string(&mut buffer)
                    .map_err(|_| BuildConfigError::referenced_non_stringifiable(reference))?;
                last_match = end;
            }
            buffer.push_str(&s[last_match..]);

            *s = buffer;
        }
        return Ok(entry);

        unsafe fn raw_access<'a>(mut doc: *mut Yaml, path: &str) -> Result<*mut Yaml, BuildConfigError> {
            for key in path.split('.') {
                let next = match &mut *doc {
                    Yaml::Array(arr) => key.parse::<usize>().ok().and_then(|i| arr.get_mut(i)),
                    Yaml::Hash(hash) => hash.get_mut(&Yaml::String(key.to_owned())),
                    _ => None,
                };

                if next.is_none() {
                    return Err(BuildConfigError::reference_not_found(path));
                }

                doc = next.unwrap();
            }
            return Ok(doc);
        }
    }
    fn resolve(&self, config_doc: *mut Yaml, reference: &str) -> Result<ResolveResult, BuildConfigError> {
        const ENV_PREFIX: &str = "env:";
        const CONF_PREFIX: &str = "config:";
        const SELF_PREFIX: &str = "self:";
        if let Some(name) = reference.strip_prefix(ENV_PREFIX) {
            if let Ok(var) = env::var(name) {
                return Ok(ResolveResult::String(var));
            } else {
                return Err(BuildConfigError::EnvironmentValueNotFound { name: name.to_owned() });
            }
        } else if let Some(name) = reference.strip_prefix(CONF_PREFIX) {
            let bundle_config = self.bundle_config;
            let value = match name {
                "profile" => bundle_config.profile.clone(),
                "stage" => bundle_config.stage.clone(),
                "public_url" => bundle_config.public_url.clone(),
                "html_file" => bundle_config.html_file.clone(),
                "source_dir" => bundle_config.source_dir.clone(),
                "dist_dir" => bundle_config.dist_dir.clone(),
                "build_config_path" => bundle_config.build_config_path.clone(),
                _ => {
                    return Err(BuildConfigError::ConfigPropertyNotFound { name: name.to_owned() });
                }
            };
            return Ok(ResolveResult::String(value));
        } else if let Some(name) = reference.strip_prefix(SELF_PREFIX) {
            let value = match name {
                "identity" => self.stage.target.as_os_str().to_string_lossy().as_ref().replace('\\', "/"),
                _ => return Err(BuildConfigError::SelfPropertyNotFound { name: name.to_owned() }),
            };
            return Ok(ResolveResult::String(value));
        } else {
            return self.access(config_doc, reference).map(|doc| ResolveResult::Yaml(doc));
        }
    }
    fn read(&mut self, config_doc: &'a mut Yaml) -> Result<(), BuildConfigError> {
        unsafe {
            let dist = self
                .access(config_doc, "dist")
                .and_then(|e| stringify(&*e).ok_or_else(|| BuildConfigError::invalid_prop_value("dist")))?;
            self.stage.dist = Some(self.stage.target.join(dist));

            let deploy = match self.access(config_doc, "deploy") {
                Ok(e) => stringify(&*e)
                    .map(|s| Path::new(&s).to_path_buf())
                    .ok_or_else(|| BuildConfigError::invalid_prop_value("deploy"))?,
                Err(_) => Path::new(&self.bundle_config.default_deploy_dir).join(&self.stage.target),
            };

            self.stage.deploy = Some(deploy);

            let paths = self
                .access(config_doc, "build")
                .and_then(|e| as_hash_mut(&mut *e).ok_or_else(|| BuildConfigError::invalid_prop_value("build")))?;

            self.stage.paths = self.read_paths(paths.iter_mut())?;
        }

        Ok(())
    }

    fn read_paths<'b>(&self, source: impl Iterator<Item = (&'b Yaml, &'b mut Yaml)>) -> Result<HashMap<String, Vec<Step>>, BuildConfigError> {
        source
            .map(|(k, v)| {
                let k = k.as_str().ok_or_else(|| BuildConfigError::invalid_prop_value("build"))?;
                let v = as_vec_mut(v).ok_or_else(|| BuildConfigError::invalid_prop_value(format!("build.{}", k)))?;
                let steps = self.read_steps(v)?;
                Ok((k.to_string(), steps))
            })
            .collect()
    }

    fn read_steps(&self, steps: &mut Vec<Yaml>) -> Result<Vec<Step>, BuildConfigError> {
        unsafe {
            let steps = steps.iter_mut().map(|step_doc| {
                let run = self
                    .access(step_doc, "run")
                    .map_err(|e| match e {
                        BuildConfigError::ReferenceNotFound { reference } if reference == "run" => BuildConfigError::req_prop_not_exists("run"),
                        err @ _ => err,
                    })
                    .and_then(|c| (&*c).as_str().ok_or_else(|| BuildConfigError::invalid_prop_value("run")))?;
                let splitted = split_to_args(run);
                let exec = splitted[0].to_string();
                let args: Vec<_> = splitted.iter().skip(1).map(|s| s.to_string()).collect();
                let label = if let Ok(l) = self.access(step_doc, "label") {
                    (&*l)
                        .as_str()
                        .map(|s| s.to_string())
                        .ok_or_else(|| BuildConfigError::invalid_prop_value("label"))?
                } else {
                    args.iter().fold(exec.clone(), |mut buf, arg| {
                        buf.push(' ');
                        buf.push_str(arg);
                        buf
                    })
                };

                let working_dir = if let Ok(d) = self.access(step_doc, "working_dir") {
                    (&*d)
                        .as_str()
                        .map(|s| self.stage.target.join(s))
                        .ok_or_else(|| BuildConfigError::invalid_prop_value("working_dir"))?
                } else {
                    self.stage.target.clone()
                };

                let step = Step {
                    exec: exec.to_string(),
                    working_dir,
                    args,
                    label,
                };
                return Ok(step);
            });

            let mut vec = Vec::new();
            for step in steps {
                match step {
                    Ok(step) => vec.push(step),
                    Err(msg) => return Err(msg),
                }
            }

            return Ok(vec);
        }
    }

    fn as_path_part<'b>(m: &Match<'b>) -> &'b str {
        let s = m.as_str();
        &s[3..(s.len() - 2)]
    }
}

struct BuildConfigStage {
    target: PathBuf,
    dist: Option<PathBuf>,
    deploy: Option<PathBuf>,
    paths: HashMap<String, Vec<Step>>,
}
impl BuildConfigStage {
    fn complete(self) -> BuildConfig {
        BuildConfig {
            target: self.target,
            dist: self.dist.unwrap(),
            deploy: self.deploy.unwrap(),
            paths: self.paths,
        }
    }
}

enum ResolveResult {
    Yaml(*mut Yaml),
    String(String),
}

impl ResolveResult {
    fn into_yaml(self) -> Yaml {
        unsafe {
            match self {
                ResolveResult::Yaml(doc) => (&*doc).clone(),
                ResolveResult::String(s) => Yaml::String(s),
            }
        }
    }

    fn write_to_string(&self, buf: &mut String) -> Result<(), ()> {
        unsafe {
            match self {
                ResolveResult::String(s) => Ok(buf.push_str(s.as_str())),
                ResolveResult::Yaml(doc) => write_to_string(&**doc, buf),
            }
        }
    }
}

#[derive(Debug)]
struct Step {
    label: String,
    exec: String,
    args: Vec<String>,
    working_dir: PathBuf,
}

#[derive(Debug)]
struct BundleConfig {
    profile: String,
    stage: String,
    public_url: String,
    html_file: String,
    source_dir: String,
    dist_dir: String,
    build_config_path: String,
    targets_dir: String,
    default_deploy_dir: String,
}

#[derive(Debug)]
struct BuildConfig {
    target: PathBuf,
    dist: PathBuf,
    deploy: PathBuf,
    paths: HashMap<String, Vec<Step>>,
}

#[derive(Debug, Clone)]
enum BuildConfigError {
    ReadBuildConfig { msg: String },
    ConfigSyntax { err: ScanError },
    RequiredPropertyNotExists { path: String },
    ReferenceNotFound { reference: String },
    ReferencedNonStringifiable { reference: String },
    EnvironmentValueNotFound { name: String },
    ConfigPropertyNotFound { name: String },
    SelfPropertyNotFound { name: String },
    InvalidPropertyValue { reference: String },
}

impl BuildConfigError {
    fn reference_not_found<'a, T>(reference: T) -> Self
    where
        Cow<'a, str>: From<T>,
    {
        return Self::ReferenceNotFound {
            reference: Cow::from(reference).into_owned(),
        };
    }

    fn referenced_non_stringifiable<'a, T>(reference: T) -> Self
    where
        Cow<'a, str>: From<T>,
    {
        return Self::ReferencedNonStringifiable {
            reference: Cow::from(reference).into_owned(),
        };
    }

    fn invalid_prop_value<'a, T>(reference: T) -> Self
    where
        Cow<'a, str>: From<T>,
    {
        return Self::InvalidPropertyValue {
            reference: Cow::from(reference).into_owned(),
        };
    }

    fn req_prop_not_exists<'a, T>(path: T) -> Self
    where
        Cow<'a, str>: From<T>,
    {
        return Self::RequiredPropertyNotExists {
            path: Cow::from(path).into_owned(),
        };
    }
}

impl std::convert::From<std::io::Error> for BuildConfigError {
    fn from(e: std::io::Error) -> Self {
        BuildConfigError::ReadBuildConfig { msg: format!("{}", e) }
    }
}

impl std::convert::From<yaml_rust::scanner::ScanError> for BuildConfigError {
    fn from(err: yaml_rust::scanner::ScanError) -> Self {
        BuildConfigError::ConfigSyntax { err }
    }
}

impl Display for BuildConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use BuildConfigError::*;
        match self {
            RequiredPropertyNotExists { path } => write!(f, "required property of {} does not exists", path),
            ReadBuildConfig { msg } => write!(f, "{}", msg),
            ConfigSyntax { err } => write!(f, "config contains invalid syntax : {}", err),
            ReferenceNotFound { reference } => write!(f, "reference {} was not found", reference),
            ReferencedNonStringifiable { reference } => write!(f, "referenced {} but it is not stringifiable", reference),
            EnvironmentValueNotFound { name } => write!(f, "environment value of {} was not found", name),
            ConfigPropertyNotFound { name } => write!(f, "config property of {} was not found", name),
            SelfPropertyNotFound { name } => write!(f, "self property of {} was not found", name),
            InvalidPropertyValue { reference } => write!(f, "property {} has invalid value", reference),
        }
    }
}
