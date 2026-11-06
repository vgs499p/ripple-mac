// SPDX-License-Identifier: GPL-2.0-or-later
use crate::{
    parse::{address, readable, sel, web_form, WebForm},
    trusted_url, Error, Link, Upload,
};
use scraper::ElementRef;
#[cfg(test)]
use scraper::Html;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Multiline,
    Password,
    Checkbox,
    Select,
    File,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub label: String,
    pub value: String,
}
impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub label: String,
    pub value: String,
    pub kind: FieldKind,
    pub checked: bool,
    pub choices: Vec<Choice>,
    pub required: bool,
    pub max_len: Option<usize>,
}
#[derive(Debug, Clone)]
pub struct EditableForm {
    pub title: String,
    pub form: WebForm,
    pub fields: Vec<Field>,
}
#[derive(Debug, Clone)]
pub struct Notification {
    pub text: String,
    pub links: Vec<Link>,
}

pub(crate) fn navigation(root: ElementRef<'_>, base: &Url) -> Vec<Link> {
    let mut seen = std::collections::HashSet::new();
    root.select(&sel(".topbar a[href], .nav a[href], .tabs a[href]"))
        .filter_map(|a| {
            if a.ancestors()
                .filter_map(ElementRef::wrap)
                .any(|e| e.value().classes().any(|c| c == "userpost"))
            {
                return None;
            }
            let label = readable(a);
            let href = a.value().attr("href")?;
            let url = base.join(href).ok()?;
            let path = url.path().to_lowercase();
            if !trusted_url(&url)
                || href.starts_with('#')
                || ["delete", "logout", "remove", "like", "repost"]
                    .iter()
                    .any(|word| path.contains(word))
            {
                return None;
            }
            if url.query_pairs().any(|(k, _)| {
                ["delete", "action", "logout", "remove"]
                    .iter()
                    .any(|word| k.contains(word))
            }) {
                return None;
            }
            let relevant = [
                "notif",
                "setting",
                "account",
                "editprofile",
                "edit_profile",
                "following",
            ]
            .iter()
            .any(|word| path.contains(word) || label.to_lowercase().contains(word));
            (relevant && seen.insert(url.to_string())).then(|| Link {
                label,
                url: url.to_string(),
            })
        })
        .collect()
}

pub(crate) fn forms(root: ElementRef<'_>, base: &Url) -> Vec<EditableForm> {
    root.select(&sel("form"))
        .filter_map(|element| {
            if matches!(element.value().attr("id"), Some("npform" | "quoteForm"))
                || element
                    .select(&sel(
                        "input[name=username][type=text], input[name=username]:not([type])",
                    ))
                    .next()
                    .is_some()
                    && element
                        .select(&sel("input[type=password]"))
                        .next()
                        .is_some()
            {
                return None;
            }
            let form = web_form(element, base)?;
            if ["delete", "logout", "register", "quote"]
                .iter()
                .any(|word| form.action.to_lowercase().contains(word))
            {
                return None;
            }
            let mut fields = Vec::new();
            for input in element.select(&sel("input[name],textarea[name],select[name]")) {
                if input.value().attr("disabled").is_some() {
                    continue;
                }
                let name = input.value().attr("name")?.to_string();
                let tag = input.value().name();
                let ty = input.value().attr("type").unwrap_or("text").to_lowercase();
                if matches!(ty.as_str(), "hidden" | "submit" | "button" | "reset") {
                    continue;
                }
                let kind = match (tag, ty.as_str()) {
                    ("textarea", _) => FieldKind::Multiline,
                    ("select", _) => FieldKind::Select,
                    (_, "password") => FieldKind::Password,
                    (_, "checkbox") => FieldKind::Checkbox,
                    (_, "file") => FieldKind::File,
                    (_, "text" | "email" | "url" | "number" | "search" | "color") => {
                        FieldKind::Text
                    }
                    _ => return None,
                };
                let id = input.value().attr("id");
                let label = root
                    .select(&sel("label"))
                    .find(|l| id.is_some() && l.value().attr("for") == id)
                    .map(readable)
                    .or_else(|| {
                        input
                            .ancestors()
                            .filter_map(ElementRef::wrap)
                            .find(|e| e.value().name() == "label")
                            .map(readable)
                    })
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| humanize(&name));
                let choices: Vec<_> = input
                    .select(&sel("option"))
                    .map(|o| Choice {
                        label: readable(o),
                        value: o
                            .value()
                            .attr("value")
                            .map(str::to_string)
                            .unwrap_or_else(|| readable(o)),
                    })
                    .collect();
                let value = if tag == "textarea" {
                    input.text().collect()
                } else if tag == "select" {
                    input
                        .select(&sel("option[selected]"))
                        .next()
                        .map(|o| {
                            o.value()
                                .attr("value")
                                .map(str::to_string)
                                .unwrap_or_else(|| readable(o))
                        })
                        .or_else(|| choices.first().map(|c| c.value.clone()))
                        .unwrap_or_default()
                } else if kind == FieldKind::Password {
                    String::new()
                } else {
                    input
                        .value()
                        .attr("value")
                        .unwrap_or(if kind == FieldKind::Checkbox {
                            "on"
                        } else {
                            ""
                        })
                        .to_string()
                };
                fields.push(Field {
                    name,
                    label,
                    value,
                    kind,
                    checked: input.value().attr("checked").is_some(),
                    choices,
                    required: input.value().attr("required").is_some(),
                    max_len: input.value().attr("maxlength").and_then(|s| s.parse().ok()),
                });
            }
            if fields.is_empty() {
                return None;
            }
            let title = element
                .select(&sel("legend,h2,h3,h4"))
                .next()
                .map(readable)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Account settings".into());
            Some(EditableForm {
                title,
                form,
                fields,
            })
        })
        .take(12)
        .collect()
}

pub(crate) fn notifications(root: ElementRef<'_>, base: &Url) -> Vec<Notification> {
    if !base.path().to_lowercase().contains("notif")
        && !base
            .query_pairs()
            .any(|(key, value)| key == "tlmode" && value == "mentions")
    {
        return Vec::new();
    }
    root.select(&sel(".notification, .notification-item, .notifications li, .notifications .userpost, .notification-list > div")).filter_map(|e| {
        let text=readable(e);if text.is_empty(){return None;}
        let links=e.select(&sel("a[href]")).filter_map(|a|Some(Link{label:readable(a),url:address(base,a.value().attr("href")?,true)?})).collect();
        Some(Notification{text,links})
    }).take(100).collect()
}
fn humanize(name: &str) -> String {
    let s = name.replace(['_', '-'], " ").replace("[]", "");
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().to_string() + chars.as_str())
        .unwrap_or(s)
}

pub(crate) fn validate_submission(
    form: &EditableForm,
    values: &[(String, String)],
    files: &[(String, Upload)],
) -> Result<(), Error> {
    if files.len() > 5 || files.iter().map(|(_, f)| f.bytes.len()).sum::<usize>() > 20 * 1024 * 1024
    {
        return Err(Error::InvalidUpload);
    }
    for (name, value) in values {
        let field = form
            .fields
            .iter()
            .find(|f| &f.name == name && f.kind != FieldKind::File)
            .ok_or(Error::InvalidForm)?;
        if field
            .max_len
            .is_some_and(|max| value.encode_utf16().count() > max)
        {
            return Err(Error::InvalidForm);
        }
        if field.kind == FieldKind::Select && !field.choices.iter().any(|c| &c.value == value) {
            return Err(Error::InvalidForm);
        }
    }
    for (name, file) in files {
        if !form.form.multipart
            || !form
                .fields
                .iter()
                .any(|f| &f.name == name && f.kind == FieldKind::File)
        {
            return Err(Error::NoUpload);
        }
        if file.bytes.len() > 8 * 1024 * 1024
            || !matches!(
                file.mime.as_str(),
                "image/png" | "image/jpeg" | "image/gif" | "image/webp"
            )
        {
            return Err(Error::InvalidUpload);
        }
    }
    for field in &form.fields {
        if field.required
            && field.kind != FieldKind::Checkbox
            && field.kind != FieldKind::File
            && !values
                .iter()
                .any(|(name, value)| name == &field.name && !value.trim().is_empty())
        {
            return Err(Error::InvalidForm);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_preserve_controls_and_reject_undeclared_fields() {
        let doc = Html::parse_document(
            r#"<form method="post" action="/settings/save" enctype="multipart/form-data"><legend>Profile</legend><input type="hidden" name="csrf" value="fresh"><label for="display">Display name</label><input id="display" name="display_name" value="Alice" required maxlength="20"><textarea name="bio">Hello</textarea><input type="checkbox" name="private" checked><select name="theme"><option value="light">Light</option><option value="dark" selected>Dark</option></select><input type="file" name="background"></form>"#,
        );
        let forms = forms(doc.root_element(), &Url::parse(crate::ORIGIN).unwrap());
        let f = &forms[0];
        assert_eq!(f.title, "Profile");
        assert_eq!(f.fields[0].label, "Display name");
        assert_eq!(f.fields[1].kind, FieldKind::Multiline);
        assert!(f.fields[2].checked);
        assert_eq!(f.fields[3].value, "dark");
        assert_eq!(f.form.hidden[0].1, "fresh");
        assert!(validate_submission(
            f,
            &[
                ("display_name".into(), "Bob".into()),
                ("theme".into(), "dark".into())
            ],
            &[]
        )
        .is_ok());
        assert!(validate_submission(f, &[("admin".into(), "1".into())], &[]).is_err());
        assert!(validate_submission(
            f,
            &[
                ("display_name".into(), "Bob".into()),
                ("theme".into(), "invented".into())
            ],
            &[]
        )
        .is_err());
    }
}
