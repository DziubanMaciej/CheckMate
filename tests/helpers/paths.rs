pub fn get_cargo_bin(name: &str) -> Option<std::path::PathBuf> {
    fn target_dir() -> std::path::PathBuf {
        let mut path = std::env::current_exe().unwrap();
        path.pop();
        if path.ends_with("deps") {
            path.pop();
        }
        path
    }

    let env_var = format!("CARGO_BIN_EXE_{}", name);
    let path = std::env::var_os(&env_var)
        .map(|p| p.into())
        .unwrap_or_else(|| target_dir().join(format!("{}{}", name, std::env::consts::EXE_SUFFIX)));
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}
