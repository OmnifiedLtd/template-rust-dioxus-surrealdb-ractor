use dioxus::prelude::*;

use ui::Navbar;
use ui::admin::AdminDashboard;
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

    // Admin routes (no navbar)
    #[layout(AdminLayout)]
        #[route("/admin")]
        Admin {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const ADMIN_CSS: Asset = asset!("/assets/admin.css");

fn main() {
    // Initialize logging on server
    #[cfg(feature = "server")]
    {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    dioxus::launch(App);
}

/// Server startup hook - initialize job queue system.
#[cfg(feature = "server")]
#[server(endpoint = "/api/__init")]
async fn init_server() -> Result<(), ServerFnError> {
    api::init_job_queue().await.map_err(|e| ServerFnError::new(e.to_string()))
}

#[component]
fn App() -> Element {
    // Initialize server on first render (server-side only)
    #[cfg(feature = "server")]
    {
        use_server_future(move || async move {
            init_server().await.ok()
        });
    }

    rsx! {
        // Global app resources
        document::Link { rel: "icon", href: FAVICON }
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
                to: Route::Admin {},
                "Admin"
            }
        }

        Outlet::<Route> {}
    }
}

/// Layout for admin routes (no navigation, full-width).
#[component]
fn AdminLayout() -> Element {
    rsx! {
        Outlet::<Route> {}
    }
}

/// Admin dashboard page.
#[component]
fn Admin() -> Element {
    rsx! {
        AdminDashboard {}
    }
}
