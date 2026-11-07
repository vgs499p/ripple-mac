use ripple_core::{parse_page, trusted_url, Route, ORIGIN};
use url::Url;

fn base() -> Url {
    Url::parse(ORIGIN).unwrap()
}

// A compact fixture matching the inspected live DOM, with synthetic content.
const TIMELINE: &str = r##"
<a class="brand" href="/"><img alt="Ripple"></a>
<form method="post" action="/login"><input name="username"><input type="password" name="password"><input type="hidden" name="csrf" value="test-token"></form>
<div class="userpost" id="post-42"><div><img class="postpfp" src="/content/pfp/7.jpg"></div><div>
<span class="postheader"><a href="~alice"><b class="displayname">Alice &amp; Co</b></a><img class="verified"><span class="username">@alice</span><span class="timeago">just now</span><span class="gtr"><a href="/status/41">↪ @bob</a></span>
<div class="hover-card"><p class="hover-card-bio">THIS IS NOT THE POST</p></div></span>
<p>Hello &lt;Rust&gt;!<br>Second line <a href="/results?q=%23hello">#hello</a><img alt="🙂" class="emoji"><script>doNotExecute()</script></p>
<img class="attachment clickable-image" src="/content/attachments/test.jpg">
<img class="attachment" src="https://outside.example/tracker.jpg">
<div class="interactions"><a><span class="count">3</span></a><a><span class="count">2</span></a><a><span class="count">1</span></a></div>
</div></div>
<div class="userpost fwtlist"><span class="username">@not-a-post</span></div>
<a href="/?pageno=2">Older →</a><a href="/results?q=%23world">#world</a>
"##;

#[test]
fn timeline_retains_post_content_without_hover_bio_or_script() {
    let page = parse_page(TIMELINE, &base()).unwrap();
    assert_eq!(page.posts.len(), 1);
    let p = &page.posts[0];
    assert_eq!(p.id, 42);
    assert_eq!(p.author, "Alice & Co");
    assert_eq!(p.handle, "@alice");
    assert_eq!(p.text, "Hello <Rust>!\nSecond line #hello🙂");
    assert_eq!(
        p.images,
        ["https://ripple.tel/content/attachments/test.jpg"]
    );
    assert_eq!(
        p.avatar.as_deref(),
        Some("https://ripple.tel/content/pfp/7.jpg")
    );
    assert!(p.verified);
    assert_eq!(
        (&p.likes, &p.replies, &p.reposts),
        (&"3".into(), &"2".into(), &"1".into())
    );
    assert_eq!(
        p.reply_to.as_ref().unwrap().url,
        "https://ripple.tel/status/41"
    );
    assert!(page.older);
    assert_eq!(page.trends, ["#world"]);
    assert!(page.composer.is_none());
    let form = page.login_form.unwrap();
    assert_eq!(form.action, "https://ripple.tel/login");
    assert_eq!(form.hidden, [("csrf".into(), "test-token".into())]);
}

#[test]
fn composer_is_discovered_and_preserves_hidden_and_submit_fields() {
    let html = r#"<a class="brand" href="/"></a><form id="npform" method="post" action="/new-post" enctype="multipart/form-data"><input type="hidden" name="token" value="abc"><textarea name="postcontents"></textarea><input type="submit" name="submit" value="Post"></form>"#;
    let page = parse_page(html, &base()).unwrap();
    let composer = page.composer.unwrap();
    assert_eq!(composer.field, "postcontents");
    assert_eq!(composer.form.action, "https://ripple.tel/new-post");
    assert!(composer.form.multipart);
    assert_eq!(composer.form.submit, Some(("submit".into(), "Post".into())));
    assert_eq!(composer.form.hidden, [("token".into(), "abc".into())]);
}

#[test]
fn quote_form_retains_the_live_target_and_distinct_fields() {
    let html = r#"<a class="brand" href="/"></a><form id="quoteForm" method="post" action="/quote" enctype="multipart/form-data"><input type="hidden" name="qid" id="modalqid"><textarea name="postcontentsQ"></textarea><input type="file" name="qimages[]" multiple><button type="submit">Post</button></form>"#;
    let quote = parse_page(html, &base()).unwrap().quote_form.unwrap();
    assert_eq!(quote.form.action, "https://ripple.tel/quote");
    assert_eq!(quote.field, "postcontentsQ");
    assert!(quote.form.multipart);
    assert_eq!(quote.file_field.as_deref(), Some("qimages[]"));
    assert!(quote.form.hidden.contains(&("qid".into(), String::new())));
}

#[test]
fn forms_cannot_send_credentials_or_posts_off_origin() {
    let html = r#"<a class="brand" href="/"></a><form id="npform" method="post" action="https://outside.example/post"><textarea name="postcontents"></textarea></form><form method="post" action="//outside.example/login"><input name="username"><input type="password"></form>"#;
    let page = parse_page(html, &base()).unwrap();
    assert!(page.composer.is_none());
    assert!(page.login_form.is_none());
    for bad in [
        "http://ripple.tel/",
        "https://ripple.tel.evil.example/",
        "https://ripple.tel:444/",
        "https://secret@ripple.tel/",
    ] {
        assert!(!trusted_url(&Url::parse(bad).unwrap()));
    }
}

#[test]
fn routes_encode_search_and_reject_profile_path_injection() {
    let url = Route::Search("#rust & mac".into()).url(2).unwrap();
    assert_eq!(
        url.query_pairs().collect::<Vec<_>>(),
        [
            ("q".into(), "#rust & mac".into()),
            ("pageno".into(), "2".into())
        ]
    );
    assert_eq!(
        Route::Profile("@alice".into()).url(1).unwrap().path(),
        "/alice"
    );
    for name in [
        ".",
        "..",
        "",
        "../logout",
        "alice?logout=1",
        "//elsewhere",
        "alice/bob",
    ] {
        assert!(Route::Profile(name.into()).url(1).is_err());
    }
}

#[test]
fn challenges_are_errors_and_branded_empty_pages_are_valid() {
    assert!(parse_page("<title>Just a moment…</title>", &base()).is_err());
    let empty = parse_page(r#"<a class="brand" href="/"></a>"#, &base()).unwrap();
    assert!(empty.posts.is_empty());
    assert!(!empty.older);
}

#[test]
fn profile_lists_are_not_mistaken_for_posts() {
    let html = format!(
        r##"{TIMELINE}<div class="page-header"><h1>Alice <small><div id="usernamecontainer">@alice</div></small></h1></div><div class="biobox"><p>About Alice</p></div><ul class="tabs"><a href="#posts">Posts <b>5</b></a><a href="#followers">Followers <b>8</b></a></ul><div class="userpost fwtlist"><span class="postheader"><span class="username">@bob</span></span></div>"##
    );
    let page = parse_page(&html, &base()).unwrap();
    assert_eq!(page.posts.len(), 1);
    let profile = page.profile.unwrap();
    assert_eq!(profile.name, "Alice");
    assert_eq!(profile.handle, "@alice");
    assert_eq!(profile.bio, "About Alice");
    assert_eq!(profile.stats, ["Posts 5", "Followers 8"]);
}

#[test]
fn wallpaper_banner_and_follow_lists_are_parsed_without_executing_css() {
    let html = r##"<a class="brand" href="/"></a><style>body { background: url('/content/background/7.jpg') no-repeat center fixed; }</style><div class="page-header"><h1>Alice <small><div id="usernamecontainer">@alice</div></small></h1></div><img class="banner" src="/content/banner/7.jpg"><div id="followers"><div class="userpost fwtlist"><img class="postpfp" src="/content/pfp/8.jpg"><span class="displayname">Bob</span><span class="username">@bob</span></div></div><div id="following"><div class="userpost fwtlist"><span class="displayname">Carol</span><span class="username">@carol</span></div></div>"##;
    let page = parse_page(html, &base()).unwrap();
    assert_eq!(
        page.background.as_deref(),
        Some("https://ripple.tel/content/background/7.jpg")
    );
    let profile = page.profile.unwrap();
    assert_eq!(
        profile.banner.as_deref(),
        Some("https://ripple.tel/content/banner/7.jpg")
    );
    assert_eq!(profile.followers[0].handle, "@bob");
    assert_eq!(profile.following[0].handle, "@carol");
    assert!(page.posts.is_empty());
    let evil = html.replace(
        "'/content/background/7.jpg'",
        "'https://outside.example/content/background/tracker.jpg'",
    );
    assert!(parse_page(&evil, &base()).unwrap().background.is_none());
}

#[test]
fn authenticated_navigation_is_scoped_to_real_navigation_and_same_origin() {
    let html = r#"<a class="brand" href="/"></a><div class="topbar"><ul class="nav"><a href="/notifications">Notifications</a><a href="/settings/profile">Profile settings</a><a href="/logout">Logout</a><a href="https://outside.example/settings">Settings</a><a href="/settings?delete=1">Delete</a></ul></div><div class="userpost"><a href="/settings/forged">Settings</a></div>"#;
    let page = parse_page(html, &base()).unwrap();
    assert_eq!(page.navigation.len(), 2);
    assert_eq!(page.navigation[0].url, "https://ripple.tel/notifications");
    assert_eq!(
        page.navigation[1].url,
        "https://ripple.tel/settings/profile"
    );
}

#[tokio::test]
async fn client_rejects_undiscovered_account_routes_before_any_network_request() {
    let client = ripple_core::Client::new().unwrap();
    let result = client
        .load(&Route::Page("https://ripple.tel/settings".into()), 1)
        .await;
    assert!(matches!(result, Err(ripple_core::Error::InvalidAddress)));
}

#[test]
fn notification_items_keep_native_text_and_links() {
    let html = r#"<a class="brand" href="/"></a><main class="notifications"><div class="notification"><a href="/~alice">Alice</a> replied to <a href="/status/42">your post</a>.</div></main>"#;
    let page = parse_page(
        html,
        &Url::parse("https://ripple.tel/notifications").unwrap(),
    )
    .unwrap();
    assert_eq!(page.notifications.len(), 1);
    assert!(page.notifications[0].text.contains("replied to"));
    assert_eq!(
        page.notifications[0].links[1].url,
        "https://ripple.tel/status/42"
    );
}

#[test]
fn real_timeline_tabs_map_following_and_mentions() {
    let html = r#"<a class="brand" href="/"></a><ul class="tabs"><a href="/?tlmode=following">Following</a><a href="/?tlmode=mentions">Notifications</a></ul>"#;
    let page = parse_page(html, &base()).unwrap();
    assert_eq!(page.navigation.len(), 2);
    assert_eq!(
        Route::from_navigation(&page.navigation[0].url),
        Route::Following
    );
    assert_eq!(
        Route::from_navigation(&page.navigation[1].url),
        Route::Notifications
    );
    assert_eq!(
        Route::Notifications.url(2).unwrap().as_str(),
        "https://ripple.tel/?tlmode=mentions&pageno=2"
    );
}

#[test]
fn directory_cards_and_arrow_pagination_match_live_users_page() {
    let html = r#"<a class="brand" href="/"></a><div class="page-header"><h1>Users <small>235 registered, 32 seen recently</small></h1></div><div class="urow"><div class="thumbnail"><img src="/content/pfp/7.jpg"><div class="caption"><h3><a href="/~alice">Alice<img class="verified"></a></h3></div></div></div><ul class="pagination"><li class="next"><a href="/users?pageno=2">→</a></li></ul>"#;
    let page = parse_page(html, &Route::Users.url(1).unwrap()).unwrap();
    assert_eq!(page.people.len(), 1);
    assert_eq!(page.people[0].handle, "@alice");
    assert_eq!(page.people[0].name, "Alice");
    assert!(page.people[0].verified);
    assert!(page.heading.contains("235 registered"));
    assert!(page.older);
    assert!(page.posts.is_empty());
    let last = html
        .replace("class=\"next\"", "class=\"next disabled\"")
        .replace("href=\"/users?pageno=2\"", "href=\"#\"");
    assert!(
        !parse_page(&last, &Route::Users.url(18).unwrap())
            .unwrap()
            .older
    );
}

#[test]
fn profile_php_like_links_are_retained() {
    let html = TIMELINE.replace(
        "<a><span class=\"count\">3",
        "<a href=\"like.php?v=42\"><span class=\"count\">3",
    );
    let page = parse_page(&html, &Route::Profile("alice".into()).url(1).unwrap()).unwrap();
    assert_eq!(
        page.posts[0].like_action.as_deref(),
        Some("https://ripple.tel/like.php?v=42")
    );
}

#[test]
fn account_credentials_form_is_not_a_login_challenge() {
    let html = r#"<a class="brand" href="/"></a><form method="post" action="/account/save"><input name="username"><input type="password" name="password"></form>"#;
    assert!(
        parse_page(html, &Url::parse("https://ripple.tel/account").unwrap())
            .unwrap()
            .login_form
            .is_none()
    );
}

#[test]
fn source_footer_belongs_to_the_requested_post_not_its_replies() {
    let html = r#"<a class="brand" href="/"></a><div class="userpost" id="post-42"><span class="postheader"><b class="displayname">Alice</b><span class="username">@alice</span></span><p>Hello</p></div><p class="text-muted">Today <span>via Ripple for iOS (legacy)</span></p><div class="userpost" id="post-43"><span class="postheader"><b class="displayname">Bob</b><span class="username">@bob</span></span><p>Reply</p></div>"#;
    let page = parse_page(html, &url::Url::parse("https://ripple.tel/status/42").unwrap()).unwrap();
    assert_eq!(page.posts[0].source.as_deref(), Some("Ripple for iOS (legacy)"));
    assert_eq!(page.posts[0].text, "Hello");
    assert_eq!(page.posts[1].source, None);
    let timeline = parse_page(html, &base()).unwrap();
    assert!(timeline.posts.iter().all(|p| p.source.is_none()));
}
