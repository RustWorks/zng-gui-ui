//! Initialize a new repository from a Zng template repository

use std::{
    fs, io, mem,
    path::{Path, PathBuf},
};

use clap::*;
use color_print::cstr;
use convert_case::{Case, Casing};

use crate::util;

#[derive(Args, Debug)]
pub struct NewArgs {
    /// Set template values by position
    ///
    /// The first value for all templates is the app name.
    ///
    /// EXAMPLE
    ///
    /// cargo zng new "My App!" | creates a "my-app" project.
    ///
    /// cargo zng new "my_app"  | creates a "my_app" project.
    #[arg(num_args(0..))]
    value: Vec<String>,

    /// Zng template
    ///
    /// Can be `.git` URL or an `owner/repo` for a GitHub repository.
    ///
    /// Can also be an absolute path or `./path` to a local template directory.
    #[arg(short, long, default_value = "zng-ui/zng-template")]
    template: String,

    /// Set a template value
    ///
    /// Templates have a `.zng-template` file that defines the possible options.
    #[arg(short, long, num_args(0..))]
    set: Vec<String>,

    /// Show all possible values that can be set on the template
    #[arg(short, long, action)]
    keys: bool,
}

pub fn run(args: NewArgs) {
    let template = parse_template(args.template);

    if args.keys {
        return print_keys(template);
    }

    let arg_keys = match parse_key_values(args.value, args.set) {
        Ok(arg_keys) => {
            if arg_keys.is_empty() || (!arg_keys[0].0.is_empty() && arg_keys.iter().all(|(k, _)| k != "app")) {
                fatal!("missing required key `app`")
            }
            arg_keys
        }
        Err(e) => fatal!("{e}"),
    };

    println!(cstr!("<bold>validate name and init<bold>"));
    let app = &arg_keys[0].1;
    let project_name = clean_value(app, true)
        .unwrap_or_else(|e| fatal!("{e}"))
        .replace(' ', "-")
        .to_lowercase();
    if let Err(e) = util::cmd("cargo new --quiet --bin", &[project_name.as_str()], &[]) {
        let _ = std::fs::remove_dir_all(&project_name);
        fatal!("{e}");
    }

    if let Err(e) = cleanup_cargo_new(&project_name) {
        fatal!("failed to cleanup `cargo new` template, {e}");
    }

    println!(cstr!("<bold>clone template<bold>"));
    let template_temp = PathBuf::from(format!("{project_name}.zng_template.tmp"));

    let fatal_cleanup = || {
        let _ = fs::remove_dir_all(&template_temp);
        let _ = fs::remove_dir_all(&project_name);
    };

    let template_keys = template.git_clone(&template_temp, false).unwrap_or_else(|e| {
        fatal_cleanup();
        fatal!("failed to clone template, {e}")
    });

    let cx = Fmt::new(template_keys, arg_keys).unwrap_or_else(|e| {
        fatal_cleanup();
        fatal!("{e}")
    });
    println!(cstr!("<bold>generate template<bold>"));
    if let Err(e) = apply_template(&cx, &template_temp, &project_name) {
        error!("{e}");
        fatal_cleanup();
        util::exit();
    }

    println!(cstr!("<bold>cargo fmt<bold>"));
    if let Err(e) = std::env::set_current_dir(project_name).and_then(|_| util::cmd("cargo fmt", &[], &[])) {
        fatal!("{e}")
    }
}

fn clean_value(value: &str, required: bool) -> io::Result<String> {
    let mut first_char = false;
    let clean_value: String = value
        .chars()
        .filter(|c| {
            if first_char {
                first_char = c.is_ascii_alphabetic();
                first_char
            } else {
                *c == ' ' || *c == '-' || *c == '_' || c.is_ascii_alphanumeric()
            }
        })
        .collect();
    let clean_value = clean_value.trim().to_owned();

    if required && clean_value.is_empty() {
        if clean_value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot derive clean value from `{value}`, must contain at least one ascii alphabetic char"),
            ));
        }
        if clean_value.len() > 62 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot derive clean value from `{value}`, must contain <= 62 ascii alphanumeric chars"),
            ));
        }
    }
    Ok(clean_value)
}

fn parse_key_values(value: Vec<String>, define: Vec<String>) -> io::Result<ArgsKeyMap> {
    let mut r = Vec::with_capacity(value.len() + define.len());

    for value in value {
        r.push((String::new(), value));
    }

    for key_value in define {
        if let Some((key, value)) = key_value.split_once('=') {
            if !is_key(key) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid key `{key}`")));
            }
            r.push((key.to_owned(), value.to_owned()));
        }
    }

    Ok(r)
}

fn print_keys(template: Template) {
    println!(cstr!("<bold>clone template to temp dir<bold>"));

    for i in 0..100 {
        let template_temp = std::env::temp_dir().join(format!("cargo-zng-template-keys-help-{i}"));
        if template_temp.exists() {
            continue;
        }

        match template.git_clone(&template_temp, true) {
            Ok(keys) => {
                println!("TEMPLATE KEYS\n");
                for kv in keys {
                    let value = match &kv.value {
                        Some(dft) => dft.as_str(),
                        None => cstr!("<bold><y>required</y></bold>"),
                    };
                    println!(cstr!("<bold>{}=</bold>{}"), kv.key, value);
                    if !kv.docs.is_empty() {
                        for line in kv.docs.lines() {
                            println!("   {line}");
                        }
                        println!();
                    }
                }
            }
            Err(e) => {
                error!("failed to clone template, {e}");
            }
        }
        let _ = fs::remove_dir_all(&template_temp);
        return;
    }
    fatal!("failed to clone template, no temp dir available");
}

fn parse_template(arg: String) -> Template {
    if arg.ends_with(".git") {
        return Template::Git(arg);
    }

    if arg.starts_with("./") {
        return Template::Local(PathBuf::from(arg));
    }

    if let Some((owner, repo)) = arg.split_once('/') {
        if !owner.is_empty() && !repo.is_empty() && !repo.contains('/') {
            return Template::Git(format!("https://github.com/{owner}/{repo}.git"));
        }
    }

    let path = PathBuf::from(arg);
    if path.is_absolute() {
        return Template::Local(path);
    }

    fatal!("--template must be a `.git` URL, `owner/repo`, `./local` or `/absolute/local`");
}

enum Template {
    Git(String),
    Local(PathBuf),
}
impl Template {
    /// Clone repository, if it is a template return the `.zng-template` file contents.
    fn git_clone(self, to: &Path, include_docs: bool) -> io::Result<KeyMap> {
        let from = match self {
            Template::Git(url) => url,
            Template::Local(path) => {
                let path = dunce::canonicalize(path)?;
                path.display().to_string()
            }
        };
        util::cmd("git clone --depth 1", &[from.as_str(), &to.display().to_string()], &[])?;

        match fs::read_to_string(PathBuf::from(to).join(".zng-template")) {
            Ok(s) => parse_keys(s, include_docs),
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "git repo is not a zng template, missing `.zng-template`",
                    ));
                }
                Err(e)
            }
        }
    }
}

fn cleanup_cargo_new(path: &str) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else if path.is_file() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn apply_template(cx: &Fmt, template_temp: &Path, package_name: &str) -> io::Result<()> {
    // remove template .git, .zng-template and Cargo.lock
    fs::remove_dir_all(template_temp.join(".git"))?;
    fs::remove_file(template_temp.join(".zng-template"))?;

    // remove .zng-template-ignore
    let template_temp = dunce::canonicalize(template_temp)?;
    match fs::read_to_string(template_temp.join(".zng-template-ignore")) {
        Ok(ignore) => {
            for glob in ignore.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with('#')) {
                for path in glob::glob(glob).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))? {
                    let path = path.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    let path = dunce::canonicalize(&path)?;
                    if path.starts_with(&template_temp) {
                        if path.is_dir() {
                            fs::remove_dir_all(path)?;
                        } else if path.is_file() {
                            fs::remove_file(path)?;
                        }
                    }
                }
            }
        }
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                return Err(e);
            }
        }
    }
    fs::remove_file(template_temp.join(".zng-template-ignore"))?;

    // rename/rewrite template and move it to new package dir
    apply(cx, &template_temp, &PathBuf::from(package_name))?;
    // remove (empty) template temp
    fs::remove_dir_all(template_temp)
}

fn apply(cx: &Fmt, from: &Path, to: &Path) -> io::Result<()> {
    for entry in fs::read_dir(from)? {
        let from = entry?.path();
        if from.is_dir() {
            let from = cx.rename(&from)?;
            let to = to.join(from.file_name().unwrap());
            println!("{}", to.display());
            fs::create_dir(&to)?;
            apply(cx, &from, &to)?;
        } else if from.is_file() {
            let from = cx.rename(&from)?;
            let to = to.join(from.file_name().unwrap());
            cx.rewrite(&from)?;
            println!("{}", to.display());
            fs::rename(from, to).unwrap();
        }
    }
    Ok(())
}

struct Fmt {
    replace: ReplaceMap,
}
impl Fmt {
    fn new(mut template_keys: KeyMap, arg_keys: ArgsKeyMap) -> io::Result<Self> {
        for (i, (key, value)) in arg_keys.into_iter().enumerate() {
            if key.is_empty() {
                if i >= template_keys.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "more positional values them template keys",
                    ));
                }
                template_keys[i].value = Some(value);
            } else if let Some(kv) = template_keys.iter_mut().find(|kv| kv.key == key) {
                kv.value = Some(value);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown key `{key}`, not declared by template"),
                ));
            }
        }
        Ok(Self {
            replace: make_replacements(&template_keys)?,
        })
    }

    fn rename(&self, template_path: &Path) -> io::Result<PathBuf> {
        let mut path = template_path.to_string_lossy().into_owned();
        for (key, value) in &self.replace {
            path = path.replace(key, value);
        }
        let path = PathBuf::from(path);
        if template_path != path {
            fs::rename(template_path, &path)?;
        }
        Ok(path)
    }

    fn rewrite(&self, template_path: &Path) -> io::Result<()> {
        match fs::read_to_string(template_path) {
            Ok(txt) => {
                let mut new_txt = txt.clone();
                for (key, value) in &self.replace {
                    new_txt = new_txt.replace(key, value);
                }
                if new_txt != txt {
                    fs::write(template_path, new_txt.as_bytes())?;
                }
                Ok(())
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::InvalidData {
                    // not utf-8 text file
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }
}

static PATTERNS: &[(&str, &str, Option<Case>)] = &[
    ("t-key-t", "kebab-case", Some(Case::Kebab)),
    ("T-KEY-T", "UPPER-KEBAB-CASE", Some(Case::UpperFlat)),
    ("t_key_t", "snake_case", Some(Case::Snake)),
    ("T_KEY_T", "UPPER_SNAKE_CASE", Some(Case::UpperSnake)),
    ("T-Key-T", "Train-Case", Some(Case::Train)),
    ("t.key.t", "lower case", Some(Case::Lower)),
    ("T.KEY.T", "UPPER CASE", Some(Case::Upper)),
    ("T.Key.T", "Title Case", Some(Case::Title)),
    ("ttKeyTt", "camelCase", Some(Case::Camel)),
    ("TtKeyTt", "PascalCase", Some(Case::Pascal)),
    ("{{key}}", "<unchanged>", None),
];

type KeyMap = Vec<TemplateKey>;
type ArgsKeyMap = Vec<(String, String)>;
type ReplaceMap = Vec<(String, String)>;

struct TemplateKey {
    docs: String,
    key: String,
    value: Option<String>,
    required: bool,
}

fn is_key(s: &str) -> bool {
    s.len() >= 3 && s.is_ascii() && s.chars().all(|c| c.is_ascii_alphabetic() && c.is_lowercase())
}

fn parse_keys(zng_template_v1: String, include_docs: bool) -> io::Result<KeyMap> {
    let mut r = vec![];

    let mut docs = String::new();

    for (i, line) in zng_template_v1.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            docs.clear();
            continue;
        }

        if line.starts_with('#') {
            if include_docs {
                let mut line = line.trim_start_matches('#');
                if line.starts_with(' ') {
                    line = &line[1..];
                }
                docs.push_str(line);
                docs.push('\n');
            }
            continue;
        }

        if r.is_empty() && line != "app=" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "broken template, first key must be `app=`",
            ));
        }

        let docs = mem::take(&mut docs);
        if let Some((key, val)) = line.split_once('=') {
            if is_key(key) {
                if val.is_empty() {
                    r.push(TemplateKey {
                        docs,
                        key: key.to_owned(),
                        value: None,
                        required: true,
                    });
                    continue;
                } else if val.starts_with('"') && val.ends_with('"') {
                    r.push(TemplateKey {
                        docs,
                        key: key.to_owned(),
                        value: Some(val[1..val.len() - 1].to_owned()),
                        required: false,
                    });
                    continue;
                }
            }
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("broken template, invalid syntax in `.zng-template:{}`", i + 1),
        ));
    }

    Ok(r)
}
fn make_replacements(keys: &KeyMap) -> io::Result<ReplaceMap> {
    let mut r = Vec::with_capacity(keys.len() * PATTERNS.len());
    for kv in keys {
        let value = match &kv.value {
            Some(v) => v,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("missing required key `{}`", kv.key),
                ))
            }
        };
        let clean_value = clean_value(value, kv.required)?;

        for (pattern, _, case) in PATTERNS {
            let prefix = &pattern[..2];
            let suffix = &pattern[pattern.len() - 2..];
            let (key, value) = if let Some(case) = case {
                (kv.key.to_case(*case), clean_value.to_case(*case))
            } else {
                (kv.key.to_owned(), value.to_owned())
            };
            let key = format!("{prefix}{key}{suffix}");
            r.push((key, value));
        }
    }
    Ok(r)
}