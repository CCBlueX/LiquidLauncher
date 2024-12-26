#[macro_export]
macro_rules! join_and_mkdir {
    ($path:expr, $join:expr) => {{
        let path = $path.join($join);
        $crate::mkdir!(&path);
        path
    }};
}

#[macro_export]
macro_rules! join_and_mkdir_vec {
    ($path:expr, $joins:expr) => {{
        let mut path = $path.to_path_buf();
        for join in $joins {
            path = path.join(join);
            $crate::mkdir!(&path);
        }
        path
    }};
}

#[macro_export]
macro_rules! mkdir {
    ($path:expr) => {
        if !$path.exists() {
            if let Err(e) = std::fs::create_dir_all(&$path) {
                error!("Failed to create directory {:?}: {}", $path, e);
            }
        }
    };
}
