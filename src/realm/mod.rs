use topcoat::{router::page, Result, view::{View, view}};

#[page]
pub async fn home() -> Result<impl View> {
    Ok(view! {
        <h1>"Policy Gate"</h1>
        <p>"realm shell — game shell coming soon"</p>
    })
}
