//! Public blog documents are pure renderer output: these goldens pin the
//! complete index-card and article-page contracts independently of storage.

use alo_sites::blog::{
    BlogArticle, BlogCard, BlogFeedItem, BlogPagination, render_blog_feed, render_blog_index,
    render_blog_post,
};
use alo_sites::render::{EN, ImageSources, SiteRenderContext};
use alo_store::site_theme::SiteTheme;

fn context<'a>(theme: &'a SiteTheme) -> SiteRenderContext<'a> {
    SiteRenderContext {
        name: "North Studio",
        base_url: "https://north.sites.test",
        theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    }
}

#[test]
fn blog_index_cards_match_the_public_contract() {
    let theme = SiteTheme::default();
    let posts = [
        BlogCard {
            slug: "launch-notes",
            title: "Launch notes",
            excerpt: "What we learned & what comes next.",
            cover_blob_id: Some("blob_cover"),
            published_date: "2026-08-09",
        },
        BlogCard {
            slug: "behind-the-scenes",
            title: "Behind the scenes",
            excerpt: "",
            cover_blob_id: None,
            published_date: "2026-08-01",
        },
    ];
    assert_eq!(
        render_blog_index(
            &context(&theme),
            &posts,
            BlogPagination {
                current_page: 2,
                total_pages: 3,
            },
        ),
        include_str!("golden/blog_index.html")
    );
}

#[test]
fn blog_rss_matches_the_discovery_contract() {
    let theme = SiteTheme::default();
    let cards = [
        BlogCard {
            slug: "launch-notes",
            title: "Launch notes",
            excerpt: "What we learned & what comes next.",
            cover_blob_id: Some("blob_cover"),
            published_date: "2026-08-09",
        },
        BlogCard {
            slug: "behind-the-scenes",
            title: "Behind the scenes",
            excerpt: "A look inside <North Studio>.",
            cover_blob_id: None,
            published_date: "2026-08-01",
        },
    ];
    let items = [
        BlogFeedItem {
            card: cards[0],
            published_rfc2822: "Sun, 09 Aug 2026 10:00:00 +0000",
        },
        BlogFeedItem {
            card: cards[1],
            published_rfc2822: "Sat, 01 Aug 2026 08:30:00 +0000",
        },
    ];
    assert_eq!(
        render_blog_feed(&context(&theme), &items),
        include_str!("golden/blog_rss.xml")
    );
}

#[test]
fn blog_post_wraps_safe_doc_html_with_article_metadata() {
    let theme = SiteTheme::default();
    let card = BlogCard {
        slug: "launch-notes",
        title: "Launch notes",
        excerpt: "What we learned & what comes next.",
        cover_blob_id: Some("blob_cover"),
        published_date: "2026-08-09",
    };
    let article = BlogArticle {
        card,
        body_html: "<p>One safe <strong>document</strong>.</p>\n",
    };
    assert_eq!(
        render_blog_post(&context(&theme), &article),
        include_str!("golden/blog_post.html")
    );
}
