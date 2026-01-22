use skp_cache::prelude::*;

#[derive(Debug, CacheKey)]
#[cache_key(namespace = "users")]
struct UserKey {
    tenant_id: u64,
    user_id: u64,
}

#[derive(Debug, CacheKey)]
#[cache_key(separator = "/")]
struct PathKey {
    folder: String,
    file: String,
    #[cache_key(skip)]
    _metadata: String,
}

#[derive(Debug, CacheKey)]
struct EmptyKey;

fn main() {
    let key = UserKey {
        tenant_id: 100,
        user_id: 456,
    };

    println!("UserKey: {}", key.cache_key());
    println!("Full Key: {}", key.full_key());
    assert_eq!(key.cache_key(), "100:456");
    assert_eq!(key.full_key(), "users:100:456");

    let path = PathKey {
        folder: "docs".to_string(),
        file: "report.pdf".to_string(),
        _metadata: "hidden".to_string(),
    };

    println!("PathKey: {}", path.cache_key());
    assert_eq!(path.cache_key(), "docs/report.pdf");

    let empty = EmptyKey;
    println!("EmptyKey: '{}'", empty.cache_key());
    assert_eq!(empty.cache_key(), "");
    
    println!("\n✅ All keys generated correctly via #[derive(CacheKey)]");
}
