# Project Title
BT Cache

## Description
A caching mechanism for downloading and storing files from URLs. 
It generates SHA3-512 hashes of URLs to create unique file names and manages local storage of cached files.

## Usage
```
let cache = BTCache::new(Some("myapp"))?;
let base64_data = cache.get_file_data_base64("https://bachuetech.biz/fake_image.png")?;

match cache.refresh_cache("https://bachuetech.biz/fake_image.png") {
    Ok(file_path) => {
        // Proceed with fetching and storing new content at file_path
        println!("Cache refreshed. New content should be stored at: {}", file_path);
    }
    Err(e) => {
        // Handle the error appropriately
        eprintln!("Failed to refresh cache: {}", e);
    }
}
```

## Version History
* 0.1.0
    * Initial Release
* 0.1.1
    * Added invalidate cache and refresh cache functions
* 0.1.2
    * Update dependencies   
* 0.1.3
    * Added async functions for all the cache sync functions
* 0.1.4
    * Validate URL in advance for better error messaging    
* 0.1.5
    * Added 10 seconds timeout to file download and default user agent
* 0.1.6
    * Added functions supporting a file name/id as cache id        

## License
GPL-3.0-only
