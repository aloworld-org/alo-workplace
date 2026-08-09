//! Pure rendering of the public blog index and article pages. Post bodies are
//! already escaped semantic fragments from [`crate::blocknote`]; this module
//! supplies the themed document, visible navigation, metadata and cards.

use crate::render::SiteRenderContext;
use crate::render::html::{esc, img_src};

/// Public card metadata, already narrowed by the tenant-scoped store door.
#[derive(Debug, Clone, Copy)]
pub struct BlogCard<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub excerpt: &'a str,
    pub cover_blob_id: Option<&'a str>,
    /// Stable ISO date for `<time datetime>` and the visible v1 fallback.
    pub published_date: &'a str,
}

/// One public article: its card metadata plus safe semantic body HTML.
#[derive(Debug, Clone, Copy)]
pub struct BlogArticle<'a> {
    pub card: BlogCard<'a>,
    pub body_html: &'a str,
}

/// Visible page controls for a bounded blog-index result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlogPagination {
    pub current_page: u32,
    pub total_pages: u32,
}

/// Public RSS metadata for one item. RSS uses an RFC 2822 publication date
/// while the HTML card keeps the compact ISO date.
#[derive(Debug, Clone, Copy)]
pub struct BlogFeedItem<'a> {
    pub card: BlogCard<'a>,
    pub published_rfc2822: &'a str,
}

/// Renders `/blog` as a complete themed document with visible article cards.
#[must_use]
pub fn render_blog_index(
    site: &SiteRenderContext<'_>,
    posts: &[BlogCard<'_>],
    pagination: BlogPagination,
) -> String {
    let title = format!("{} — {}", site.strings.blog_title, site.name);
    let path = page_href(pagination.current_page);
    let mut out = String::with_capacity(8 * 1024);
    push_start(&mut out, site, &title, &path, None, None, "website");
    push_blog_header(&mut out, site);
    out.push_str("<main id=\"main\" class=\"blog-main\">\n");
    out.push_str("<header class=\"blog-heading\"><h1>");
    out.push_str(&esc(site.strings.blog_title));
    out.push_str("</h1></header>\n");
    if posts.is_empty() {
        out.push_str("<p class=\"blog-empty\">");
        out.push_str(&esc(site.strings.blog_empty));
        out.push_str("</p>\n");
    } else {
        out.push_str("<div class=\"blog-grid\" role=\"list\" aria-label=\"");
        out.push_str(&esc(site.strings.blog_title));
        out.push_str("\">\n");
        for post in posts {
            push_card(&mut out, site, post);
        }
        out.push_str("</div>\n");
    }
    push_pagination(&mut out, site, pagination);
    out.push_str("</main>\n</body>\n</html>\n");
    out
}

/// Renders the newest published posts as a deterministic RSS 2.0 document.
#[must_use]
pub fn render_blog_feed(site: &SiteRenderContext<'_>, posts: &[BlogFeedItem<'_>]) -> String {
    let title = format!("{} — {}", site.strings.blog_title, site.name);
    let blog_url = format!("{}/blog", site.base_url);
    let feed_url = format!("{}/blog/rss.xml", site.base_url);
    let mut out = String::with_capacity(8 * 1024);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(
        "<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n<channel>\n<title>",
    );
    out.push_str(&esc(&title));
    out.push_str("</title>\n<link>");
    out.push_str(&esc(&blog_url));
    out.push_str("</link>\n<description>");
    out.push_str(&esc(&title));
    out.push_str("</description>\n<language>");
    out.push_str(&esc(site.strings.lang));
    out.push_str("</language>\n<atom:link href=\"");
    out.push_str(&esc(&feed_url));
    out.push_str("\" rel=\"self\" type=\"application/rss+xml\"/>\n");
    for item in posts {
        let post_url = format!("{}/blog/{}", site.base_url, item.card.slug);
        out.push_str("<item>\n<title>");
        out.push_str(&esc(item.card.title));
        out.push_str("</title>\n<link>");
        out.push_str(&esc(&post_url));
        out.push_str("</link>\n<guid isPermaLink=\"true\">");
        out.push_str(&esc(&post_url));
        out.push_str("</guid>\n<description>");
        out.push_str(&esc(item.card.excerpt));
        out.push_str("</description>\n<pubDate>");
        out.push_str(&esc(item.published_rfc2822));
        out.push_str("</pubDate>\n</item>\n");
    }
    out.push_str("</channel>\n</rss>\n");
    out
}

/// Renders one `/blog/<slug>` article as a complete themed document.
#[must_use]
pub fn render_blog_post(site: &SiteRenderContext<'_>, article: &BlogArticle<'_>) -> String {
    let post = article.card;
    let title = format!("{} — {}", post.title, site.name);
    let path = format!("/blog/{}", post.slug);
    let mut out = String::with_capacity(article.body_html.len() + 8 * 1024);
    push_start(
        &mut out,
        site,
        &title,
        &path,
        Some(post.excerpt),
        post.cover_blob_id,
        "article",
    );
    push_blog_header(&mut out, site);
    out.push_str("<main id=\"main\" class=\"blog-main\">\n<article class=\"blog-post\">\n");
    out.push_str("<header class=\"blog-post-header\"><p class=\"blog-kicker\"><a href=\"/blog\">");
    out.push_str(&esc(site.strings.blog_title));
    out.push_str("</a></p><h1>");
    out.push_str(&esc(post.title));
    out.push_str("</h1><p class=\"blog-date\">");
    out.push_str(&esc(site.strings.blog_published));
    out.push_str(" <time datetime=\"");
    out.push_str(&esc(post.published_date));
    out.push_str("\">");
    out.push_str(&esc(post.published_date));
    out.push_str("</time></p></header>\n");
    if let Some(cover) = post.cover_blob_id {
        out.push_str("<figure class=\"blog-cover\"><img src=\"");
        out.push_str(&site.images.src(cover));
        out.push_str("\" alt=\"\" loading=\"eager\" decoding=\"async\"></figure>\n");
    }
    out.push_str("<div class=\"blog-body\">\n");
    out.push_str(article.body_html);
    out.push_str("</div>\n</article>\n</main>\n</body>\n</html>\n");
    out
}

fn push_card(out: &mut String, site: &SiteRenderContext<'_>, post: &BlogCard<'_>) {
    let href = format!("/blog/{}", post.slug);
    out.push_str("<article class=\"blog-card\" role=\"listitem\">\n");
    if let Some(cover) = post.cover_blob_id {
        out.push_str("<a class=\"blog-card-cover\" href=\"");
        out.push_str(&esc(&href));
        out.push_str("\" tabindex=\"-1\" aria-hidden=\"true\"><img src=\"");
        out.push_str(&site.images.src(cover));
        out.push_str("\" alt=\"\" loading=\"lazy\" decoding=\"async\"></a>\n");
    }
    out.push_str("<div class=\"blog-card-copy\"><p class=\"blog-date\"><time datetime=\"");
    out.push_str(&esc(post.published_date));
    out.push_str("\">");
    out.push_str(&esc(post.published_date));
    out.push_str("</time></p><h2><a href=\"");
    out.push_str(&esc(&href));
    out.push_str("\">");
    out.push_str(&esc(post.title));
    out.push_str("</a></h2>");
    if !post.excerpt.is_empty() {
        out.push_str("<p>");
        out.push_str(&esc(post.excerpt));
        out.push_str("</p>");
    }
    out.push_str("<a class=\"blog-read\" href=\"");
    out.push_str(&esc(&href));
    out.push_str("\">");
    out.push_str(&esc(site.strings.blog_read_article));
    out.push_str("<span class=\"sr-only\">: ");
    out.push_str(&esc(post.title));
    out.push_str("</span></a></div>\n</article>\n");
}

fn push_blog_header(out: &mut String, site: &SiteRenderContext<'_>) {
    out.push_str("<header class=\"blog-nav\"><nav aria-label=\"");
    out.push_str(&esc(site.strings.nav_label));
    out.push_str("\"><a class=\"blog-brand\" href=\"/\">");
    if let Some(logo) = &site.theme.logo {
        out.push_str("<img src=\"");
        out.push_str(&site.images.src(logo.as_str()));
        out.push_str("\" alt=\"\">");
    }
    out.push_str(&esc(site.name));
    out.push_str("</a><a href=\"/\">");
    out.push_str(&esc(site.strings.blog_home));
    out.push_str("</a><a href=\"/blog\">");
    out.push_str(&esc(site.strings.blog_title));
    out.push_str("</a><a href=\"/blog/rss.xml\">");
    out.push_str(&esc(site.strings.blog_rss));
    out.push_str("</a></nav></header>\n");
}

fn push_pagination(out: &mut String, site: &SiteRenderContext<'_>, page: BlogPagination) {
    if page.total_pages <= 1 {
        return;
    }
    out.push_str("<nav class=\"blog-pages\" aria-label=\"");
    out.push_str(&esc(site.strings.blog_pagination_label));
    out.push_str("\">\n");
    if page.current_page > 1 {
        out.push_str("<a rel=\"prev\" href=\"");
        out.push_str(&page_href(page.current_page - 1));
        out.push_str("\">");
        out.push_str(&esc(site.strings.blog_previous));
        out.push_str("</a>");
    } else {
        out.push_str("<span></span>");
    }
    out.push_str("<span>");
    out.push_str(&esc(site.strings.blog_page));
    out.push(' ');
    out.push_str(&page.current_page.to_string());
    out.push(' ');
    out.push_str(&esc(site.strings.blog_page_of));
    out.push(' ');
    out.push_str(&page.total_pages.to_string());
    out.push_str("</span>");
    if page.current_page < page.total_pages {
        out.push_str("<a rel=\"next\" href=\"");
        out.push_str(&page_href(page.current_page + 1));
        out.push_str("\">");
        out.push_str(&esc(site.strings.blog_next));
        out.push_str("</a>");
    } else {
        out.push_str("<span></span>");
    }
    out.push_str("\n</nav>\n");
}

fn page_href(page: u32) -> String {
    if page <= 1 {
        "/blog".to_owned()
    } else {
        format!("/blog?page={page}")
    }
}

#[allow(clippy::too_many_arguments)]
fn push_start(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    title: &str,
    path: &str,
    description: Option<&str>,
    cover_blob_id: Option<&str>,
    og_type: &str,
) {
    let canonical = format!("{}{}", site.base_url, path);
    out.push_str("<!doctype html>\n<html lang=\"");
    out.push_str(&esc(site.strings.lang));
    out.push_str("\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>");
    out.push_str(&esc(title));
    out.push_str("</title>\n<link rel=\"canonical\" href=\"");
    out.push_str(&esc(&canonical));
    out.push_str("\">\n<meta property=\"og:type\" content=\"");
    out.push_str(og_type);
    out.push_str("\">\n<meta property=\"og:site_name\" content=\"");
    out.push_str(&esc(site.name));
    out.push_str("\">\n<meta property=\"og:title\" content=\"");
    out.push_str(&esc(title));
    out.push_str("\">\n<meta property=\"og:url\" content=\"");
    out.push_str(&esc(&canonical));
    out.push_str("\">\n");
    if let Some(description) = description.filter(|value| !value.is_empty()) {
        out.push_str("<meta name=\"description\" content=\"");
        out.push_str(&esc(description));
        out.push_str("\">\n<meta property=\"og:description\" content=\"");
        out.push_str(&esc(description));
        out.push_str("\">\n");
    }
    if let Some(cover) = cover_blob_id {
        out.push_str("<meta property=\"og:image\" content=\"");
        out.push_str(&esc(site.base_url));
        out.push_str(&img_src(cover));
        out.push_str("\">\n");
    }
    if let Some(favicon) = &site.theme.favicon {
        out.push_str("<link rel=\"icon\" href=\"");
        out.push_str(&site.images.src(favicon.as_str()));
        out.push_str("\">\n");
    }
    out.push_str("<link rel=\"alternate\" type=\"application/rss+xml\" title=\"");
    out.push_str(&esc(site.strings.blog_rss));
    out.push_str("\" href=\"/blog/rss.xml\">\n");
    out.push_str("<link rel=\"stylesheet\" href=\"/assets/site.css\">\n</head>\n<body>\n<a class=\"skip-link\" href=\"#main\">");
    out.push_str(&esc(site.strings.skip_to_content));
    out.push_str("</a>\n");
}
