//! `POST /_alo/chat/lead` — the visitor leaves their details from the
//! conversation (ADR 0040 §2, item S3.03d).
//!
//! The deterministic half of lead capture, shaped exactly like the booking
//! twin ([`super::chat_book`]): the model only ever *offers* the form (the
//! `lead` state [`super::chat`] answers with), and the capture itself is this
//! route — plain code over
//! [`alo_store::SitePublicStore::capture_conversation_lead`], which resolves
//! the serving site's own tenant and owner and writes through CRM's public
//! lead seam. No model output is anywhere in this path; every field of the
//! lead is the visitor's own typed input, plus two facts of the serving site
//! (the card's localized title and the host it was captured on).
//!
//! The wire mirrors `/_alo/chat`: same body cap, same visitor-token shape,
//! same two rate limiters (leaving details spends conversation budget). A
//! malformed body is 400, a field CRM refuses is 400 with the store's own
//! sentence in `detail` (shown verbatim — the visitor can actually fix it),
//! and success is one of two 200s: `{"state":"lead_saved"}` when a lead now
//! stands on the tenant's board, or `{"state":"lead_known"}` when CRM
//! answered that the business already knows this address (an open deal or a
//! billing customer — deliberately indistinguishable here: which one, and
//! which deal, is tenant data no anonymous wire may carry).
//!
//! Attribution is aggregate only: a successful capture bumps the site's
//! daily `chat`/`submit` counter ([`alo_store::ConversionStage`]) — a number
//! with no visitor in it. Nothing about the conversation travels with the
//! lead: the seam's own type ([`ConversationLead`]) has no field a
//! transcript, question or page view could ride in.

use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use serde::Deserialize;
use serde_json::json;
use time::OffsetDateTime;

use alo_store::{
    ConversationLead, ConversionStage, NewChatAction, PipelineSeed, PublishedSite, StageSeed,
    StoreError,
};

use super::chat::{rate_limited, record_action, state_json, valid_visitor};
use super::forms::client_key;
use super::{AppState, host};

/// The words this route hands CRM, per site language: the raised card's
/// title, and the first-use board seeded only when the tenant has never
/// opened CRM. The board words mirror CRM's own first-use seed
/// (`alo-jmap::crm::seed_words_for`) so a tenant first seeded from a site
/// conversation gets the same board the CRM screen would have given them;
/// the two tables may drift in wording only, never in shape (five stages:
/// three open, won, lost — the design in `docs/design/crm.md` § Seeding).
struct CrmWords {
    /// The raised card's title, before " — <site name>".
    title: &'static str,
    pipeline: &'static str,
    stages: [&'static str; 5],
}

static EN_WORDS: CrmWords = CrmWords {
    title: "Website enquiry",
    pipeline: "Sales",
    stages: ["New", "Qualified", "Proposal", "Won", "Lost"],
};

static FR_WORDS: CrmWords = CrmWords {
    title: "Demande via le site",
    pipeline: "Ventes",
    stages: ["Nouveau", "Qualifié", "Proposition", "Gagné", "Perdu"],
};

static NL_WORDS: CrmWords = CrmWords {
    title: "Aanvraag via de website",
    pipeline: "Verkoop",
    stages: [
        "Nieuw",
        "Gekwalificeerd",
        "Voorstel",
        "Gewonnen",
        "Verloren",
    ],
};

/// The CRM words for a site's default locale, falling back to English — the
/// same primary-subtag rule the widget's own strings resolve by.
fn crm_words_for(tag: &str) -> &'static CrmWords {
    let primary = tag.split(['-', '_']).next().unwrap_or_default();
    match primary.to_ascii_lowercase().as_str() {
        "fr" => &FR_WORDS,
        "nl" => &NL_WORDS,
        _ => &EN_WORDS,
    }
}

/// The first-use board in the site's language, with the flags CRM's design
/// fixes: three open columns, then the winning and the losing one.
fn seed_for(words: &CrmWords) -> PipelineSeed {
    let flags = [
        (false, false),
        (false, false),
        (false, false),
        (true, false),
        (false, true),
    ];
    PipelineSeed {
        name: words.pipeline.to_owned(),
        stages: words
            .stages
            .iter()
            .zip(flags)
            .map(|(name, (is_won, is_lost))| StageSeed {
                name: (*name).to_owned(),
                is_won,
                is_lost,
            })
            .collect(),
    }
}

/// The most characters accepted for the visitor's name and company — the
/// deal-card bound CRM itself enforces, mirrored here so the cap is checked
/// before the database is involved at all.
const LEAD_FIELD_MAX_CHARS: usize = 200;

/// The card's title bound (CRM's `DEAL_TITLE_MAX_CHARS`), applied to our own
/// composed title: a maximal site name must not make a legitimate capture
/// fail CRM's title rule.
const LEAD_TITLE_MAX_CHARS: usize = 200;

/// CRM's deal source bound (`DEAL_SOURCE_MAX_CHARS`), applied to the host.
const LEAD_SOURCE_MAX_CHARS: usize = 60;

#[derive(Deserialize)]
struct LeadBody {
    /// The widget's per-visitor token — rate-limit key only, never stored.
    #[serde(default)]
    visitor: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    company: String,
}

/// Captures one conversation's lead: address limit, parse, visitor limit,
/// host resolution, then CRM's seam through the resolved site's own pair.
pub(super) async fn capture(State(state): State<Arc<AppState>>, request: Request) -> Response {
    if let Err(wait) = state
        .chat_ip_rate
        .allow(&client_key(&request), Instant::now())
    {
        return rate_limited(wait);
    }

    let host_header = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let body = match Json::<LeadBody>::from_request(request, &()).await {
        Ok(Json(body)) => body,
        Err(JsonRejection::BytesRejection(_)) => {
            return state_json(StatusCode::PAYLOAD_TOO_LARGE, json!({"state": "invalid"}));
        }
        Err(_) => return state_json(StatusCode::BAD_REQUEST, json!({"state": "invalid"})),
    };

    if !valid_visitor(&body.visitor) {
        return state_json(StatusCode::BAD_REQUEST, json!({"state": "invalid"}));
    }
    if let Err(wait) = state.chat_visitor_rate.allow(&body.visitor, Instant::now()) {
        return rate_limited(wait);
    }
    // The widget's fields carry the same caps; anything past them is a
    // hand-built request, answered tersely like any other malformed body.
    if body.name.chars().count() > LEAD_FIELD_MAX_CHARS
        || body.company.chars().count() > LEAD_FIELD_MAX_CHARS
    {
        return state_json(StatusCode::BAD_REQUEST, json!({"state": "invalid"}));
    }

    let Some(scope) = host_header.and_then(|value| host::scope(&value, &state.sites_domain)) else {
        return super::not_found(state.unknown_host.clone());
    };
    let resolved = match super::resolve_scope(&state, &scope).await {
        Ok(Some(site)) => site,
        Ok(None) => return super::not_found(state.unknown_host.clone()),
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "chat lead resolver read failed");
            return super::unavailable();
        }
    };

    let words = crm_words_for(&resolved.default_locale);
    let lead = ConversationLead {
        title: bounded(
            &format!("{} — {}", words.title, resolved.name),
            LEAD_TITLE_MAX_CHARS,
        ),
        visitor_name: body.name,
        visitor_email: body.email,
        company_name: body.company,
        source: bounded(scope.host(), LEAD_SOURCE_MAX_CHARS),
    };
    match state
        .store
        .capture_conversation_lead(&resolved, &seed_for(words), &lead)
        .await
    {
        Ok(alo_store::CapturedLead::Created(_)) => {
            record_captured(&state, &resolved).await;
            record_action(&state, &resolved, &NewChatAction::lead_saved()).await;
            state_json(StatusCode::OK, json!({"state": "lead_saved"}))
        }
        // Which record made the lead unnecessary — and that it was a deal at
        // all — stays inside the tenant: the stranger only hears "we know
        // you". Nothing new was raised, so the funnel does not move. The
        // tenant's transcript records the same one bit (S3.03e).
        Ok(alo_store::CapturedLead::AlreadyKnown(_) | alo_store::CapturedLead::AlreadyCustomer) => {
            record_action(&state, &resolved, &NewChatAction::lead_known()).await;
            state_json(StatusCode::OK, json!({"state": "lead_known"}))
        }
        Err(StoreError::Validation(detail)) => state_json(
            StatusCode::BAD_REQUEST,
            json!({"state": "invalid", "detail": detail}),
        ),
        Err(StoreError::NotFound) => super::not_found(state.unknown_host.clone()),
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "chat lead capture failed");
            super::unavailable()
        }
    }
}

/// One more lead in the site's aggregate 'chat' counters — a number with no
/// visitor in it, recorded beside the capture so a failed counter can never
/// undo a lead (or claim one that was not raised).
async fn record_captured(state: &Arc<AppState>, site: &PublishedSite) {
    if let Err(error) = state
        .store
        .record_public_chat_conversion(
            site,
            OffsetDateTime::now_utc().date(),
            ConversionStage::Submit,
        )
        .await
    {
        tracing::error!(site = %site.site, %error, "chat lead conversion count failed");
    }
}

/// The first `max` characters, whole — our own composed strings held to
/// CRM's bounds so a long site name or host degrades to a shorter title
/// rather than a refused capture.
fn bounded(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_owned();
    }
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crm_words_resolve_by_primary_subtag_with_an_english_fallback() {
        assert_eq!(crm_words_for("en").title, EN_WORDS.title);
        assert_eq!(crm_words_for("fr-BE").title, FR_WORDS.title);
        assert_eq!(crm_words_for("nl").title, NL_WORDS.title);
        assert_eq!(crm_words_for("de").title, EN_WORDS.title);
        assert_eq!(crm_words_for("").title, EN_WORDS.title);
    }

    /// The seed's shape is CRM's design and must never drift: five stages,
    /// exactly one winning and one losing, in every language.
    #[test]
    fn every_language_seeds_the_same_board_shape() {
        for words in [&EN_WORDS, &FR_WORDS, &NL_WORDS] {
            let seed = seed_for(words);
            assert!(!seed.name.trim().is_empty());
            assert_eq!(seed.stages.len(), 5);
            assert_eq!(seed.stages.iter().filter(|s| s.is_won).count(), 1);
            assert_eq!(seed.stages.iter().filter(|s| s.is_lost).count(), 1);
            assert!(seed.stages[4].is_lost && seed.stages[3].is_won);
        }
    }

    #[test]
    fn composed_strings_are_bounded_at_character_boundaries() {
        assert_eq!(bounded("short", 60), "short");
        let long = "é".repeat(100);
        assert_eq!(bounded(&long, 60).chars().count(), 60);
    }
}
