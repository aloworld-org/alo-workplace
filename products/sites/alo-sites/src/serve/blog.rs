//! Dynamic public blog routes. Unlike page snapshots, post publication and
//! alo Docs bodies can change without flipping the site's publish id, so these
//! reads deliberately bypass the page render cache and answer `no-cache`.

use std::sync::Arc;

use axum::response::Response;

use alo_store::site_theme::SiteTheme;
use alo_store::{PublishedSite, PublishedSitePost};
use time::format_description::well_known::Rfc2822;

use crate::blocknote::render_blocknote;
use crate::blog::{
    BlogArticle, BlogCard, BlogFeedItem, BlogPagination, render_blog_feed, render_blog_index,
    render_blog_post,
};
use crate::render::{ImageSources, SiteRenderContext, strings_for};

use super::{AppState, dynamic_html, dynamic_rss, not_found, unavailable};

const BLOG_PAGE_SIZE: u32 = 12;
const RSS_POST_LIMIT: u32 = 50;
const MAX_BLOG_PAGE: u32 = 10_000;

pub(super) async fn serve(
    state: &Arc<AppState>,
    resolved: &PublishedSite,
    public_host: &str,
    path: &str,
    query: Option<&str>,
    themed_not_found: String,
) -> Response {
    let theme = SiteTheme::from_stored(resolved.theme.clone());
    let base_url = format!("https://{public_host}");
    let context = SiteRenderContext {
        name: &resolved.name,
        base_url: &base_url,
        locale: &resolved.default_locale,
        theme: &theme,
        strings: strings_for(&resolved.default_locale),
        images: ImageSources::PublicPaths,
    };

    if path == "/blog/rss.xml" {
        let page = match state
            .store
            .published_posts_page(resolved, 0, RSS_POST_LIMIT)
            .await
        {
            Ok(page) => page,
            Err(error) => {
                tracing::error!(site = %resolved.site, %error, "published blog feed read failed");
                return unavailable();
            }
        };
        return match render_feed(&context, &page.posts) {
            Ok(feed) => dynamic_rss(feed),
            Err(error) => {
                tracing::error!(site = %resolved.site, %error, "published blog feed date failed");
                unavailable()
            }
        };
    }

    if path == "/blog" {
        let requested_page = match parse_page(query) {
            Ok(page) => page,
            Err(()) => return not_found(themed_not_found),
        };
        let offset = (requested_page - 1) * BLOG_PAGE_SIZE;
        let page = match state
            .store
            .published_posts_page(resolved, offset, BLOG_PAGE_SIZE)
            .await
        {
            Ok(page) => page,
            Err(error) => {
                tracing::error!(site = %resolved.site, %error, "published blog index read failed");
                return unavailable();
            }
        };
        let total_pages = total_pages(page.total);
        if requested_page > total_pages {
            return not_found(themed_not_found);
        }
        return dynamic_html(render_index(
            &context,
            &page.posts,
            BlogPagination {
                current_page: requested_page,
                total_pages,
            },
        ));
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

fn render_index(
    context: &SiteRenderContext<'_>,
    posts: &[PublishedSitePost],
    pagination: BlogPagination,
) -> String {
    let dates: Vec<String> = posts
        .iter()
        .map(|post| post.published_at.date().to_string())
        .collect();
    let cards: Vec<BlogCard<'_>> = posts
        .iter()
        .zip(&dates)
        .map(|(post, date)| to_card(post, date))
        .collect();
    render_blog_index(context, &cards, pagination)
}

fn render_feed(
    context: &SiteRenderContext<'_>,
    posts: &[PublishedSitePost],
) -> Result<String, time::error::Format> {
    let iso_dates: Vec<String> = posts
        .iter()
        .map(|post| post.published_at.date().to_string())
        .collect();
    let rss_dates: Vec<String> = posts
        .iter()
        .map(|post| post.published_at.format(&Rfc2822))
        .collect::<Result<_, _>>()?;
    let items: Vec<BlogFeedItem<'_>> = posts
        .iter()
        .zip(&iso_dates)
        .zip(&rss_dates)
        .map(|((post, iso_date), rss_date)| BlogFeedItem {
            card: to_card(post, iso_date),
            published_rfc2822: rss_date,
        })
        .collect();
    Ok(render_blog_feed(context, &items))
}

fn parse_page(query: Option<&str>) -> Result<u32, ()> {
    let mut page = None;
    for pair in query
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
    {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key != "page" {
            continue;
        }
        if page.is_some() {
            return Err(());
        }
        let parsed = value.parse::<u32>().map_err(|_| ())?;
        if !(1..=MAX_BLOG_PAGE).contains(&parsed) {
            return Err(());
        }
        page = Some(parsed);
    }
    Ok(page.unwrap_or(1))
}

fn total_pages(total: u64) -> u32 {
    let pages = total.div_ceil(u64::from(BLOG_PAGE_SIZE)).max(1);
    u32::try_from(pages)
        .unwrap_or(MAX_BLOG_PAGE)
        .min(MAX_BLOG_PAGE)
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
