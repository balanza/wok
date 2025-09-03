use once_cell::sync::Lazy;
/// This module takes care of writing on a file descriptor.
/// The module constructor takes the file descriptor number as an argument.
/// The file descriptor is wrapped in a Mutex to ensure thread safety.
/// The file descriptor is opened only once and reused for subsequent writes.
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::sync::Mutex;

static FILE_DESCRIPTORS: Lazy<Mutex<HashMap<&'static str, &'static Option<Mutex<File>>>>> =
    Lazy::new(|| {
        let m = HashMap::new();
        Mutex::new(m)
    });

/// Write content to the given file descriptor if it exists.
/// If the file descriptor does not exist, do nothing and return Ok(()).
/// This function is thread-safe.
/// # Arguments
/// * `fd` - The file descriptor number to write to.
/// * `content` - The content to write to the file descriptor.
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Ok(()) if successful, Err otherwise.   
pub fn write_to_fd(fd: i32, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(file_mutex) = get_or_insert(fd) {
        let mut file = file_mutex.lock().unwrap();
        writeln!(&mut *file, "{}", content)?;
        file.flush()?;
        std::mem::forget(file);
    }

    Ok(())
}

fn get_or_insert(fd: i32) -> &'static Option<Mutex<File>> {
    let mut map = FILE_DESCRIPTORS.lock().unwrap();
    let key = format!("fd{}", fd);
    let found = map.get(&key as &str);

    if let Some(item) = found.copied() {
        return item;
    }

    let file_mutex_opt = unsafe {
        // Check if fd exists
        let flags = libc::fcntl(fd, libc::F_GETFD);
        if flags == -1 {
            // fd doesn't exist, return None
            None
        } else {
            // fd exists, create File from it
            let file = File::from_raw_fd(fd);
            Some(Mutex::new(file))
        }
    };

    let leaked_key: &'static str = Box::leak(key.into_boxed_str());
    map.insert(leaked_key, Box::leak(Box::new(file_mutex_opt)));

    map.get(leaked_key).copied().unwrap()
}

#[cfg(test)]
mod tests {
    use super::write_to_fd;

    #[test]
    fn test_write_to_invalid_fd() {
        let invalid_fd = 9999; // Assuming this fd is invalid
        let result = write_to_fd(invalid_fd, "This should not panic.");
        assert!(result.is_ok()); // Should not panic, just do nothing
    }
}
