// Dioxus `rsx!` macro expands to unwraps internally; allow to avoid false positives.
#![allow(clippy::disallowed_methods)]

use dioxus::prelude::*;

use ui::Navbar;
use ui::admin::{AdminJobDetailPage, AdminQueueDetailPage, AdminQueuesPage};
use views::{Blog, Home};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    // Main site routes
    #[layout(WebNavbar)]
        #[route("/")]
        Home {},
        #[route("/blog/:id")]
        Blog { id: i32 },

    // Admin routes with sidebar navigation
    #[layout(AdminLayout)]
        #[route("/admin")]
        AdminRedirect {},
        #[route("/admin/queues")]
        AdminQueues {},
        #[route("/admin/queues/:queue_id")]
        AdminQueueDetail { queue_id: String },
        #[route("/admin/queues/:queue_id/jobs/:job_id")]
        AdminJobDetail { queue_id: String, job_id: String },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const ADMIN_CSS: Asset = asset!("/assets/admin.css");

fn main() {
    #[cfg(feature = "server")]
    {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // Global app resources
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: ADMIN_CSS }

        Router::<Route> {}
    }
}

/// A web-specific Router around the shared `Navbar` component
/// which allows us to use the web-specific `Route` enum.
#[component]
fn WebNavbar() -> Element {
    rsx! {
        Navbar {
            Link {
                to: Route::Home {},
                "Home"
            }
            Link {
                to: Route::Blog { id: 1 },
                "Blog"
            }
            Link {
                to: Route::AdminQueues {},
                "Admin"
            }
        }

        Outlet::<Route> {}
    }
}

/// Layout for admin routes with sidebar navigation.
#[component]
fn AdminLayout() -> Element {
    rsx! {
        div { class: "admin-layout",
            // Sidebar navigation
            aside { class: "admin-sidebar",
                div { class: "sidebar-header",
                    h1 { class: "sidebar-logo", "Job Queue" }
                }
                nav { class: "sidebar-nav",
                    div { class: "nav-section",
                        span { class: "nav-section-title", "Menu" }
                        Link {
                            to: Route::AdminQueues {},
                            class: "nav-link",
                            active_class: "active",
                            span { class: "nav-icon", "▦" }
                            span { "Queues" }
                        }
                    }
                }
                div { class: "sidebar-footer",
                    Link {
                        to: Route::Home {},
                        class: "nav-link nav-link-muted",
                        span { class: "nav-icon", "←" }
                        span { "Back to Site" }
                    }
                }
            }

            // Main content area
            main { class: "admin-main",
                Outlet::<Route> {}
            }
        }
    }
}

/// Redirect /admin to /admin/queues.
#[component]
fn AdminRedirect() -> Element {
    let nav = use_navigator();
    use_effect(move || {
        nav.push(Route::AdminQueues {});
    });
    rsx! {}
}

/// Queues list page.
#[component]
fn AdminQueues() -> Element {
    rsx! {
        AdminQueuesPage {}
    }
}

/// Queue detail page.
#[component]
fn AdminQueueDetail(queue_id: String) -> Element {
    rsx! {
        AdminQueueDetailPage { queue_id }
    }
}

/// Job detail page.
#[component]
fn AdminJobDetail(queue_id: String, job_id: String) -> Element {
    rsx! {
        AdminJobDetailPage { queue_id, job_id }
    }
}
