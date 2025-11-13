use std::{error::Error, fs::{self, remove_file}, io::Write, path::PathBuf};

use base64::{Engine, engine::general_purpose};
use bt_logger::{get_error, log_error};
use sha3::{Digest, Sha3_512};

use crate::folder_manager::get_local_usr_data_path;

///BTCache provides a caching mechanism for downloading and storing files from URLs. 
///It generates SHA3-512 hashes of URLs to create unique file names and manages local storage of cached files.
pub struct BTCache{
    ///folder_path: A PathBuf containing the directory path where cached files are stored.
    folder_path: PathBuf,
}

impl BTCache {
    ///Constructor
    /// Creates a new BTCache instance by determining the local user data path for the cache directory. 
    /// The method uses get_local_usr_data_path to construct the appropriate directory path based on the application folder name and cache subdirectory.
    /// 
    /// #Parameters:
    ///     * app_folder_name: An optional string slice that specifies the application folder name. If None, a default folder name will be used.
    /// 
    /// #Returns"
    ///     * Result<Self, Box<dyn Error>>: Returns a BTCache instance on success, or an error if the local data path cannot be determined.
    pub fn new(app_folder_name: Option<&str>) -> Result<Self, Box<dyn Error>>{
        let local_path = get_local_usr_data_path(app_folder_name, Some("cache"), true)?;
        Ok(
            Self { folder_path: PathBuf::from(local_path) }
        )
    }

    ///Generate a Sha3_512 hash for the given String encoded with base64 URLSAFE no padding
    ///This ensures a consistent, unique identifier for each URL that can be safely used as a filename.
    /// 
    /// #Parameters
    ///     * input: A string slice containing the URL to be hashed.
    /// 
    /// #Returns
    ///     * String: A base64 URL-safe encoded SHA3-512 hash of the input string.
    fn get_hash_string_base64(input: &str) -> String {
        let mut hasher = Sha3_512::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        general_purpose::URL_SAFE_NO_PAD.encode(result)
    }

    ///Helper Method. Downloads a file from the specified URL and saves it to the given file path.
    ///Uses reqwest for HTTP requests and writes the response bytes directly to a file.
    /// 
    ///#Parameters
    /// * url: A string slice containing the URL to download.
    /// * int_file_path: A reference to a PathBuf specifying where the downloaded file should be saved.
    ///#Returns
    /// *   Result<(), Box<dyn Error>>: Returns Ok(()) on successful download, or an error if the download or file creation fails.
    fn download_file(url: &str, int_file_path: &PathBuf) -> Result<(), Box<dyn Error>>{
        let download_response = reqwest::blocking::get(url)?;
        let bytes = download_response.bytes()?;
        let mut file = fs::File::create(int_file_path)?;
        file.write_all(&bytes)?;

        Ok(())
    }

    ///Attempts to retrieve a local file path for a given URL. The method:
    ///The method handles both cases where the file path check fails entirely (logging an error and attempting to download) 
    ///and where the file doesn't exist at the expected location (performing a download).
    /// 
    /// #Parameters:
    ///     * url: A string slice containing the URL of the file to retrieve from cache.
    /// 
    /// #Returns:
    ///     * Result<String, Box<dyn Error>>: Returns the full local file path as a string on success, or an error if:
    ///                                         The file path cannot be retrieved due to invalid Unicode
    ///                                         File operations fail during download or path checking    
    pub fn get_local_file_path(&self, url: &str) -> Result<String,Box<dyn Error>> {
        let int_file_path = self.folder_path.join(Self::get_hash_string_base64(url));
        let path_check = int_file_path.try_exists();
        if path_check.is_err() {
            log_error!("get_local_file_path","Issue finding file '{:?}' trying downloading again",int_file_path);
            Self::download_file(url, &int_file_path)?;
        }else{
            if !path_check.unwrap(){
                //File not found
                Self::download_file(url, &int_file_path)?;
            }
        }
        if let Some(full_path) = int_file_path.to_str(){
            return Ok(full_path.to_owned())
        }else{
            return Err(get_error!("get_local_file_path","Unable to retrieve cached file path. Invalid Unicode Path").into())
        }
    } 

    ///Encodes the file bytes using standard base64 encoding
    /// 
    ///#Parameters
    ///     * url: A string slice (&str) containing the URL of the file to retrieve
    /// 
    ///#Returns
    ///    Result<String, Box<dyn Error>>: Returns a String containing the base64-encoded file data on success, or an error if:
    ///    The local file path cannot be determined
    ///    The file cannot be read from the local cache
    ///    Base64 encoding fails
    pub fn get_file_data_base64(&self, url: &str) -> Result<String,Box<dyn Error>> {
        let full_file_path = self.get_local_file_path(url)?;
        let file_data_bytes = fs::read(full_file_path)?;
        Ok(general_purpose::STANDARD.encode(file_data_bytes))
    }

    ///The invalidate_cache function is responsible for removing a cached file from the local filesystem. 
    ///This function is typically used to clear stale or outdated cache entries, ensuring that subsequent requests for the specified URL 
    ///will fetch fresh data rather than using cached content.
    /// 
    /// #Parameters
    /// * url: &str, A string slice representing the URL of the cached resource to be invalidated. This parameter is used to determine which cached file should be removed
    /// 
    /// #Returns
    /// Result<(), Box<dyn Error>>
    ///     * Success: Ok(()) - Indicates that the cache file was successfully removed
    ///     * Error: Err(Box<dyn Error>) - Contains a boxed error object describing what went wrong during the cache invalidation process
    pub fn invalidate_cache(&self, url: &str)-> Result<(),Box<dyn Error>> {
        let full_file_path = self.get_local_file_path(url)?;
        let _r = remove_file(full_file_path)?;
        Ok(())
    }

    ///The refresh_cache function is designed to refresh or revalidate a cached resource by first invalidating the existing cache entry 
    ///and then returning the local file path where the refreshed content is stored.
    /// 
    /// #Parameters
    /// * url: &str, A string slice representing the URL of the cached resource to be invalidated. This parameter is used to determine which cached file should be removed
    /// 
    /// #Returns
    /// Result<(), Box<dyn Error>>:
    ///     * Success: Ok(String) - Returns the local file path where the refreshed cache content is stored
    ///     * Error: Err(Box<dyn Error>) - Contains a boxed error object describing what went wrong during the cache refresh process
    pub fn refresh_cache(&self, url: &str)-> Result<String,Box<dyn Error>> {
       self.invalidate_cache(url)?;
       let file_path = self.get_local_file_path(url)?;
       Ok(file_path)
    }
}

//************** */
//UNIT TEST    **/
//************* */
#[cfg(test)]
mod bt_cache_tests {
    use regex::Regex;

    use super::*;

    const FILE_URL: &str = "https://avatars.githubusercontent.com/u/188628667?v=4";
    const APP_NAME: &str = "bt_cache";    

    #[test]
    fn test_get_file_path_success() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_local_file_path(FILE_URL).unwrap();
        let re = Regex::new(r"^/home/.*/\.local/share/bt_cache/cache/Pe2MEfGkJXVt54yoLZ2ziRh9v4fGIJcRWQE98MtwcYTSNgJyE4ec6lZ4tSdolTCN9SA-wVrhmtP-8HJ-7jVWGg").unwrap();   
        assert!(re.is_match(&p));
    }

    #[test]
    fn test_get_file_data_success() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_file_data_base64(FILE_URL);
        assert!(p.is_ok());
    }    

    #[test]
    fn test_get_file_data_fail() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_file_data_base64("http://invalidurl.com/fake_file.unknown");
        assert!(p.is_err());
    }    

    #[test]
    fn test_invaldiate_success() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let _ = local_cache.get_file_data_base64(FILE_URL);
        let r = local_cache.invalidate_cache(FILE_URL);
        assert!(r.is_ok())
    }

    #[test]
    fn test_invaldiate_fail() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let r = local_cache.invalidate_cache("http://invalidurl.com/fake_file.unknown");
        assert!(r.is_err())
    }    
    
    #[test]
    fn test_refresh_success() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let r = local_cache.refresh_cache(FILE_URL);
        assert!(r.is_ok())
    }

}