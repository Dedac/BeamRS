use std::time::Duration;

use leptos::{ev, html, prelude::*, task::spawn_local};
use leptos_dom::helpers::set_timeout;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes, A},
    hooks::{use_location, use_navigate, use_params_map},
    path,
};

use crate::{
    client,
    domain::{Frequency, Ray, User, MAX_RAY_TEXT_LEN},
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum IdentityState {
    Loading,
    Ready(User),
    Error(String),
}

#[derive(Clone, Copy)]
struct ToastContext {
    state: RwSignal<Option<(String, bool)>>,
}

impl ToastContext {
    fn show(self, message: impl Into<String>, is_error: bool) {
        self.state.set(Some((message.into(), is_error)));
        let state = self.state;
        set_timeout(move || state.set(None), Duration::from_millis(3500));
    }
}

#[derive(Clone, Copy)]
struct FrequencyCreationContext {
    expanded: RwSignal<bool>,
    focus_requested: RwSignal<bool>,
}

impl FrequencyCreationContext {
    fn trigger(self) {
        self.expanded.set(true);
        self.focus_requested.set(true);
    }
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let identity = RwSignal::new(IdentityState::Loading);
    let toast = ToastContext {
        state: RwSignal::new(None),
    };
    let frequency_creation = FrequencyCreationContext {
        expanded: RwSignal::new(false),
        focus_requested: RwSignal::new(false),
    };
    provide_context(identity);
    provide_context(toast);
    provide_context(frequency_creation);

    Effect::new(move |_| {
        spawn_local(async move {
            match client::load_identity().await {
                Ok(user) => identity.set(IdentityState::Ready(user)),
                Err(error) => identity.set(IdentityState::Error(error)),
            }
        });
    });

    view! {
        <Stylesheet id="leptos" href="/pkg/beamrs.css"/>
        <Title text="BeamRS"/>
        <Meta name="description" content="BeamRS, a pun-powered social feed built with Rust"/>
        <Router>
            <a class="skip-link" href="#main-content">"Skip to content"</a>
            <SiteHeader/>
            <div class="app-layout">
                <Sidebar/>
                <main id="main-content" class="main-content">
                    <div class="content-frame">
                        <Routes fallback=|| view! { <NotFoundPage/> }.into_view()>
                            <Route path=path!("") view=HomePage/>
                            <Route path=path!("/settings") view=SettingsPage/>
                            <Route path=path!("/frequency/:frequency_id") view=FrequencyPage/>
                            <Route path=path!("/user/:username") view=UserPage/>
                        </Routes>
                    </div>
                </main>
            </div>
            <Toast/>
        </Router>
    }
}

#[component]
fn SiteHeader() -> impl IntoView {
    let location = use_location();
    let identity = expect_context::<RwSignal<IdentityState>>();
    let profile_path = move || match identity.get() {
        IdentityState::Ready(user) => format!("/user/{}", user.username),
        _ => "/settings".to_string(),
    };

    view! {
        <header class="site-header">
            <A href="/" attr:class="brand" attr:aria-label="BeamRS home">
                <span class="brand-mark" aria-hidden="true">"◭"</span>
                <span>"BEAM"</span>
                <small>"RS"</small>
            </A>
            <p class="tagline">"A social network with a focused wavelength."</p>
            <nav class="top-nav" aria-label="Primary navigation">
                <A
                    href="/"
                    attr:class=move || if location.pathname.get() == "/" { "active" } else { "" }
                >
                    "Home"
                </A>
                <A
                    href="/settings"
                    attr:class=move || {
                        if location.pathname.get() == "/settings" { "active" } else { "" }
                    }
                >
                    "Settings"
                </A>
                <A href=profile_path attr:id="my-profile-link">"See Myself"</A>
            </nav>
        </header>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    let frequencies = LocalResource::new(client::list_frequencies);
    let name = RwSignal::new(String::new());
    let pending = RwSignal::new(false);
    let status = RwSignal::new(None::<(String, bool)>);
    let navigate = use_navigate();
    let frequency_creation = expect_context::<FrequencyCreationContext>();
    let name_input = NodeRef::<html::Input>::new();

    Effect::new(move |_| {
        if frequency_creation.focus_requested.get() {
            frequency_creation.focus_requested.set(false);
            if let Some(input) = name_input.get() {
                let _ = input.focus();
                input.select();
            }
        }
    });

    let submit = move |event: ev::SubmitEvent| {
        event.prevent_default();
        if pending.get_untracked() {
            return;
        }
        pending.set(true);
        status.set(Some(("Creating frequency…".to_string(), false)));
        let requested_name = name.get_untracked();
        let navigate = navigate.clone();
        spawn_local(async move {
            match client::create_frequency(requested_name).await {
                Ok(frequency) => {
                    name.set(String::new());
                    status.set(None);
                    frequencies.refetch();
                    navigate(&format!("/frequency/{}", frequency.id), Default::default());
                }
                Err(error) => status.set(Some((error, true))),
            }
            pending.set(false);
        });
    };

    view! {
        <aside class="sidebar" aria-label="Frequency navigation">
            <div class="sidebar-heading">
                <h2>"Frequencies"</h2>
                <span class="live-dot" aria-hidden="true"></span>
            </div>
            <nav class="frequency-list">
                {move || match frequencies.get() {
                    None => view! { <span class="loading-state">"Tuning…"</span> }.into_any(),
                    Some(Err(error)) => {
                        view! { <span class="inline-error">{error}</span> }.into_any()
                    }
                    Some(Ok(items)) if items.is_empty() => {
                        view! { <span class="loading-state">"No frequencies yet."</span> }
                            .into_any()
                    }
                    Some(Ok(items)) => {
                        view! {
                            <For
                                each=move || items.clone()
                                key=|frequency| frequency.id
                                children=move |frequency: Frequency| {
                                    view! {
                                        <A href=format!("/frequency/{}", frequency.id)>
                                            <span aria-hidden="true">"#"</span>
                                            {frequency.name}
                                        </A>
                                    }
                                }
                            />
                        }
                        .into_any()
                    }
                }}
            </nav>
            <form
                id="frequency-creation-form"
                class=move || {
                    if frequency_creation.expanded.get() { "compact-form visible" } else { "compact-form" }
                }
                on:submit=submit
            >
                <label for="frequency-name">"New frequency"</label>
                <div class="input-row">
                    <input
                        id="frequency-name"
                        name="name"
                        maxlength="60"
                        placeholder="rustaceans"
                        required
                        node_ref=name_input
                        prop:value=move || name.get()
                        on:input=move |event| name.set(event_target_value(&event))
                    />
                    <button type="submit" aria-label="Create frequency" disabled=move || pending.get()>
                        "+"
                    </button>
                </div>
                <FormStatus status/>
            </form>
            <IdentityCard/>
        </aside>
    }
}

#[component]
fn IdentityCard() -> impl IntoView {
    let identity = expect_context::<RwSignal<IdentityState>>();
    view! {
        <div class="identity-card">
            <span>"Transmitting as"</span>
            <strong>
                {move || match identity.get() {
                    IdentityState::Loading => "Connecting…".to_string(),
                    IdentityState::Ready(user) => user.username,
                    IdentityState::Error(_) => "Offline".to_string(),
                }}
            </strong>
        </div>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let frequency_creation = expect_context::<FrequencyCreationContext>();
    let on_create_frequency = move |_| {
        frequency_creation.trigger();
    };

    view! {
        <Title text="Home | BeamRS"/>
        <section class="welcome-panel">
            <div class="eyebrow">"Rust-powered transmission"</div>
            <h1>"Welcome to " <span>"BeamRS"</span></h1>
            <p>"Pick a frequency, send a ray, and prism the signals that deserve a wider spectrum."</p>
            <div class="welcome-actions">
                <button
                    type="button"
                    class="primary-button"
                    aria-controls="frequency-creation-form"
                    on:click=on_create_frequency
                >
                    "Create a frequency"
                </button>
                <A href="/settings" attr:class="text-link">"Tune your identity →"</A>
            </div>
        </section>
        <section class="explainer-grid" aria-label="How BeamRS works">
            <article><strong>"01"</strong><h2>"Tune in"</h2><p>"Frequencies keep every conversation on wavelength."</p></article>
            <article><strong>"02"</strong><h2>"Send a ray"</h2><p>"Share a focused thought in 300 characters or fewer."</p></article>
            <article><strong>"03"</strong><h2>"Prism it"</h2><p>"Refract a great ray to show that it resonated."</p></article>
        </section>
    }
}

#[component]
fn FrequencyPage() -> impl IntoView {
    let params = use_params_map();
    let frequency_id = move || {
        params.with(|params| {
            params
                .get("frequency_id")
                .and_then(|value| value.parse::<i32>().ok())
        })
    };
    let page = LocalResource::new(move || {
        let frequency_id = frequency_id();
        async move {
            let frequency_id = frequency_id.ok_or_else(|| "invalid frequency id".to_string())?;
            let frequencies = client::list_frequencies().await?;
            let frequency = frequencies
                .into_iter()
                .find(|frequency| frequency.id == frequency_id)
                .ok_or_else(|| format!("frequency {frequency_id} not found"))?;
            let rays = client::list_frequency_rays(frequency_id).await?;
            Ok::<_, String>((frequency, rays))
        }
    });

    view! {
        {move || match page.get() {
            None => view! { <PageLoading message="Loading frequency…"/> }.into_any(),
            Some(Err(error)) => view! { <PageError message=error/> }.into_any(),
            Some(Ok((frequency, rays))) => {
                view! {
                    <Title text=format!("#{} | BeamRS", frequency.name)/>
                    <section class="page-heading frequency-heading">
                        <div>
                            <span class="eyebrow">{format!("Frequency {}", frequency.id)}</span>
                            <h1>"#"{frequency.name.clone()}</h1>
                        </div>
                        <p>{format!("{} rays currently traveling on this wavelength.", rays.len())}</p>
                    </section>
                    <RayComposer frequency=frequency.clone() on_created=move |_| page.refetch()/>
                    <section class="feed-section">
                        <div class="section-title">
                            <h2>"Latest rays"</h2>
                            <span>{rays.len()}</span>
                        </div>
                        <RayList
                            rays
                            empty_message="This frequency is quiet. Send the first ray."
                        />
                    </section>
                }
                .into_any()
            }
        }}
    }
}

#[component]
fn RayComposer(frequency: Frequency, on_created: impl Fn(Ray) + Clone + 'static) -> impl IntoView {
    let identity = expect_context::<RwSignal<IdentityState>>();
    let text = RwSignal::new(String::new());
    let pending = RwSignal::new(false);
    let status = RwSignal::new(None::<(String, bool)>);
    let frequency_id = frequency.id;
    let frequency_name = frequency.name;

    let submit = move |event: ev::SubmitEvent| {
        event.prevent_default();
        if pending.get_untracked() {
            return;
        }
        let user = match identity.get_untracked() {
            IdentityState::Ready(user) => user,
            IdentityState::Loading => {
                status.set(Some(("Identity is still connecting.".to_string(), true)));
                return;
            }
            IdentityState::Error(error) => {
                status.set(Some((format!("Identity unavailable: {error}"), true)));
                return;
            }
        };
        pending.set(true);
        status.set(Some(("Transmitting…".to_string(), false)));
        let ray_text = text.get_untracked();
        let on_created = on_created.clone();
        spawn_local(async move {
            match client::create_ray(frequency_id, user.id, ray_text).await {
                Ok(ray) => {
                    text.set(String::new());
                    status.set(None);
                    on_created(ray);
                }
                Err(error) => status.set(Some((error, true))),
            }
            pending.set(false);
        });
    };

    view! {
        <section class="composer-panel">
            <form on:submit=submit>
                <label for="ray-text">{format!("Send a ray to #{frequency_name}")}</label>
                <textarea
                    id="ray-text"
                    name="text"
                    maxlength=MAX_RAY_TEXT_LEN
                    rows="3"
                    placeholder="What is on your wavelength?"
                    required
                    prop:value=move || text.get()
                    on:input=move |event| text.set(event_target_value(&event))
                ></textarea>
                <div class="composer-footer">
                    <span>{move || text.get().chars().count()}"/"{MAX_RAY_TEXT_LEN}</span>
                    <button class="primary-button" type="submit" disabled=move || pending.get()>
                        "Transmit ray"
                    </button>
                </div>
                <FormStatus status/>
            </form>
        </section>
    }
}

#[component]
fn UserPage() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.with(|params| params.get("username"));
    let page = LocalResource::new(move || {
        let username = username();
        async move {
            let username = username.ok_or_else(|| "missing username".to_string())?;
            let authored = client::list_authored_rays(&username).await?;
            let prismed = client::list_prismed_rays(&username).await?;
            Ok::<_, String>((username, authored, prismed))
        }
    });

    view! {
        {move || match page.get() {
            None => view! { <PageLoading message="Loading profile…"/> }.into_any(),
            Some(Err(error)) => view! { <PageError message=error/> }.into_any(),
            Some(Ok((username, authored, prismed))) => {
                let initial = username.chars().next().unwrap_or('?');
                view! {
                    <Title text=format!("@{username} | BeamRS")/>
                    <section class="profile-header">
                        <div class="avatar" aria-hidden="true">{initial}</div>
                        <div>
                            <span class="eyebrow">"Beam profile"</span>
                            <h1>"@"{username.clone()}</h1>
                            <p>{format!(
                                "{} authored rays · {} prismed rays",
                                authored.len(),
                                prismed.len()
                            )}</p>
                        </div>
                    </section>
                    <section class="feed-section">
                        <div class="section-title">
                            <h2>"Authored rays"</h2>
                            <span>{authored.len()}</span>
                        </div>
                        <RayList
                            rays=authored
                            empty_message="No rays transmitted by this user yet."
                        />
                    </section>
                    <section class="feed-section">
                        <div class="section-title">
                            <h2>"Prismed rays"</h2>
                            <span>{prismed.len()}</span>
                        </div>
                        <RayList
                            rays=prismed
                            empty_message="No rays prismed by this user yet."
                        />
                    </section>
                }
                .into_any()
            }
        }}
    }
}

#[component]
fn SettingsPage() -> impl IntoView {
    let identity = expect_context::<RwSignal<IdentityState>>();
    let username = RwSignal::new(String::new());
    let pending = RwSignal::new(false);
    let status = RwSignal::new(None::<(String, bool)>);
    let toast = expect_context::<ToastContext>();

    Effect::new(move |_| {
        if let IdentityState::Ready(user) = identity.get() {
            username.set(user.username);
        }
    });

    let submit = move |event: ev::SubmitEvent| {
        event.prevent_default();
        if pending.get_untracked() {
            return;
        }
        let user = match identity.get_untracked() {
            IdentityState::Ready(user) => user,
            IdentityState::Loading => {
                status.set(Some(("Identity is still connecting.".to_string(), true)));
                return;
            }
            IdentityState::Error(error) => {
                status.set(Some((format!("Identity unavailable: {error}"), true)));
                return;
            }
        };
        pending.set(true);
        status.set(Some(("Retuning identity…".to_string(), false)));
        let requested_username = username.get_untracked();
        spawn_local(async move {
            match client::update_username(user.id, requested_username).await {
                Ok(updated) => match client::store_identity(&updated) {
                    Ok(()) => {
                        username.set(updated.username.clone());
                        identity.set(IdentityState::Ready(updated.clone()));
                        status.set(Some(("Identity updated.".to_string(), false)));
                        toast.show(format!("Now transmitting as {}", updated.username), false);
                    }
                    Err(error) => status.set(Some((error, true))),
                },
                Err(error) => status.set(Some((error, true))),
            }
            pending.set(false);
        });
    };

    view! {
        <Title text="Settings | BeamRS"/>
        <section class="page-heading">
            <div><span class="eyebrow">"Signal controls"</span><h1>"Settings"</h1></div>
            <p>"Your identity is stored in this browser. BeamRS has no authentication, matching the original Beam experience."</p>
        </section>
        <div class="settings-grid">
            <section class="panel">
                <h2>"Display name"</h2>
                <p>"Use letters, numbers, dashes, or underscores. Changing this name updates your existing profile."</p>
                <form class="stacked-form" on:submit=submit>
                    <label for="username-input">"Username"</label>
                    <input
                        id="username-input"
                        name="username"
                        maxlength="40"
                        autocomplete="nickname"
                        required
                        prop:value=move || username.get()
                        on:input=move |event| username.set(event_target_value(&event))
                    />
                    <button class="primary-button" type="submit" disabled=move || pending.get()>
                        "Update identity"
                    </button>
                    <FormStatus status/>
                </form>
            </section>
            <BeamCanvas/>
        </div>
    }
}

#[component]
fn BeamCanvas() -> impl IntoView {
    let canvas = NodeRef::<html::Canvas>::new();
    let _canvas_for_view = canvas;
    let passes = RwSignal::new(0_u32);

    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    Effect::new(move |_| start_beam_animation(canvas, passes));

    view! {
        <section class="panel beam-lab">
            <div>
                <span class="eyebrow">"Optics lab"</span>
                <h2>"Beam pass counter"</h2>
                <p>"A tiny Rust/WASM signal rendered directly with the browser Canvas API."</p>
            </div>
            <canvas
                node_ref=_canvas_for_view
                width="640"
                height="220"
                aria-label="Animated red beam traveling across an optics track"
            ></canvas>
            <div class="counter">
                <strong>{move || passes.get()}</strong>
                <span>"completed passes"</span>
            </div>
        </section>
    }
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn start_beam_animation(canvas: NodeRef<html::Canvas>, passes: RwSignal<u32>) {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use send_wrapper::SendWrapper;
    use wasm_bindgen::{closure::Closure, JsCast};
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

    let Some(canvas) = canvas.get() else {
        return;
    };
    let canvas: HtmlCanvasElement = (*canvas).clone().unchecked_into();
    let Ok(Some(context)) = canvas.get_context("2d") else {
        return;
    };
    let Ok(context) = context.dyn_into::<CanvasRenderingContext2d>() else {
        return;
    };
    let Some(window) = web_sys::window() else {
        return;
    };
    let reduce_motion = window
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches());
    let duration = if reduce_motion { 8000.0 } else { 2600.0 };
    let started_at = Rc::new(Cell::new(None::<f64>));
    let frame_id = Rc::new(Cell::new(None::<i32>));
    let animation = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
    let animation_ref = animation.clone();
    let animation_weak = Rc::downgrade(&animation);
    let window_ref = window.clone();
    let frame_id_ref = frame_id.clone();
    let cleanup_frame_id = SendWrapper::new(frame_id.clone());
    let cleanup_animation_ref = SendWrapper::new(animation_ref.clone());
    let cleanup_window = SendWrapper::new(window.clone());

    *animation_ref.borrow_mut() = Some(Closure::wrap(Box::new(move |now: f64| {
        let start = started_at.get().unwrap_or_else(|| {
            started_at.set(Some(now));
            now
        });
        let elapsed = now - start;
        if elapsed >= duration {
            let completed = (elapsed / duration).floor() as u32;
            passes.update(|passes| *passes += completed);
            started_at.set(Some(start + f64::from(completed) * duration));
        }

        let progress = ((now - started_at.get().unwrap_or(now)) / duration).clamp(0.0, 1.0);
        let width = f64::from(canvas.width());
        let height = f64::from(canvas.height());
        let x = -36.0 + progress * (width + 72.0);
        context.clear_rect(0.0, 0.0, width, height);
        context.set_fill_style_str("#10141d");
        context.fill_rect(0.0, 0.0, width, height);
        context.set_stroke_style_str("#303849");
        context.set_line_width(2.0);
        context.begin_path();
        context.move_to(30.0, height / 2.0);
        context.line_to(width - 30.0, height / 2.0);
        context.stroke();
        if let Ok(glow) =
            context.create_radial_gradient(x, height / 2.0, 2.0, x, height / 2.0, 55.0)
        {
            let _ = glow.add_color_stop(0.0, "rgba(255, 70, 86, 1)");
            let _ = glow.add_color_stop(0.25, "rgba(255, 38, 65, .7)");
            let _ = glow.add_color_stop(1.0, "rgba(255, 38, 65, 0)");
            context.set_fill_style_canvas_gradient(&glow);
            context.fill_rect(x - 60.0, height / 2.0 - 60.0, 120.0, 120.0);
        }
        context.set_fill_style_str("#ff304f");
        context.fill_rect(x - 13.0, height / 2.0 - 18.0, 26.0, 36.0);

        if let Some(animation) = animation_weak.upgrade() {
            if let Some(callback) = animation.borrow().as_ref() {
                if let Ok(id) =
                    window_ref.request_animation_frame(callback.as_ref().unchecked_ref())
                {
                    frame_id_ref.set(Some(id));
                }
            }
        }
    }) as Box<dyn FnMut(f64)>));

    if let Some(callback) = animation_ref.borrow().as_ref() {
        if let Ok(id) = window.request_animation_frame(callback.as_ref().unchecked_ref()) {
            frame_id.set(Some(id));
        }
    }
    on_cleanup(move || {
        if let Some(id) = cleanup_frame_id.get() {
            let _ = cleanup_window.cancel_animation_frame(id);
        }
        cleanup_animation_ref.borrow_mut().take();
    });
}

#[component]
fn RayList(rays: Vec<Ray>, empty_message: &'static str) -> impl IntoView {
    if rays.is_empty() {
        return view! {
            <div class="empty-state">
                <span aria-hidden="true">"◌"</span>
                <p>{empty_message}</p>
            </div>
        }
        .into_any();
    }

    view! {
        <div class="ray-list">
            <For
                each=move || rays.clone()
                key=|ray| ray.id
                children=move |ray| view! { <RayCard ray/> }
            />
        </div>
    }
    .into_any()
}

#[component]
fn RayCard(ray: Ray) -> impl IntoView {
    let identity = expect_context::<RwSignal<IdentityState>>();
    let toast = expect_context::<ToastContext>();
    let count = RwSignal::new(ray.prism_count);
    let prismed = RwSignal::new(false);
    let pending = RwSignal::new(false);
    let users_prismed = ray.users_prismed.clone();
    let ray_id = ray.id;

    Effect::new(move |_| {
        if let IdentityState::Ready(user) = identity.get() {
            prismed.set(
                users_prismed
                    .iter()
                    .any(|username| username == &user.username),
            );
        }
    });

    let toggle = move |_| {
        if pending.get_untracked() {
            return;
        }
        let user = match identity.get_untracked() {
            IdentityState::Ready(user) => user,
            IdentityState::Loading => {
                toast.show("Identity is still connecting.", true);
                return;
            }
            IdentityState::Error(error) => {
                toast.show(format!("Identity unavailable: {error}"), true);
                return;
            }
        };
        let target = !prismed.get_untracked();
        pending.set(true);
        spawn_local(async move {
            match client::set_prism(user.id, ray_id, target).await {
                Ok(result) => {
                    count.set(result.prism_count);
                    prismed.set(result.prismed);
                }
                Err(error) => toast.show(error, true),
            }
            pending.set(false);
        });
    };
    let author = if ray.user_name.is_empty() {
        "Unknown".to_string()
    } else {
        ray.user_name
    };

    view! {
        <article class="ray-card">
            <div class="ray-accent" aria-hidden="true"></div>
            <div class="ray-body">
                <div class="ray-meta">
                    <A href=format!("/user/{author}")>"@"{author.clone()}</A>
                    <span>{format!("Ray #{}", ray.id)}</span>
                </div>
                <p>{ray.text}</p>
                <button
                    class="prism-button"
                    class:active=move || prismed.get()
                    type="button"
                    aria-pressed=move || prismed.get().to_string()
                    disabled=move || pending.get()
                    on:click=toggle
                >
                    <span class="prism-icon" aria-hidden="true">
                        {move || if prismed.get() { "◆" } else { "◇" }}
                    </span>
                    <span class="prism-count">
                        {move || {
                            let count = count.get();
                            format!("{count} {}", if count == 1 { "prism" } else { "prisms" })
                        }}
                    </span>
                </button>
            </div>
        </article>
    }
}

#[component]
fn FormStatus(status: RwSignal<Option<(String, bool)>>) -> impl IntoView {
    view! {
        <p
            class="form-status"
            class:error=move || status.get().is_some_and(|(_, is_error)| is_error)
            role="status"
        >
            {move || status.get().map(|(message, _)| message).unwrap_or_default()}
        </p>
    }
}

#[component]
fn Toast() -> impl IntoView {
    let toast = expect_context::<ToastContext>();
    view! {
        <div
            class="toast"
            class:visible=move || toast.state.get().is_some()
            class:error=move || toast.state.get().is_some_and(|(_, is_error)| is_error)
            role="status"
            aria-live="polite"
        >
            {move || toast.state.get().map(|(message, _)| message).unwrap_or_default()}
        </div>
    }
}

#[component]
fn PageLoading(message: &'static str) -> impl IntoView {
    view! { <div class="page-state" role="status">{message}</div> }
}

#[component]
fn PageError(message: String) -> impl IntoView {
    view! {
        <div class="page-state error" role="alert">
            <h1>"Signal lost"</h1>
            <p>{message}</p>
            <A href="/" attr:class="primary-button">"Return home"</A>
        </div>
    }
}

#[component]
fn NotFoundPage() -> impl IntoView {
    view! {
        <Title text="Not found | BeamRS"/>
        <div class="page-state error">
            <h1>"Frequency not found"</h1>
            <p>"The requested route is outside the known spectrum."</p>
            <A href="/" attr:class="primary-button">"Return home"</A>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_state_tracks_ready_user() {
        let state = IdentityState::Ready(User {
            id: 7,
            username: "beam_user".to_string(),
        });
        assert!(matches!(state, IdentityState::Ready(user) if user.id == 7));
    }
}
