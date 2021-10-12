use super::Error;
use std::{
    ffi::{OsStr, OsString},
    io::{BufRead, BufReader, Write},
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    thread::JoinHandle,
};

/// Compiler / linker tool.
pub struct Tool {
    /// Tool path.
    path: PathBuf,
    /// Command line arguments.
    args: Vec<OsString>,
}

impl Tool {
    /// Gets new instance of compiler tool.
    pub fn new() -> Result<Self, Error> {
        let mut c = cc::Build::new();
        println!("Detecting compiler tool...");
        c.no_default_flags(true);
        let tool = c.try_get_compiler()?;

        let r = Self {
            path: tool.path().to_path_buf(),
            args: Vec::new(),
        };

        println!("Found compiler tool: '{}'", r.description());

        Ok(r)
    }

    /// Adds a command line argument.
    fn push_str(&mut self, s: &str) {
        self.args.push(OsString::from(s));
    }

    /// Creates a shared library path from the given directory and library name.
    pub fn make_shared_lib_path(
        out_dir: &Path,
        lib_name: &OsStr,
    ) -> Result<PathBuf, Error> {
        let lib_path = &format!("lib{}.so", lib_name.to_str().unwrap());
        let res = out_dir.to_path_buf().join(lib_path);
        Ok(res)
    }

    /// Sets output path.
    pub fn set_output(&mut self, out_path: &Path) {
        self.push_str("-o");
        self.push_str(out_path.to_str().unwrap());
    }

    /// Adds a library reference.
    pub fn add_lib(&mut self, name: &OsStr) {
        self.push_str(&format!("-l{}", name.to_str().unwrap()));
    }

    /// Adds a static library reference.
    pub fn add_archive(&mut self, name: &OsStr) {
        self.push_str(&format!("-l:lib{}.a", name.to_str().unwrap()));
    }

    /// Adds a library search directory.
    pub fn add_lib_dir(&mut self, path: &Path) {
        self.push_str(&format!("-L{}", path.to_str().unwrap()));
    }

    /// Adds a compiler flag.
    pub fn add_flag(&mut self, s: &str) {
        self.push_str(&format!("-{}", s));
    }

    /// Adds a linker flag.
    pub fn add_linker_flag(&mut self, s: &str) {
        self.push_str(&format!("-Wl,--{}", s));
    }

    /// Runs the compiler tool.
    pub fn run(&mut self) -> Result<(), Error> {
        let mut cmd = Command::new(&self.path);
        cmd.args(&self.args);

        let desc = self.description();

        println!("Executing tool '{}': {:?}", desc, cmd);

        let (status, _) = run_command(&mut cmd, &desc, Some("cc"))?;

        if status.success() {
            Ok(())
        } else {
            Err(Error::Generic(format!(
                "Command '{}' failed: {}",
                desc, status
            )))
        }
    }

    /// Returns tool's description.
    fn description(&self) -> String {
        self.path.to_str().unwrap().to_string()
    }
}

/// Runs a shell command and returns its output.
pub fn run_command(
    cmd: &mut Command,
    desc: &str,
    short_desc: Option<&str>,
) -> Result<(ExitStatus, Vec<OsString>), Error> {
    let (mut child, out_reader) = spawn_child(cmd, desc, short_desc)?;

    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => {
            return Err(Error::Generic(format!(
                "Failed to wait on spawned child process '{}': {}",
                desc, e
            )));
        }
    };

    let lines = match out_reader.join() {
        Ok(lines) => lines,
        Err(e) => {
            return Err(Error::Generic(format!(
                "Failed to read output of child process  '{}': {:?}",
                desc, e
            )));
        }
    };

    Ok((status, lines))
}

/// Spawns a child process and returns a thread handle that reads its standard
/// output.
fn spawn_child(
    cmd: &mut Command,
    desc: &str,
    short_desc: Option<&str>,
) -> Result<(Child, JoinHandle<Vec<OsString>>), Error> {
    match cmd.stdout(Stdio::piped()).spawn() {
        Ok(mut child) => {
            let short_desc = short_desc.map(String::from);

            let stddout = BufReader::new(child.stdout.take().unwrap());

            let out_reader = thread::spawn(move || {
                let mut lines = Vec::new();
                for line in stddout.split(b'\n').filter_map(|l| l.ok()) {
                    if let Some(ref s) = short_desc {
                        print!("[{}] ", s);
                        std::io::stdout().write_all(&line).unwrap();
                        println!();
                    }

                    lines.push(OsString::from_vec(line));
                }
                lines
            });

            Ok((child, out_reader))
        }
        Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => Err(
            Error::Generic(format!("Command '{}' not found: {}", desc, e)),
        ),
        Err(e) => Err(Error::Generic(format!(
            "Failed to spawn a child process '{}': {}",
            desc, e
        ))),
    }
}
