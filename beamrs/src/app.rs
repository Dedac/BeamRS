use leptos::*;

use crate::domain::{Frequency, Ray};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActivePage {
    Home,
    Frequency,
    Settings,
    User,
}

pub fn render_page(
    title: &str,
    active: ActivePage,
    frequencies: Vec<Frequency>,
    content: String,
) -> String {
    let title = title.to_string();
    let document_title = format!("{title} | BeamRS");
    let html = leptos::ssr::render_to_string(move || {
        let home_class = nav_class(active == ActivePage::Home);
        let settings_class = nav_class(active == ActivePage::Settings);
        view! {
            <html lang="en">
                <head>
                    <meta charset="utf-8"/>
                    <meta name="viewport" content="width=device-width, initial-scale=1"/>
                    <meta name="description" content="BeamRS, a pun-powered social feed built with Rust"/>
                    <title>{document_title}</title>
                    <link rel="stylesheet" href="/static/style.css"/>
                    <script src="/static/app.js" defer></script>
                </head>
                <body data-page=page_name(active)>
                    <a class="skip-link" href="#main-content">"Skip to content"</a>
                    <header class="site-header">
                        <a class="brand" href="/" aria-label="BeamRS home">
                            <span class="brand-mark" aria-hidden="true">"◭"</span>
                            <span>"BEAM"</span>
                            <small>"RS"</small>
                        </a>
                        <p class="tagline">"A social network with a focused wavelength."</p>
                        <nav class="top-nav" aria-label="Primary navigation">
                            <a class=home_class href="/">"Home"</a>
                            <a class=settings_class href="/settings">"Settings"</a>
                            <a id="my-profile-link" href="/user/Anon1">"See Myself"</a>
                        </nav>
                    </header>
                    <div class="app-layout">
                        <aside class="sidebar" aria-label="Frequency navigation">
                            <div class="sidebar-heading">
                                <h2>"Frequencies"</h2>
                                <span class="live-dot" aria-hidden="true"></span>
                            </div>
                            <nav class="frequency-list">
                                {frequencies
                                    .into_iter()
                                    .map(|frequency| {
                                        let href = format!("/frequency/{}", frequency.id);
                                        view! {
                                            <a href=href>
                                                <span aria-hidden="true">"#"</span>
                                                {frequency.name}
                                            </a>
                                        }
                                    })
                                    .collect_view()}
                            </nav>
                            <form id="frequency-form" class="compact-form">
                                <label for="frequency-name">"New frequency"</label>
                                <div class="input-row">
                                    <input
                                        id="frequency-name"
                                        name="name"
                                        maxlength="60"
                                        placeholder="rustaceans"
                                        required
                                    />
                                    <button type="submit" aria-label="Create frequency">"+"</button>
                                </div>
                                <p class="form-status" role="status"></p>
                            </form>
                            <div class="identity-card">
                                <span>"Transmitting as"</span>
                                <strong id="current-username">"Connecting…"</strong>
                            </div>
                        </aside>
                        <main id="main-content" class="main-content">
                            <div class="content-frame" inner_html=content></div>
                        </main>
                    </div>
                    <div id="toast" class="toast" role="status" aria-live="polite"></div>
                </body>
            </html>
        }
    })
    .to_string();
    format!("<!doctype html>{html}")
}

fn nav_class(active: bool) -> &'static str {
    if active {
        "active"
    } else {
        ""
    }
}

fn page_name(active: ActivePage) -> &'static str {
    match active {
        ActivePage::Home => "home",
        ActivePage::Frequency => "frequency",
        ActivePage::Settings => "settings",
        ActivePage::User => "user",
    }
}

pub fn render_ray_cards(rays: &[Ray], empty_message: &str) -> String {
    if rays.is_empty() {
        return format!(
            "<div class=\"empty-state\"><span aria-hidden=\"true\">◌</span><p>{}</p></div>",
            escape_html(empty_message)
        );
    }

    rays.iter()
        .map(|ray| {
            let author = if ray.user_name.is_empty() {
                "Unknown"
            } else {
                &ray.user_name
            };
            let author_path = urlencoding::encode(author);
            let prism_users = ray.users_prismed.join(",");
            let prism_label = if ray.prism_count == 1 {
                "1 prism".to_string()
            } else {
                format!("{} prisms", ray.prism_count)
            };
            format!(
                r#"<article class="ray-card" data-ray-id="{id}">
                    <div class="ray-accent" aria-hidden="true"></div>
                    <div class="ray-body">
                        <div class="ray-meta">
                            <a href="/user/{author_path}">@{author}</a>
                            <span>Ray #{id}</span>
                        </div>
                        <p>{text}</p>
                        <button class="prism-button" type="button" data-ray-id="{id}" data-prism-users="{prism_users}" aria-pressed="false">
                            <span class="prism-icon" aria-hidden="true">◇</span>
                            <span class="prism-count">{prism_label}</span>
                        </button>
                    </div>
                </article>"#,
                id = ray.id,
                author_path = author_path,
                author = escape_html(author),
                text = escape_html(&ray.text),
                prism_users = escape_attribute(&prism_users),
                prism_label = prism_label,
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_attribute(input: &str) -> String {
    escape_html(input).replace('`', "&#96;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_cards_escape_untrusted_content() {
        let html = render_ray_cards(
            &[Ray {
                id: 1,
                frequency_id: 1,
                text: "<script>alert(1)</script>".to_string(),
                user_id: Some(1),
                user_name: "demo_user".to_string(),
                prism_count: 0,
                users_prismed: vec![],
            }],
            "empty",
        );

        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }
}
