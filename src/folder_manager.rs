use std::{error::Error, fs, path::PathBuf};

use once_cell::sync::Lazy;


///The get_local_usr_data_path function determines the appropriate local user data directory for an application based on the operating system, 
///and optionally creates the directory structure if it doesn't exist.

///A lazy static variable that determines the base user data directory path based on the target operating system. 
///This variable is initialized only when first accessed and uses the Lazy initialization pattern for efficiency.
/// Windows: Uses the %LOCALAPPDATA% environment variable, falling back to current directory
/// macOS: Uses the user's home directory joined with Library/Application Support
/// Linux: Uses either $XDG_DATA_HOME or $HOME/.local/share, falling back to current directory
/// Android: Uses the home directory (placeholder for sandboxed environment)
/// iOS: Uses the home directory joined with Documents
/// Other/Unknown Platforms: Falls back to current directory
static DATA_PATH: Lazy<PathBuf> = Lazy::new(|| {
    const FALLBACK_DIR: &str = ".";

    #[cfg(target_os = "windows")]
    {
        use std::{env, path::PathBuf};
        let base = env::var("LOCALAPPDATA").unwrap_or_else(|_| FALLBACK_DIR.to_string());
        return PathBuf::from(base); 
    }

    #[cfg(target_os = "macos")]
    {
        let base = env::var("HOME").unwrap_or_else(|_| FALLBACK_DIR.to_string());
        return PathBuf::from(base)
            .join("Library")
            .join("Application Support");
    }

    #[cfg(target_os = "linux")]
    {
        use std::env;

        let base = env::var("XDG_DATA_HOME")
            .or_else(|_| env::var("HOME").map(|h| format!("{}/.local/share", h)))
            .unwrap_or_else(|_| FALLBACK_DIR.to_string());
        return PathBuf::from(base);
    }

    #[cfg(target_os = "android")]
    {
        // Android apps typically use context-specific storage, but here's a fallback
        let base = env::var("HOME").unwrap_or_else(|_| FALLBACK_DIR.to_string());
        return PathBuf::from(base);//.join(app_name);
    }

    #[cfg(target_os = "ios")]
    {
        // iOS apps use sandboxed directories; this is a placeholder
        let base = env::var("HOME").unwrap_or_else(|_| FALLBACK_DIR.to_string());
        return PathBuf::from(base).join("Documents");//.join(app_name);
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android",
        target_os = "ios"
    )))]
    {
        // Fallback for unknown platforms
        PathBuf::from(FALLBACK_DIR)
    }
});

///This function constructs a complete file system path for application data storage
///# Parameters
/// * app_folder_name: An optional string slice (Option<&str>) that specifies the application folder name. If None or empty, no application-specific subdirectory is created.
/// * subfolder: An optional string slice (Option<&str>) that specifies a subfolder within the application directory. If None or empty, no subfolder is added.
/// * create_if_not_exists: A boolean flag that determines whether to create the directory structure if it doesn't exist.
///
/// Returns
///    Result<String, Box<dyn Error>>: Returns a String containing the full path as a string on success, or an error if:
///     Directory creation fails when create_if_not_exists is true
///     Path conversion to string fails
pub fn get_local_usr_data_path(app_folder_name: Option<&str>, subfolder: Option<&str>, create_if_not_exists: bool) -> Result<String, Box<dyn Error>>{
    let mut d_path = DATA_PATH.to_path_buf();
    if let Some (asf) = app_folder_name{
        if asf.trim().len() > 0 {
             d_path = d_path.join(asf.trim());
        }
    }

    if let Some(sf) = subfolder{
        if sf.trim().len() > 0 {
            d_path = d_path.join(sf.trim());
        }
    }

    let path = d_path.to_string_lossy().into_owned();
    if create_if_not_exists{
         fs::create_dir_all(d_path)?;
    }

    Ok(path)
}

//*************** */
//UNIT TEST     **/
//************** */
#[cfg(test)]
mod info_app_tests {
    use regex::Regex;

    use super::*;
    const APP_NAME: &str = "bt_file_cache";

    #[test]
    fn test_get_app_path_no_create_success() {
        let subfolder = "db";
        let re = Regex::new(r"^/home/.*/\.local/share/bt_file_cache/db").unwrap();
        let df = get_local_usr_data_path(Some(APP_NAME), Some(subfolder), false).unwrap();
        //assert_eq!(get_local_usr_data_path(Some("db"), false).unwrap(),"");
        assert!(re.is_match(&df));
    }

    #[test]
    fn test_get_app_path_create_fail() {
        let subfolder = ":/?\0'"; //Should not be able to create this folder
        let df = get_local_usr_data_path(Some(APP_NAME), Some(subfolder), true);
        assert!(df.is_err())
    }

        #[test]
    fn test_get_app_path_create_success() {
        let subfolder = "db.test"; //Should not be able to create this folder
        let re = Regex::new(r"^/home/.*/\.local/share/bt_file_cache/db.test").unwrap();        
        let df = get_local_usr_data_path(Some(APP_NAME), Some(subfolder), true);
        assert!(df.is_ok());
        assert!(re.is_match(&df.unwrap()));
    }
}
