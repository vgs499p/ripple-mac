//! Read-only live smoke test. Does not sign in or perform social actions.
use ripple_core::{Client, Route};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("User-Agent: {}", ripple_core::user_agent());
    let client = Client::new()?;
    let home = client.load(&Route::Public, 1).await?;
    if let Some(quote) = &home.quote_form {
        assert_eq!(quote.form.action, "https://ripple.tel/quote");
        assert_eq!(quote.field, "postcontentsQ");
        assert!(quote.form.hidden.iter().any(|(name, _)| name == "qid"));
        println!("PASS quote form discovery (read-only; no quote published)");
    }
    assert!(!home.posts.is_empty(), "Public timeline unexpectedly empty");
    println!(
        "PASS public timeline: {} posts, {} trends, older={}, login_form={}",
        home.posts.len(),
        home.trends.len(),
        home.older,
        home.login_form.is_some()
    );
    let id = home.posts[0].id;
    let handle = home.posts[0].handle.trim_start_matches('@').to_string();
    if let Some(avatar) = &home.posts[0].avatar {
        let bytes = client.image(avatar).await?;
        assert!(!bytes.is_empty());
        println!("PASS avatar download: {} bytes", bytes.len());
    }
    let thread = client.load(&Route::Thread(id), 1).await?;
    assert!(thread.posts.iter().any(|p| p.id == id));
    println!("PASS thread: {} posts", thread.posts.len());
    let profile = client.load(&Route::Profile(handle), 1).await?;
    assert!(profile.profile.is_some());
    assert!(!profile.posts.is_empty());
    println!(
        "PASS profile: {} posts, {} statistics",
        profile.posts.len(),
        profile.profile.unwrap().stats.len()
    );
    if home.older {
        let older = client.load(&Route::Public, 2).await?;
        assert!(!older.posts.is_empty());
        assert_ne!(home.posts[0].id, older.posts[0].id);
        println!("PASS pagination: {} older posts", older.posts.len());
    }
    let search = client.load(&Route::Search("#wow".into()), 1).await?;
    println!("PASS search: {} results", search.posts.len());
    let users = client.load(&Route::Users, 1).await?;
    assert!(!users.people.is_empty());
    assert!(users.posts.is_empty());
    println!(
        "PASS users directory: {} users, older={}",
        users.people.len(),
        users.older
    );
    if users.older {
        let next = client.load(&Route::Users, 2).await?;
        assert!(!next.people.is_empty());
        assert_ne!(users.people[0].handle, next.people[0].handle);
        println!("PASS directory pagination: {} users", next.people.len());
    }
    Ok(())
}
