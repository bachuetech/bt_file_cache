# Project Title
BT Cache

## Description
A caching mechanism for downloading and storing files from URLs. 
It generates SHA3-512 hashes of URLs to create unique file names and manages local storage of cached files.

## Usage
```
let cache = BTCache::new(Some("myapp"))?;
let base64_data = cache.get_file_data_base64("https://bachuetech.biz/fake_image.png")?;
```

## Version History
* 0.1.0
    * Initial Release   

## License
GPL-3.0-only
