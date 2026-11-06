use topcoat::router::{module_router, page};
use topcoat::view::{View, view};

#[tokio::main]
async fn main() {
    topcoat::start(module_router!().build()).await.unwrap();
}

#[page]
async fn home() -> topcoat::Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head><title>"Policy Gate"</title></head>
            <body>
                <h1>"Policy Gate"</h1>
                <p>"loading the realm..."</p>
            </body>
        </html>
    })
}
