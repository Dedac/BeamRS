use leptos::*;

#[component]
pub fn BeamShell() -> impl IntoView {
    view! {
        <div class="beam-shell">
            <header class="hero">
                <div class="brand-wrap">
                    <span class="brand">BEAM</span>
                    <span class="tagline">the pun-powered social feed</span>
                </div>
                <nav class="main-nav">
                    <a href="/">Home</a>
                    <a href="/settings">Settings</a>
                </nav>
            </header>
            <main class="layout">
                <aside class="sidebar">
                    <div class="sidebar-section">
                        <h3>Frequencies</h3>
                        <p>Choose a channel to browse rays.</p>
                    </div>
                    <div class="sidebar-section">
                        <a href="/">Welcome</a>
                    </div>
                </aside>
                <section class="content">
                    <h1>BeamRS</h1>
                    <p>Welcome to a Rust reimplementation of Beam with Axum, PostgreSQL, and Leptos.</p>
                </section>
            </main>
        </div>
    }
}
