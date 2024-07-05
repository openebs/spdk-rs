use super::{run_command, Error, Tool};
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    env,
    ffi::{OsStr, OsString},
    fmt::{self, Display, Formatter},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Returns contents of a PATH-like environment variable, or an empty vector if
/// the variable is not defined.
fn get_path_var<V: AsRef<OsStr>>(var_name: V) -> Vec<PathBuf> {
    match env::var_os(&var_name) {
        Some(val) => env::split_paths(&val).collect(),
        None => Vec::new(),
    }
}

/// Prints contents of a PATH-like environment variable.
#[allow(dead_code)]
pub fn print_path_var<V: AsRef<OsStr>>(prefix: &str, var: V) {
    let svar = OsString::from(&var);
    match env::var_os(&var) {
        Some(s) => {
            println!("{} {}:", prefix, svar.to_str().unwrap());
            env::split_paths(&s).for_each(|s| {
                println!("    {}", s.to_str().unwrap());
            });
        }
        None => {
            println!("{} {}: (undefined)", prefix, svar.to_str().unwrap());
        }
    }
}

/// Merges items of two PATH-like environment variables: it an item from the
/// second variable is not found in the first one, it is appended to the end of
/// the first. If the first one already has the same item as the second one, it
/// is ignored. The original order of items is preserved.
#[allow(dead_code)]
pub fn merge_path_var<U: AsRef<OsStr>, V: AsRef<OsStr>>(first: U, second: V) {
    let mut a = get_path_var(&first);
    let mut ex = BTreeSet::new();
    for p in &a {
        ex.insert(p.clone());
    }

    for p in get_path_var(&second) {
        if !ex.contains(&p) {
            ex.insert(p.clone());
            a.push(p);
        }
    }

    let dst = env::join_paths(a).unwrap();
    env::set_var(&first, dst);
}

/// Appends a new path to a PATH-like environment variable.
#[allow(dead_code)]
pub fn append_path_var<V: AsRef<OsStr>>(var_name: V, new_path: &Path) {
    let mut lst = get_path_var(&var_name);
    lst.push(PathBuf::from(&new_path));
    let new = env::join_paths(lst).unwrap();
    env::set_var(&var_name, new);
}

/// Library dependency.
#[derive(Debug, Clone)]
pub struct Library {
    /// Short library name (e.g. `z` for `-lz`).
    lib_name: OsString,
    /// Name as reported by pkg-config (e.g. `libz.a` for `-l:libz.a`).
    pkg_name: OsString,
    /// True if pkg-config reports as .a lib (e.g. `-l:libz.a`).
    is_ar: bool,
    /// True for system libs.
    is_system: bool,
    /// Name of the library that brought this dependency library.
    parent_name: OsString,
}

impl Eq for Library {}

impl PartialEq<Self> for Library {
    fn eq(&self, other: &Self) -> bool {
        self.lib_name == other.lib_name
    }
}

impl PartialOrd<Self> for Library {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Library {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.is_system.cmp(&other.is_system) {
            Ordering::Equal => self.lib_name.cmp(&other.lib_name),
            r => r,
        }
    }
}

impl Display for Library {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(
            fmt,
            "[{}] {} <- {}",
            self.flag_repr(),
            self.name_repr(),
            self.parent_name.to_str().unwrap()
        )
    }
}

impl Library {
    /// Creates a new `Library` instance.
    fn new<U: AsRef<OsStr>, V: AsRef<OsStr>>(
        name: U,
        parent_name: V,
        cfg: &LibraryConfig,
    ) -> Self {
        let pkg_name = name.as_ref().to_str().unwrap();

        let (lib_name, is_ar) =
            if pkg_name.starts_with(":lib") && pkg_name.ends_with(".a") {
                let s = pkg_name
                    .strip_prefix(":lib")
                    .unwrap()
                    .strip_suffix(".a")
                    .unwrap();
                (OsString::from(&s), true)
            } else {
                (OsString::from(&pkg_name), false)
            };

        let is_system = cfg.marked_system.contains(&lib_name);

        Self {
            lib_name,
            pkg_name: OsString::from(pkg_name),
            is_ar,
            is_system,
            parent_name: OsString::from(&parent_name),
        }
    }

    /// Prints Cargo link instructions.
    fn cargo(&self) {
        if self.is_system {
            println!(
                "cargo:rustc-link-lib={}",
                self.lib_name.to_str().unwrap()
            );
        } else {
            // Use 'whole-archive' flag on non-system libs.
            println!(
                "cargo:rustc-link-lib=static:+whole-archive,-bundle={}",
                self.lib_name.to_str().unwrap()
            );
        }
    }

    fn add(&self, tool: &mut Tool) {
        if self.is_ar {
            tool.add_archive(&self.lib_name);
        } else {
            tool.add_lib(&self.lib_name);
        }
    }

    fn flag_repr(&self) -> String {
        format!(
            "{}{}",
            if self.is_ar { "A" } else { "-" },
            if self.is_system { "S" } else { "-" }
        )
    }

    fn name_repr(&self) -> String {
        if self.lib_name.eq(&self.pkg_name) {
            self.lib_name.to_str().unwrap().to_string()
        } else {
            format!(
                "{} ({})",
                self.lib_name.to_str().unwrap(),
                self.pkg_name.to_str().unwrap()
            )
        }
    }
}

/// Library configuration.
#[derive(Default)]
pub struct LibraryConfig {
    /// Underlying pkg_config structure.
    cfg: pkg_config::Config,
    /// Set of excludes dependencies.
    excluded: BTreeSet<OsString>,
    /// Libraries treated as system.
    marked_system: BTreeSet<OsString>,
    /// Found library dependencies.
    libs: BTreeSet<Library>,
    /// Found library paths.
    lib_paths: BTreeSet<PathBuf>,
    /// Found include paths.
    inc_paths: BTreeSet<PathBuf>,
}

impl LibraryConfig {
    /// Creates a new library configuration instance.
    pub fn new() -> Self {
        let mut s = LibraryConfig::default();
        s.cfg.cargo_metadata(false);
        s.cfg.env_metadata(false);
        s
    }

    /// Adds a new pkg_config search path. The path must exist on the file
    /// system.
    #[allow(dead_code)]
    pub fn add_pkg_cfg_path<T: AsRef<Path>>(
        &self,
        new_path: T,
    ) -> Result<(), Error> {
        match fs::canonicalize(&new_path) {
            Ok(cp) => {
                println!(
                    "Added PKG_CONFIG_PATH_FOR_TARGET: {}",
                    cp.to_str().unwrap()
                );
                append_path_var(OsStr::new("PKG_CONFIG_PATH_FOR_TARGET"), &cp);
                Ok(())
            }
            Err(e) => Err(Error::Generic(format!(
                "Failed to add pkg config path '{}': {}",
                new_path.as_ref().to_str().unwrap(),
                e
            ))),
        }
    }

    /// Searches the file system for directories that contain .pc files.
    #[allow(dead_code)]
    pub fn find_pkg_config_dirs<T: AsRef<Path>>(
        &mut self,
        root_dir: T,
    ) -> Result<Vec<PathBuf>, Error> {
        let mut cmd = Command::new("find");
        cmd.arg(root_dir.as_ref());
        cmd.arg("-type").arg("d");
        cmd.arg("-name").arg("pkgconfig");

        let (status, lines) = run_command(&mut cmd, "find pkg config", None)?;

        if status.success() {
            let paths: Vec<PathBuf> = lines.iter().map(PathBuf::from).collect();
            for p in paths.iter() {
                self.add_pkg_cfg_path(p)?;
            }
            Ok(paths)
        } else {
            Err(Error::Generic(format!("Command 'find' failed: {status}")))
        }
    }

    /// Adds an excluded library.
    pub fn exclude_lib<T: AsRef<OsStr>>(&mut self, lib_name: T) {
        self.excluded.insert(OsString::from(&lib_name));
    }

    /// Marks a library as system.
    pub fn mark_system<T: AsRef<OsStr>>(&mut self, lib_name: T) {
        self.marked_system.insert(OsString::from(&lib_name));
    }

    /// Enables or disable static search pkg_config mode.
    pub fn set_static_search(&mut self, s: bool) {
        self.cfg.statik(s);
    }

    /// Adds an include directory. The path must exist on the file system.
    pub fn add_inc<T: AsRef<Path>>(&mut self, inc_dir: T) -> Result<(), Error> {
        match fs::canonicalize(&inc_dir) {
            Ok(cp) => {
                println!("Added include path: {}", cp.to_str().unwrap());
                self.inc_paths.insert(cp);
                Ok(())
            }
            Err(e) => Err(Error::Generic(format!(
                "Failed to add include path '{}': {}",
                inc_dir.as_ref().to_str().unwrap(),
                e
            ))),
        }
    }

    /// Adds an include directory. If the path does not exist on the file
    /// system, an alternative path is added.
    pub fn add_inc_alt<U: AsRef<Path>, V: AsRef<Path>>(
        &mut self,
        inc_dir: U,
        alt_inc_dir: V,
    ) -> Result<(), Error> {
        match self.add_inc(inc_dir.as_ref()) {
            Ok(r) => Ok(r),
            Err(_) => self.add_inc(alt_inc_dir.as_ref()),
        }
    }

    /// Finds a library via pkg_config.
    pub fn find_lib<T: AsRef<OsStr>>(
        &mut self,
        lib_name: T,
    ) -> Result<(), Error> {
        let lib_name = OsString::from(&lib_name);
        let library = self.cfg.probe(lib_name.to_str().unwrap())?;

        println!("Found pkg config dependencies for {lib_name:?}:");

        for name in &library.libs {
            let lib = Library::new(name, &lib_name, self);
            println!("    link lib {n:?}", n = lib.pkg_name);
            if self.is_lib_needed(&lib) {
                self.add_lib(lib);
            }
        }

        for file_path in &library.link_files {
            println!("    link file {file_path:?}");

            let mut dep_lib_name = OsString::from(":");
            dep_lib_name.push(file_path.file_name().unwrap());

            let lib = Library::new(&dep_lib_name, &lib_name, self);

            if self.is_lib_needed(&lib) {
                self.add_lib_path(file_path.parent().unwrap());
                self.add_lib(lib);
            }
        }

        for path in &library.link_paths {
            println!("    lib path {path:?}");
            self.add_lib_path(path);
        }

        for path in &library.include_paths {
            println!("    inc path {path:?}");
            self.inc_paths.insert(path.clone());
        }

        Ok(())
    }

    /// Checks if the lib is needed.
    pub fn is_lib_needed(&self, lib: &Library) -> bool {
        !self.excluded.contains(&lib.lib_name) && !self.libs.contains(lib)
    }

    /// Adds a libs.
    pub fn add_lib(&mut self, lib: Library) {
        self.libs.insert(lib);
    }

    /// Adds a lib search path.
    pub fn add_lib_path<T: AsRef<Path>>(&mut self, path: T) {
        if !self.lib_paths.contains(path.as_ref()) {
            let path = path.as_ref().to_owned();
            println!("Added lib search path: {path:?}");
            self.lib_paths.insert(path);
        }
    }

    /// Finds all given libraries via pkg-config.
    pub fn find_libs(&mut self, libs: &[&str]) -> Result<(), Error> {
        for name in libs.iter() {
            self.find_lib(name)?;
        }

        Ok(())
    }

    /// Returns list of include directories.
    #[allow(dead_code)]
    pub fn get_inc_paths(&self) -> Vec<PathBuf> {
        self.inc_paths.iter().map(PathBuf::from).collect()
    }

    /// Outputs cargo derictives to link to the found libraries.
    #[allow(dead_code)]
    pub fn cargo(&self) {
        for s in &self.lib_paths {
            println!("cargo:rustc-link-search={}", s.to_str().unwrap());
        }

        for s in &self.libs {
            s.cargo();
        }
    }

    /// TODO
    #[allow(dead_code)]
    pub fn dump(&self) {
        println!("**** Will link against libraries (A: archive, S: system):");
        for lib in self.libs.iter() {
            println!("    {lib}");
        }

        println!("**** Will use library paths:");
        for p in self.lib_paths.iter() {
            println!("    {}", p.to_str().unwrap());
        }

        println!("**** Will use include paths:");
        for p in self.inc_paths.iter() {
            println!("    {}", p.to_str().unwrap());
        }
    }

    /// Builds a shared (.so) library from all the libraries found previously.
    #[allow(dead_code)]
    pub fn build_shared_lib(
        &self,
        out_dir: &Path,
        lib_name: &OsStr,
    ) -> Result<PathBuf, Error> {
        let mut tool = Tool::new()?;

        tool.add_flag("shared");

        let lib_path = Tool::make_shared_lib_path(out_dir, lib_name)?;
        tool.set_output(&lib_path);

        self.libs.iter().filter(|p| p.is_system).for_each(|p| {
            p.add(&mut tool);
        });

        self.lib_paths.iter().for_each(|p| {
            tool.add_lib_dir(p);
        });

        tool.add_linker_flag("whole-archive");
        self.libs.iter().filter(|p| !p.is_system).for_each(|p| {
            p.add(&mut tool);
        });
        tool.add_linker_flag("no-whole-archive");

        tool.run()?;

        Ok(lib_path)
    }
}
