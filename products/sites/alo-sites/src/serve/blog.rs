//! Dynamic public blog routes. Unlike page snapshots, post publication and
//! alo Docs bodies can change without flipping the site's publish id, so these
//! reads deliberately bypass the page render cache and answer `no-cache`.

use std::sync::Arc;

use axum::response::Response;

use alo_store::site_theme::SiteTheme;
use alo_store::{PublishedSite, PublishedSitePost};

use crate::blocknote::render_blocknote;
use crate::blog::{BlogArticle, BlogCard, render_blog_index, render_blog_post};
use crate::render::{EN, ImageSources, SiteRenderContext};

use super::{AppState, dynamic_html, not_found, unavailable};

pub(super) async fn serve(
    state: &Arc<AppState>,
    resolved: &PublishedSite,
    subdomain: &str,
    path: &str,
    themed_not_found: String,
) -> Response {
    let theme = SiteTheme::from_stored(resolved.theme.clone());
    let base_url = format!("https://{subdomain}.{}", state.sites_domain);
    let context = SiteRenderContext {
        name: &resolved.name,
        base_url: &base_url,
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };

    if path == "/blog" {
        let posts = match state.store.published_posts(resolved).await {
            Ok(posts) => posts,
            Err(error) => {
                tracing::error!(site = %resolved.site, %error, "published blog index read failed");
                return unavailable();
            }
        };
        return dynamic_html(render_index(&context, &posts));
    }

    let Some(slug) = path.strip_prefix("/blog/").filter(|slug| !slug.is_empty()) else {
        return not_found(themed_not_found);
    };
    if slug.contains('/') {
        return not_found(themed_not_found);
    }
    let stored = match state.store.published_post(resolved, slug).await {
        Ok(Some(post)) => post,
        Ok(None) => return not_found(themed_not_found),
        Err(error) => {
            tracing::error!(site = %resolved.site, post = slug, %error, "published blog post read failed");
            return unavailable();
        }
    };
    let body = match render_blocknote(&stored.body) {
        Ok(body) => body,
        Err(error) => {
            tracing::error!(site = %resolved.site, post = slug, %error, "published blog body is invalid");
            return unavailable();
        }
    };
    let date = stored.post.published_at.date().to_string();
    let card = to_card(&stored.post, &date);
    dynamic_html(render_blog_post(
        &context,
        &BlogArticle {
            card,
            body_html: &body,
        },
    ))
}

fn render_index(context: &SiteRenderContext<'_>, posts: &[PublishedSitePost]) -> String {
    let dates: Vec<String> = posts
        .iter()
        .map(|post| post.published_at.date().to_string())
        .collect();
    let cards: Vec<BlogCard<'_>> = posts
        .iter()
        .zip(&dates)
        .map(|(post, date)| to_card(post, date))
        .collect();
    render_blog_index(context, &cards)
}

fn to_card<'a>(post: &'a PublishedSitePost, date: &'a str) -> BlogCard<'a> {
    BlogCard {
        slug: &post.slug,
        title: &post.title,
        excerpt: &post.excerpt,
        cover_blob_id: post.cover_blob_id.as_ref().map(|blob| blob.as_str()),
        published_date: date,
    }
}
