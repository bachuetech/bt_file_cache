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

## License
GPL-3.0-only
