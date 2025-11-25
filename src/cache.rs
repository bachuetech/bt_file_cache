use std::{env, error::Error, fs::{self, remove_file}, io::Write, path::PathBuf, time::Duration};

use base64::{Engine, engine::general_purpose};
use bt_logger::{get_error, log_error};
use once_cell::sync;
use reqwest::{Client, Url};
use sha3::{Digest, Sha3_512};

use crate::folder_manager::get_local_usr_data_path;

static DEFAULT_USER_AGENT: sync::Lazy<String> = sync::Lazy::new(||{
    format!("Mozilla/5.0 ({}; {}; {}) {}/{}", env::consts::FAMILY, env::consts::OS, env::consts::ARCH, option_env!("CARGO_PKG_NAME").unwrap_or("bt_file_cache"), option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.1b"))
});

static HTTP_CLIENT: sync::Lazy<Client> = sync::Lazy::new(||{ 
        const CLIENT_REQUEST_TIMEOUT: u64 = 10;
        if let Ok(c) = reqwest::Client::builder()
                                        .timeout(Duration::from_secs(CLIENT_REQUEST_TIMEOUT))
                                        .user_agent(DEFAULT_USER_AGENT.clone())
                                        .build(){
            c
        }else{
            reqwest::Client::new()
        }
    });

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

    ///ASYNC Helper Method. Downloads a file from the specified URL and saves it to the given file path.
    ///Uses reqwest for HTTP requests and writes the response bytes directly to a file.
    /// 
    ///#Parameters
    /// * url: A string slice containing the URL to download.
    /// * int_file_path: A reference to a PathBuf specifying where the downloaded file should be saved.
    ///#Returns
    /// *   Result<(), Box<dyn Error>>: Returns Ok(()) on successful download, or an error if the download or file creation fails.
    async fn download_file_async(url: &str, int_file_path: &PathBuf, token: Option<&str>) -> Result<(), Box<dyn Error>>{
        let parsed_url = Url::parse(url)?;
        let mut request_builder = HTTP_CLIENT.get(parsed_url);
        if  token.is_some(){
            request_builder = request_builder.bearer_auth(token.unwrap());
        }
        let response = request_builder.send().await?; //reqwest::blocking::get(url)?;
        let bytes = response.bytes().await?;
        let mut file = fs::File::create(int_file_path)?;
        file.write_all(&bytes)?;

        Ok(())
    }

    ///ASYNC Function that attempts to retrieve a local file path for a given URL. The method:
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
    pub async fn get_local_file_path_async(&self, url: &str) -> Result<String,Box<dyn Error>> {
        self.get_local_file_path_with_name_token_async(url, url, None).await
    } 

    ///ASYNC Function that attempts to retrieve a local file path for a given URL. The method:
    ///The method handles both cases where the file path check fails entirely (logging an error and attempting to download) 
    ///and where the file doesn't exist at the expected location (performing a download).
    /// 
    /// #Parameters:
    ///     * url: A string slice containing the URL of the file to retrieve from cache.
    ///     * file_name: desire file name or file id. Useful when file may associuted to multiple URLs
    /// 
    /// #Returns:
    ///     * Result<String, Box<dyn Error>>: Returns the full local file path as a string on success, or an error if:
    ///                                         The file path cannot be retrieved due to invalid Unicode
    ///                                         File operations fail during download or path checking    
    pub async fn get_local_file_path_with_name_async(&self, url: &str, file_name: &str) -> Result<String,Box<dyn Error>> {
        self.get_local_file_path_with_name_token_async(url, file_name, None).await
        /*let int_file_path = self.get_file(file_name); //self.folder_path.join(Self::get_hash_string_base64(file_name));

        let path_check = int_file_path.try_exists();
        if path_check.is_err() {
            log_error!("get_local_file_path","Issue finding file '{:?}' trying downloading again",int_file_path);
            Self::download_file_async(url, &int_file_path, None).await?;
        }else{
            if !path_check.unwrap(){
                //File not found
                Self::download_file_async(url, &int_file_path, None).await?;
            }
        }        

        if let Some(full_path) = int_file_path.to_str(){
            return Ok(full_path.to_owned())
        }else{
            return Err(get_error!("get_local_file_path","Unable to retrieve cached file path. Invalid Unicode Path").into())
        }*/
    }    

    ///ASYNC Function that attempts to retrieve a local file path for a given URL. The method:
    ///The method handles both cases where the file path check fails entirely (logging an error and attempting to download) 
    ///and where the file doesn't exist at the expected location (performing a download).
    /// 
    /// #Parameters:
    ///     * url: A string slice containing the URL of the file to retrieve from cache.
    ///     * file_name: desire file name or file id. Useful when file may associuted to multiple URLs
    ///     * Token: Access token to be use to access the URL resource
    /// 
    /// #Returns:
    ///     * Result<String, Box<dyn Error>>: Returns the full local file path as a string on success, or an error if:
    ///                                         The file path cannot be retrieved due to invalid Unicode
    ///                                         File operations fail during download or path checking    
    pub async fn get_local_file_path_with_name_token_async(&self, url: &str, file_name: &str, token: Option<&str>) -> Result<String,Box<dyn Error>> {
        let int_file_path = self.get_file(file_name); //self.folder_path.join(Self::get_hash_string_base64(file_name));

        let path_check = int_file_path.try_exists();
        if path_check.is_err() {
            log_error!("get_local_file_path","Issue finding file '{:?}' trying downloading again",int_file_path);
            Self::download_file_async(url, &int_file_path, token).await?;
        }else{
            if !path_check.unwrap(){
                //File not found
                Self::download_file_async(url, &int_file_path, None).await?;
            }
        }        

        if let Some(full_path) = int_file_path.to_str(){
            return Ok(full_path.to_owned())
        }else{
            return Err(get_error!("get_local_file_path","Unable to retrieve cached file path. Invalid Unicode Path").into())
        }
    }        

    ///Async function that encodes the file bytes using standard base64 encoding
    /// 
    ///#Parameters
    ///     * url: A string slice (&str) containing the URL of the file to retrieve
    /// 
    ///#Returns
    ///    Result<String, Box<dyn Error>>: Returns a String containing the base64-encoded file data on success, or an error if:
    ///    The local file path cannot be determined
    ///    The file cannot be read from the local cache
    ///    Base64 encoding fails
    pub async fn get_file_data_base64_async(&self, url: &str) -> Result<String,Box<dyn Error>> {
        let full_file_path = self.get_local_file_path_with_name_token_async(url, url, None).await?;
        let file_data_bytes = fs::read(full_file_path)?;
        Ok(general_purpose::STANDARD.encode(file_data_bytes))
    }    

    ///Async function that encodes the file bytes using standard base64 encoding
    /// 
    ///#Parameters
    ///     * url: A string slice (&str) containing the URL of the file to retrieve
    ///     * file_name: desire file name or file id. Useful when file may associuted to multiple URLs
    /// 
    ///#Returns
    ///    Result<String, Box<dyn Error>>: Returns a String containing the base64-encoded file data on success, or an error if:
    ///    The local file path cannot be determined
    ///    The file cannot be read from the local cache
    ///    Base64 encoding fails
    pub async fn get_file_data_base64_with_name_async(&self, url: &str, file_name: &str) -> Result<String,Box<dyn Error>> {
        let full_file_path = self.get_local_file_path_with_name_token_async(url, file_name, None).await?;
        let file_data_bytes = fs::read(full_file_path)?;
        Ok(general_purpose::STANDARD.encode(file_data_bytes))
    } 

    ///Async function that encodes the file bytes using standard base64 encoding
    /// 
    ///#Parameters
    ///     * url: A string slice (&str) containing the URL of the file to retrieve
    ///     * file_name: desire file name or file id. Useful when file may associuted to multiple URLs
    ///     * token: Access token to access the URL resource
    /// 
    ///#Returns
    ///    Result<String, Box<dyn Error>>: Returns a String containing the base64-encoded file data on success, or an error if:
    ///    The local file path cannot be determined
    ///    The file cannot be read from the local cache
    ///    Base64 encoding fails
    pub async fn get_file_data_base64_with_name_token_async(&self, url: &str, file_name: &str, token: Option<&str>) -> Result<String,Box<dyn Error>> {
        let full_file_path = self.get_local_file_path_with_name_token_async(url, file_name, token).await?;
        let file_data_bytes = fs::read(full_file_path)?;
        Ok(general_purpose::STANDARD.encode(file_data_bytes))
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
        let parsed_url = Url::parse(url)?;        
        let download_response = reqwest::blocking::get(parsed_url)?;
        let bytes = download_response.bytes()?;
        let mut file = fs::File::create(int_file_path)?;
        file.write_all(&bytes)?;

        Ok(())
    }

    //Build file name to standarize it
    fn get_file(&self, file_name: &str) -> PathBuf{
        self.folder_path.join(Self::get_hash_string_base64(file_name))
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
        let int_file_path = self.get_file(url); //self.folder_path.join(Self::get_hash_string_base64(url));
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
        let file = self.get_file(url);
        let exist_check =  file.try_exists();
        if exist_check.is_ok(){
            if exist_check.unwrap() {
                let full_file_path = file.to_str(); 
                if full_file_path.is_some(){//self.get_local_file_path(url)?;
                    let _r = remove_file(full_file_path.unwrap())?;
                }else{
                    return Err(get_error!("invalidate_cache","Error extractive file path (none)").into())
                }
            }else{
                return Err(get_error!("invalidate_cache","File not found").into())
            }
        }else{
            return Err(get_error!("invalidate_cache","File check Error: {}",exist_check.unwrap_err()).into())
        }
        Ok(())
    }

    ///ASYNC The invalidate_cache function is responsible for removing a cached file from the local filesystem. 
    ///This function is typically used to clear stale or outdated cache entries, ensuring that subsequent requests for the specified URL 
    ///will fetch fresh data rather than using cached content.
    /// 
    /// #Parameters
    /// * url_name_id: &str, A string slice representing the URL or Name ID of the cached resource to be invalidated. 
    ///                      This parameter is used to determine which cached file should be removed
    /// 
    /// #Returns
    /// Result<(), Box<dyn Error>>
    ///     * Success: Ok(()) - Indicates that the cache file was successfully removed
    ///     * Error: Err(Box<dyn Error>) - Contains a boxed error object describing what went wrong during the cache invalidation process
    pub async fn invalidate_cache_async(&self, url_name_id: &str)-> Result<(),Box<dyn Error>> {
        //let full_file_path = self.get_local_file_path_with_name_async(url_name_id,url_name_id).await?;
        //let _r = remove_file(full_file_path)?;
        self.invalidate_cache(url_name_id)?;
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

    ///ASYNC The refresh_cache function is designed to refresh or revalidate a cached resource by first invalidating the existing cache entry 
    ///and then returning the local file path where the refreshed content is stored.
    /// 
    /// #Parameters
    /// * url_name_id: &str, A string slice representing the URL or Name ID of the cached resource to be invalidated. This parameter is used to determine which cached file should be removed
    /// 
    /// #Returns
    /// Result<(), Box<dyn Error>>:
    ///     * Success: Ok(String) - Returns the local file path where the refreshed cache content is stored
    ///     * Error: Err(Box<dyn Error>) - Contains a boxed error object describing what went wrong during the cache refresh process
    pub async fn refresh_cache_async(&self, url_name_id: &str)-> Result<String,Box<dyn Error>> {
       self.invalidate_cache_async(url_name_id).await?;
       let file_path = self.get_local_file_path_with_name_token_async(url_name_id,url_name_id, None).await?;
       Ok(file_path)
    } 

    ///ASYNC The refresh_cache function is designed to refresh or revalidate a cached resource by first invalidating the existing cache entry 
    ///and then returning the local file path where the refreshed content is stored.
    /// 
    /// #Parameters
    /// * url: &str, A string slice representing the URL to refresh This parameter is used to determine which cached file should be removed
    /// * name: name of the file to store
    /// * token: access token to access the UTL resource
    /// #Returns
    /// Result<(), Box<dyn Error>>:
    ///     * Success: Ok(String) - Returns the local file path where the refreshed cache content is stored
    ///     * Error: Err(Box<dyn Error>) - Contains a boxed error object describing what went wrong during the cache refresh process
    pub async fn refresh_cache_with_name_async(&self, url: &str, name: &str, token: Option<&str>)-> Result<String,Box<dyn Error>> {
       self.invalidate_cache_async(name).await?;
       let file_path = self.get_local_file_path_with_name_token_async(url,name, token).await?;
       Ok(file_path)
    }       

    ///ASYNC The refresh_cache function is designed to refresh or revalidate a cached resource by first invalidating the existing cache entry 
    ///and then returning the local file path where the refreshed content is stored.
    /// 
    /// #Parameters
    /// * url: &str, A string slice representing the URL to refresh This parameter is used to determine which cached file should be removed
    /// * token: access token to access the UTL resource
    /// 
    /// #Returns
    /// Result<(), Box<dyn Error>>:
    ///     * Success: Ok(String) - Returns the local file path where the refreshed cache content is stored
    ///     * Error: Err(Box<dyn Error>) - Contains a boxed error object describing what went wrong during the cache refresh process
    pub async fn refresh_cache_with_token_async(&self, url: &str, token: Option<&str>)-> Result<String,Box<dyn Error>> {
       self.invalidate_cache_async(url).await?;
       let file_path = self.get_local_file_path_with_name_token_async(url,url, token).await?;
       Ok(file_path)
    }      
}

//************** */
//UNIT TEST    **/
//************* */
#[cfg(test)]
mod bt_cache_tests {
    use std::sync::Once;

    use bt_logger::{LogLevel, LogTarget, build_logger, log_verbose};
    use regex::Regex;

    use super::*;

    const FILE_URL: &str = "https://avatars.githubusercontent.com/u/188628667?v=4";
    //const FILE_URL: &str = "https://www.google.com/s2/favicons?sz=64&domain=indeed.com";
    const APP_NAME: &str = "bt_cache";    

    static INIT: Once = Once::new();
    fn ini_log() {
        INIT.call_once(|| {
            build_logger("BACHUETECH", "UNIT TEST RUST CACHE", LogLevel::VERBOSE, LogTarget::STD_ERROR, None );     
        });
    }

    #[test]
    fn test_get_file_path_success() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_local_file_path(FILE_URL).unwrap();
        let re = Regex::new(r"^/home/.*/\.local/share/bt_cache/cache/Pe2MEfGkJXVt54yoLZ2ziRh9v4fGIJcRWQE98MtwcYTSNgJyE4ec6lZ4tSdolTCN9SA-wVrhmtP-8HJ-7jVWGg").unwrap();   
        assert!(re.is_match(&p));
    }

    #[test]
    fn test_get_file_data_success() {
        ini_log();
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_file_data_base64(FILE_URL);
        //log_verbose!("test_get_file_data_success","Content: {:?}",p);
        assert!(p.is_ok());
    }    

    #[test]
    fn test_get_file_data_fail() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_file_data_base64("http://invalidurl.com/fake_file.unknown");
        log_verbose!("test_get_file_data_fail","Result {:?}",p);
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
    fn test_invalidate_fail() {
        ini_log();
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let r = local_cache.invalidate_cache("http://invalidurl.com/fake_file.unknown");
        log_verbose!("test_invalidate_fail","Result {:?}", r);
        assert!(r.is_err())
    }    
    
    #[test]
    fn test_refresh_success() {
        ini_log();
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let _ = local_cache.get_local_file_path(FILE_URL);
        let r = local_cache.refresh_cache(FILE_URL);
        log_verbose!("test_refresh_success","Result {:?}",r);
        assert!(r.is_ok())
    }
}

#[cfg(test)]
mod bt_cache_with_name_async_tests {
    use std::sync::Once;

    use regex::Regex;
    use bt_logger::{LogLevel, LogTarget, build_logger, log_verbose};
    use super::*;

    static INIT: Once = Once::new();
    fn ini_log() {
        INIT.call_once(|| {
            build_logger("BACHUETECH", "UNIT TEST RUST CACHE", LogLevel::VERBOSE, LogTarget::STD_ERROR, None );     
        });
    }

    const FILE_URL: &str = "https://www.google.com/s2/favicons?sz=64&domain=google.com";
    const FILE_URL2: &str = "https://www.google.com/s2/favicons?sz=64&domain=walmart.com";
    const FILE_URL3: &str = "https://www.google.com/s2/favicons?sz=64&domain=cnn.com";

    const APP_NAME: &str = "bt_cache";    

    #[tokio::test]
    async fn test_get_file_path_success_async() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_local_file_path_with_name_async(FILE_URL3,"file1").await.unwrap();
        let re = Regex::new(r"^/home/.*/\.local/share/bt_cache/cache/S6ZZaVaoYQsQDYaR7piO7byg3ThvoCZk4Gg4GWCVxac-qy1RvtBWTmdbS0OeEhJMviRfOG7fsVqGQcPoGIVD-w").unwrap();   
        assert!(re.is_match(&p));
    }

    #[tokio::test]
    async fn test_get_file_data_success_async() {
        ini_log();
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_file_data_base64_with_name_async(FILE_URL,"file2").await;
        //log_verbose!("test_get_file_data_success_async","File: {:?}",p);
        assert!(p.is_ok());
    }    

    #[tokio::test]
    async fn test_get_file_data_fail_async() {
        ini_log();
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_file_data_base64_with_name_async("http://invalidwesite.com/fake_file.unknown","useless_invalid").await;
        log_verbose!("test_get_file_data_fail_async","Result: {:?}",p);
        assert!(p.is_err());
    }    

    #[tokio::test]
    async fn test_invalidate_success_async() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let _ = local_cache.get_file_data_base64_with_name_async(FILE_URL2,"file4").await;
        let r = local_cache.invalidate_cache_async("file4").await;
        assert!(r.is_ok())
    }

    #[tokio::test]
    async fn test_refresh_success_async() {
        ini_log();
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let r = local_cache.refresh_cache_async(FILE_URL).await;
        log_verbose!("test_refresh_success_async","Res: {:?}",r);
        assert!(r.is_ok())
    }
}

#[cfg(test)]
mod bt_cache_async_tests {
    use std::sync::Once;

    use regex::Regex;
    use bt_logger::{LogLevel, LogTarget, build_logger, log_verbose};
    use super::*;

    static INIT: Once = Once::new();
    fn ini_log() {
        INIT.call_once(|| {
            build_logger("BACHUETECH", "UNIT TEST RUST CACHE", LogLevel::VERBOSE, LogTarget::STD_ERROR, None );     
        });
    }

    //const FILE_URL: &str = "https://s.gravatar.com/avatar/5ff5566fb3d2f67fe25a9ed89953d876?s=480&r=pg&d=https%3A%2F%2Fcdn.auth0.com%2Favatars%2Fce.png";
    const FILE_URL: &str = "https://www.google.com/s2/favicons?sz=64&domain=indeed.com";
    const FILE_URL2: &str = "https://www.google.com/s2/favicons?sz=64&domain=monster.com";
    const FILE_URL3: &str = "https://www.google.com/s2/favicons?sz=64&domain=quora.com";

    const APP_NAME: &str = "bt_cache";    

    #[tokio::test]
    async fn test_get_file_path_success_async() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_local_file_path_async(FILE_URL3).await.unwrap();
        let re = Regex::new(r"^/home/.*/\.local/share/bt_cache/cache/JV0VrQPUrmKposXxQFD0TwLbPHK1fQb_hOouSb3BvaG-xqIm_Lw9GKyCyTvVDlc2v29sT-5lQgytuJbEVv2eyA").unwrap();   
        assert!(re.is_match(&p));
    }

    #[tokio::test]
    async fn test_get_file_data_success_async() {
        ini_log();
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_file_data_base64_async(FILE_URL).await;
        log_verbose!("test_get_file_data_success_async","File: {:?}",p);
        assert!(p.is_ok());
    }    

    #[tokio::test]
    async fn test_get_file_data_fail_async() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let p = local_cache.get_file_data_base64_async("http://invalidurl.com/fake_file.unknown").await;
        assert!(p.is_err());
    }    

    #[tokio::test]
    async fn test_invalidate_success_async() {
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let _ = local_cache.get_file_data_base64_async(FILE_URL2).await;
        let r = local_cache.invalidate_cache_async(FILE_URL2).await;
        assert!(r.is_ok())
    }

    #[tokio::test]
    async fn test_refresh_success_async() {
        ini_log();
        let local_cache = BTCache::new(Some(APP_NAME)).unwrap();
        let r = local_cache.refresh_cache_async(FILE_URL).await;
        log_verbose!("test_refresh_success_async","Res: {:?}",r);
        assert!(r.is_ok())
    }
}