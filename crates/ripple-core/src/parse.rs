// SPDX-License-Identifier: GPL-2.0-or-later
use crate::{trusted_url, Error};
use scraper::{ElementRef, Html, Node, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: u64,
    pub author: String,
    pub handle: String,
    pub avatar: Option<String>,
    pub verified: bool,
    pub time: String,
    #[serde(default)]
    pub source: Option<String>,
    pub text: String,
    pub images: Vec<String>,
    pub links: Vec<Link>,
    pub reply_to: Option<Link>,
    pub likes: String,
    pub replies: String,
    pub reposts: String,
    pub like_action: Option<String>,
    pub repost_action: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub handle: String,
    pub bio: String,
    pub avatar: Option<String>,
    pub stats: Vec<String>,
    pub banner: Option<String>,
    pub followers: Vec<Person>,
    pub following: Vec<Person>,
    pub follow_action: Option<String>,
    pub follow_label: String,
    #[serde(default)]
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub handle: String,
    pub avatar: Option<String>,
    pub followers: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    pub title: String,
    pub byline: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct WebForm {
    pub action: String,
    pub hidden: Vec<(String, String)>,
    pub submit: Option<(String, String)>,
    pub multipart: bool,
}

#[derive(Debug, Clone)]
pub struct Composer {
    pub form: WebForm,
    pub field: String,
    pub file_field: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub posts: Vec<Post>,
    pub older: bool,
    pub trends: Vec<String>,
    pub profile: Option<Profile>,
    pub composer: Option<Composer>,
    pub login_form: Option<WebForm>,
    pub account: Option<String>,
    pub people: Vec<Person>,
    pub heading: String,
    pub announcement: Option<Announcement>,
    pub background: Option<String>,
    pub quote_form: Option<Composer>,
    pub navigation: Vec<Link>,
    pub editable_forms: Vec<crate::EditableForm>,
    pub notifications: Vec<crate::Notification>,
    pub document_text: String,
}

pub(crate) fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("constant CSS selector")
}
fn first_text(root: ElementRef<'_>, selector: &str) -> String {
    root.select(&sel(selector))
        .next()
        .map(readable)
        .unwrap_or_default()
}
fn has_class(e: ElementRef<'_>, class: &str) -> bool {
    e.value().classes().any(|c| c == class)
}
pub(crate) fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn readable(element: ElementRef<'_>) -> String {
    let mut raw = String::new();
    for node in element.descendants() {
        if node
            .ancestors()
            .filter_map(ElementRef::wrap)
            .any(|e| matches!(e.value().name(), "script" | "style"))
        {
            continue;
        }
        match node.value() {
            Node::Text(text) => raw.push_str(&text.text),
            Node::Element(e) if e.name() == "br" => raw.push('\n'),
            Node::Element(e) if e.name() == "img" => {
                if let Some(alt) = e.attr("alt") {
                    raw.push_str(alt);
                }
            }
            _ => (),
        }
    }
    raw.lines()
        .map(normalize)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn address(base: &Url, href: &str, internal: bool) -> Option<String> {
    let u = base.join(href).ok()?;
    if !matches!(u.scheme(), "https" | "http") || !u.username().is_empty() || u.password().is_some()
    {
        return None;
    }
    if internal && !trusted_url(&u) {
        return None;
    }
    Some(u.to_string())
}
fn image_url(element: ElementRef<'_>, selector: &str, base: &Url) -> Option<String> {
    element
        .select(&sel(selector))
        .next()?
        .value()
        .attr("src")
        .and_then(|s| address(base, s, true))
}

pub(crate) fn web_form(element: ElementRef<'_>, base: &Url) -> Option<WebForm> {
    if !element
        .value()
        .attr("method")
        .unwrap_or("get")
        .eq_ignore_ascii_case("post")
    {
        return None;
    }
    let action = address(base, element.value().attr("action").unwrap_or(""), true)?;
    let hidden = element
        .select(&sel("input[type=hidden][name]"))
        .map(|e| {
            (
                e.value().attr("name").unwrap().to_string(),
                e.value().attr("value").unwrap_or("").to_string(),
            )
        })
        .collect();
    let submit = element
        .select(&sel("button[type=submit][name], input[type=submit][name]"))
        .next()
        .map(|e| {
            (
                e.value().attr("name").unwrap().to_string(),
                e.value().attr("value").unwrap_or("").to_string(),
            )
        });
    Some(WebForm {
        action,
        hidden,
        submit,
        multipart: element.value().attr("enctype") == Some("multipart/form-data"),
    })
}

pub fn parse_page(html: &str, base: &Url) -> Result<Page, Error> {
    if !trusted_url(base) {
        return Err(Error::InvalidAddress);
    }
    let doc = Html::parse_document(html);
    let root = doc.root_element();
    if root
        .select(&sel(
            "a.brand[href], img[alt=Ripple], .userpost[id^=post-], #npform",
        ))
        .next()
        .is_none()
    {
        return Err(Error::UnexpectedPage);
    }
    let mut ids = HashSet::new();
    let mut posts = Vec::new();
    for element in root.select(&sel(".userpost[id^=post-]")) {
        let Some(id) = element
            .value()
            .attr("id")
            .and_then(|s| s.strip_prefix("post-"))
            .and_then(|s| s.parse::<u64>().ok())
        else {
            continue;
        };
        if !ids.insert(id) {
            continue;
        }
        let handle = first_text(element, ".postheader .username");
        if handle.is_empty() {
            continue;
        }
        let body: Vec<_> = element
            .select(&sel("p"))
            .filter(|p| {
                !p.ancestors()
                    .filter_map(ElementRef::wrap)
                    .any(|e| has_class(e, "hover-card"))
            })
            .collect();
        let text = body
            .iter()
            .map(|p| readable(*p))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        let mut links = Vec::new();
        let mut link_urls = HashSet::new();
        for paragraph in body {
            for a in paragraph.select(&sel("a[href]")) {
                if let Some(url) = a.value().attr("href").and_then(|h| address(base, h, false)) {
                    if link_urls.insert(url.clone()) {
                        links.push(Link {
                            label: readable(a),
                            url,
                        });
                    }
                }
            }
        }
        let mut image_urls = HashSet::new();
        let images = element
            .select(&sel("img.attachment, .carousel img, img.clickable-image"))
            .filter_map(|i| {
                let url = address(base, i.value().attr("src")?, true)?;
                image_urls.insert(url.clone()).then_some(url)
            })
            .take(5)
            .collect();
        let counts: Vec<_> = element
            .select(&sel(".interactions .count"))
            .map(readable)
            .collect();
        let reply_to = element.select(&sel(".gtr a[href]")).next().and_then(|a| {
            Some(Link {
                label: readable(a),
                url: address(base, a.value().attr("href")?, true)?,
            })
        });
        posts.push(Post {
            id,
            author: first_text(element, ".postheader .displayname"),
            handle,
            avatar: image_url(element, "img.postpfp", base),
            verified: element
                .select(&sel(".postheader > img.verified"))
                .next()
                .is_some(),
            time: first_text(element, ".timeago"),
            source: None,
            text,
            images,
            links,
            reply_to,
            likes: counts.first().cloned().unwrap_or_else(|| "0".into()),
            replies: counts.get(1).cloned().unwrap_or_else(|| "0".into()),
            reposts: counts.get(2).cloned().unwrap_or_else(|| "0".into()),
            like_action: action_url(element, ".interactions a[href]", "like", base),
            repost_action: action_url(element, ".interactions a[href]", "repost", base),
        });
    }
    // The website exposes immutable client attribution on the post detail page.
    // Its footer belongs to the requested post, not to the replies below it.
    if let Some(id) = base.path().rsplit_once("/status/").and_then(|(_, id)| id.parse::<u64>().ok()) {
        if let Some(source) = root.select(&sel("p.text-muted > span"))
            .map(readable).find_map(|s| s.strip_prefix("via Ripple ").map(|tail| format!("Ripple {tail}"))) {
            if let Some(post) = posts.iter_mut().find(|post| post.id == id) {
                post.source = Some(source);
            }
        }
    }
    let older = root.select(&sel("a[href]")).any(|a| {
        let label = readable(a).to_lowercase();
        (label.contains("older")
            || label.contains("next")
            || a.parent()
                .and_then(ElementRef::wrap)
                .is_some_and(|p| has_class(p, "next") && !has_class(p, "disabled")))
            && a.value()
                .attr("href")
                .and_then(|h| base.join(h).ok())
                .is_some_and(|u| {
                    trusted_url(&u)
                        && u.query_pairs()
                            .any(|(k, v)| k == "pageno" && v.parse::<u32>().is_ok())
                })
    });
    let mut trend_set = HashSet::new();
    let trends = root
        .select(&sel("a[href*='/results'], a[href^='results']"))
        .filter_map(|a| {
            if a.ancestors()
                .filter_map(ElementRef::wrap)
                .any(|e| has_class(e, "userpost"))
            {
                return None;
            }
            let label = readable(a);
            (label.starts_with('#') && trend_set.insert(label.clone())).then_some(label)
        })
        .take(12)
        .collect();
    let profile = root.select(&sel("#usernamecontainer")).next().map(|username| {
        let handle = readable(username);
        let full_name = first_text(root, ".page-header h1");
        let stats = root.select(&sel(".tabs a[href='#posts'], .tabs a[href='#followers'], .tabs a[href='#following']")).map(readable).collect();
        Profile { name: full_name.replace(&handle, "").trim().to_string(), handle,
            bio: root.select(&sel(".biobox")).next().map(readable).unwrap_or_default(),
            avatar: image_url(root, "#rechannel-pfp", base), stats,
            banner: image_url(root, "img.banner", base),
            followers: people_in(root, "#followers .userpost", base),
            following: people_in(root, "#following .userpost", base),
            follow_label: first_text(root,".subbtnrechannel .sub-button-text"),
            follow_action: root.select(&sel(".subbtnrechannel a[href]")).next().and_then(|a| {
                address(base, a.value().attr("href")?, true)
            }),
            details: root.select(&sel("#channel > div")).filter(|e| e.select(&sel("b")).next().is_some()).map(readable).filter(|s| !s.is_empty()).take(20).collect(),
        }
    });
    let login_form = root
        .select(&sel("form"))
        .find(|f| {
            f.value()
                .attr("action")
                .and_then(|a| base.join(a).ok())
                .is_some_and(|u| trusted_url(&u) && u.path() == "/login")
                && f.select(&sel("input[name=username]")).next().is_some()
                && f.select(&sel("input[type=password]")).next().is_some()
        })
        .and_then(|f| web_form(f, base));
    let composer = root
        .select(&sel("form#npform"))
        .next()
        .and_then(|f| compose_form(f, base));
    let quote_form = root
        .select(&sel("form#quoteForm"))
        .next()
        .and_then(|f| compose_form(f, base));
    let account = root
        .select(&sel(
            ".secondary-nav .dropdown-toggle .username, .secondary-nav .username",
        ))
        .next()
        .map(readable);
    let mut handles = HashSet::new();
    let mut people: Vec<_> = root
        .select(&sel(".span4 .userpost.ftl"))
        .filter_map(|e| {
            let handle = first_text(e, ".username");
            if handle.is_empty() || !handles.insert(handle.clone()) {
                return None;
            }
            Some(Person {
                name: first_text(e, ".displayname"),
                handle,
                avatar: image_url(e, "img.postpfp", base),
                followers: first_text(
                    e,
                    ".yt-subscription-button-subscriber-count-branded-horizontal",
                ),
                verified: e.select(&sel("img.verified")).next().is_some(),
            })
        })
        .take(15)
        .collect();
    if base.path() == "/users" {
        people = root
            .select(&sel(".urow .thumbnail"))
            .filter_map(|e| {
                let link = e.select(&sel("h3 a[href]")).next()?;
                let url = base.join(link.value().attr("href")?).ok()?;
                if !trusted_url(&url) {
                    return None;
                }
                let name = url.path().strip_prefix("/~")?;
                Some(Person {
                    name: readable(link),
                    handle: format!("@{name}"),
                    avatar: image_url(e, "img", base),
                    followers: String::new(),
                    verified: e.select(&sel("img.verified")).next().is_some(),
                })
            })
            .take(100)
            .collect();
    }
    let heading = first_text(root, ".page-header h1");
    let announcement = root.select(&sel(".span4 .mb-2")).next().map(|title| {
        let byline = title
            .children()
            .filter_map(ElementRef::wrap)
            .nth(1)
            .map(readable)
            .unwrap_or_default();
        Announcement {
            title: title
                .children()
                .filter_map(ElementRef::wrap)
                .next()
                .map(readable)
                .unwrap_or_default(),
            byline,
            body: first_text(root, ".span4 .text-body"),
        }
    });
    let background = root.select(&sel("style")).find_map(|style| {
        let css = style.text().collect::<String>();
        css.split("url(").skip(1).find_map(|part| {
            let path = part.split(')').next()?.trim().trim_matches(['\'', '"']);
            let u = base.join(path).ok()?;
            (trusted_url(&u) && u.path().starts_with("/content/background/")).then(|| u.to_string())
        })
    });
    let navigation = crate::account::navigation(root, base);
    let editable_forms = crate::account::forms(root, base);
    let notifications = crate::account::notifications(root, base);
    let document_text = root
        .select(&sel("main,.content"))
        .next()
        .map(readable)
        .unwrap_or_default()
        .chars()
        .take(16000)
        .collect();
    Ok(Page {
        posts,
        older,
        trends,
        profile,
        composer,
        login_form,
        account,
        people,
        heading,
        announcement,
        background,
        quote_form,
        navigation,
        editable_forms,
        notifications,
        document_text,
    })
}

fn compose_form(form: ElementRef<'_>, base: &Url) -> Option<Composer> {
    Some(Composer {
        field: form
            .select(&sel("textarea[name]"))
            .next()?
            .value()
            .attr("name")?
            .to_string(),
        file_field: form
            .select(&sel("input[type=file][name]"))
            .next()
            .and_then(|i| i.value().attr("name"))
            .map(str::to_string),
        form: web_form(form, base)?,
    })
}

fn action_url(root: ElementRef<'_>, selector: &str, action: &str, base: &Url) -> Option<String> {
    root.select(&sel(selector)).find_map(|a| {
        let url = base.join(a.value().attr("href")?).ok()?;
        (trusted_url(&url) && url.path().trim_matches('/').trim_end_matches(".php") == action)
            .then(|| url.to_string())
    })
}

fn people_in(root: ElementRef<'_>, selector: &str, base: &Url) -> Vec<Person> {
    root.select(&sel(selector))
        .filter_map(|e| {
            let handle = first_text(e, ".username");
            if handle.is_empty() {
                return None;
            }
            Some(Person {
                name: first_text(e, ".displayname"),
                handle,
                avatar: image_url(e, "img.postpfp", base),
                followers: first_text(
                    e,
                    ".yt-subscription-button-subscriber-count-branded-horizontal",
                ),
                verified: e.select(&sel("img.verified")).next().is_some(),
            })
        })
        .take(200)
        .collect()
}
