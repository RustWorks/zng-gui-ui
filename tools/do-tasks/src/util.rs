use std::env;
use std::format_args as f;
use std::io::Write;
use std::process::{self, Command, Stdio};

// shell script that builds and runs do-tasks.
pub static DO: &str = env!("DO_NAME");

// Run a command, args are chained, empty ("") arg strings are filtered, command streams are inherited.
pub fn cmd(cmd: &str, default_args: &[&str], user_args: &[&str]) {
    cmd_impl(cmd, default_args, user_args, false)
}
// Like [`cmd`] but exists the task runner if the command fails.
//fn cmd_req(cmd: &str, default_args: &[&str], user_args: &[&str]) {
//    cmd_impl(cmd, default_args, user_args, true)
//}
fn cmd_impl(cmd: &str, default_args: &[&str], user_args: &[&str], required: bool) {
    let info = TaskInfo::get();
    let args: Vec<_> = default_args.iter().chain(user_args.iter()).filter(|a| !a.is_empty()).collect();

    let mut cmd = Command::new(cmd);
    cmd.args(&args[..]);

    if info.dump {
        if let Some(stdout) = info.stdout_dump() {
            cmd.stdout(Stdio::from(stdout));
        }
        if let Some(stderr) = info.stderr_dump() {
            cmd.stdout(Stdio::from(stderr));
        }
    }

    let status = cmd.status();
    match status {
        Ok(status) => {
            if !status.success() {
                let msg = format!("task {:?} failed with {}", info.name, status);
                if required {
                    fatal(msg);
                } else {
                    error(msg);
                }
            }
        }
        Err(e) => {
            let msg = format!("task {:?} failed to run, {}", info.name, e);
            if required {
                fatal(msg)
            } else {
                error(msg);
            }
        }
    }
}

// Like [`cmd`] but runs after a small delay and does not block.
// Use this for commands that need write access to the self executable.
pub fn cmd_external(cmd: &str, default_args: &[&str], user_args: &[&str]) {
    let args: Vec<_> = default_args.iter().chain(user_args.iter()).filter(|a| !a.is_empty()).collect();

    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(&["/C", "ping", "localhost", "-n", "3", ">", "nul", "&"])
            .arg(cmd)
            .args(&args)
            .spawn()
            .ok();
    }
    #[cfg(not(windows))]
    {
        todo!("cmd_external only implemented in windows")
    }
}

// Removes all of the flags in `any` from `args`. Returns if found any.
pub fn take_arg(args: &mut Vec<&str>, any: &[&str]) -> bool {
    let mut i = 0;
    let mut found = false;
    while i < args.len() {
        if any.iter().any(|&a| args[i] == a) {
            found = true;
            args.remove(i);
            continue;
        }
        i += 1;
    }
    found
}

// Removes all of the `option` values, fails with "-o <value_name>" if a value is missing.
pub fn take_option<'a>(args: &mut Vec<&'a str>, option: &[&str], value_name: &str) -> Option<Vec<&'a str>> {
    let mut i = 0;
    let mut values = vec![];
    while i < args.len() {
        if option.iter().any(|&o| args[i] == o) {
            let next_i = i + 1;
            if next_i == args.len() || args[next_i].starts_with('-') {
                fatal(f!("expected value for option {} {}", args[i], value_name));
            }

            args.remove(i); // remove option
            values.push(args.remove(i)) // take value.
        }
        i += 1;
    }

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

// Parses the initial input. Returns ("task-name", ["task", "args"]).
pub fn args() -> (&'static str, Vec<&'static str>) {
    #[cfg(windows)]
    unsafe {
        ANSI_ENABLED = ansi_term::enable_ansi_support().is_ok();
    }

    let mut args: Vec<_> = env::args().skip(1).collect();
    if args.is_empty() {
        return ("", vec![]);
    }
    let task = Box::leak(args.remove(0).into_boxed_str());
    let mut args = args.into_iter().map(|a| Box::leak(a.into_boxed_str()) as &'static str).collect();

    // set task name and flags
    let info = TaskInfo::get();
    info.name = task;
    info.dump = take_arg(&mut args, &["--dump"]);

    // prints header
    println(f!("{}Running{}: {}{} {:?} {:?}", c_green(), c_wb(), DO, c_w(), task, args));

    (task, args)
}

// Information about the running task.
pub struct TaskInfo {
    pub name: &'static str,
    pub dump: bool,
    stdout_dump: &'static str,
    stderr_dump: &'static str,
    stdout_dump_file: Option<std::fs::File>,
    stderr_dump_file: Option<std::fs::File>,
}
static mut TASK_INFO: TaskInfo = TaskInfo {
    name: "",
    dump: false,
    stdout_dump: "dump.log",
    stderr_dump: "dump.log",
    stdout_dump_file: None,
    stderr_dump_file: None,
};
impl TaskInfo {
    pub fn get() -> &'static mut TaskInfo {
        unsafe { &mut TASK_INFO }
    }
    pub fn set_stdout_dump(&mut self, file: &'static str) {
        self.stdout_dump_file = None;
        self.stdout_dump = file;
    }
    pub fn stdout_dump(&mut self) -> Option<std::fs::File> {
        if self.dump && !self.stdout_dump.is_empty() {
            if self.stdout_dump_file.is_none() {
                self.stdout_dump_file = std::fs::File::create(self.stdout_dump).ok();
            }
            self.stdout_dump_file.as_ref().and_then(|f| f.try_clone().ok())
        } else {
            None
        }
    }
    pub fn stderr_dump(&mut self) -> Option<std::fs::File> {
        if self.dump && !self.stderr_dump.is_empty() {
            if self.stderr_dump_file.is_none() {
                if self.stderr_dump == self.stdout_dump {
                    let file = self.stdout_dump();
                    self.stderr_dump_file = file;
                } else {
                    self.stdout_dump_file = std::fs::File::create(self.stdout_dump).ok();
                }
            }
            self.stderr_dump_file.as_ref().and_then(|f| f.try_clone().ok())
        } else {
            None
        }
    }
}

// Get all paths to `dir/*/Cargo.toml`
pub fn top_cargo_toml(dir: &str) -> Vec<String> {
    glob(&format!("{}/*/Cargo.toml", dir))
}

// Get all `dir/**/*.rs` files.
pub fn all_rs(dir: &str) -> Vec<String> {
    glob(&format!("{}/**/*.rs", dir))
}

fn glob(pattern: &str) -> Vec<String> {
    match glob::glob(pattern) {
        Ok(iter) => iter
            .filter_map(|r| match r {
                Ok(p) => Some(p.to_string_lossy().into_owned()),
                Err(e) => {
                    error(e);
                    None
                }
            })
            .collect(),
        Err(e) => {
            error(e);
            return vec![];
        }
    }
}

pub fn println(msg: impl std::fmt::Display) {
    if let Some(mut dump) = TaskInfo::get().stdout_dump() {
        writeln!(dump, "{}", msg).ok();
    } else {
        println!("{}", msg);
    }
}
pub fn print(msg: impl std::fmt::Display) {
    if let Some(mut dump) = TaskInfo::get().stdout_dump() {
        write!(dump, "{}", msg).ok();
    } else {
        print!("{}", msg);
        std::io::stdout().lock().flush().ok();
    }
}

// Prints an error message, use `error(f!("{}", .."))` for formatting.
pub fn error(msg: impl std::fmt::Display) {
    if let Some(mut dump) = TaskInfo::get().stderr_dump() {
        writeln!(dump, "{}error{}: {}{} {}", c_red(), c_wb(), DO, c_w(), msg).ok();
    } else {
        eprintln!("{}error{}: {}{} {}", c_red(), c_wb(), DO, c_w(), msg);
    }
}

// Prints an [`error`] and exists with code `-1`.
pub fn fatal(msg: impl std::fmt::Display) -> ! {
    error(msg);
    process::exit(-1)
}

// ANSI colors.
pub fn c_green() -> &'static str {
    color("\x1B[1;32m")
}
pub fn c_red() -> &'static str {
    color("\x1B[1;31m")
}
pub fn c_wb() -> &'static str {
    color("\x1B[1;37m")
}
pub fn c_w() -> &'static str {
    color("\x1B[0m")
}
fn color(color: &str) -> &str {
    if TaskInfo::get().dump || !unsafe { ANSI_ENABLED } {
        ""
    } else {
        color
    }
}
static mut ANSI_ENABLED: bool = false;
