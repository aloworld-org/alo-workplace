//! Visitor-facing chrome strings of a rendered site, per locale.
//!
//! Section content is the tenant's own words; these are the few strings the
//! *renderer* contributes (skip link, menu button, form labels). They are
//! externalized here — never inline in the markup builders — so more locales
//! are a new const, not a code hunt. English, French, and Dutch ship with the
//! multilingual public-site contract; other valid locales use English chrome
//! while preserving their exact BCP 47 tag in document metadata.

/// The renderer-contributed strings for one locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiStrings {
    /// BCP 47 tag for `<html lang>`.
    pub lang: &'static str,
    /// Skip-navigation link text (first focusable element).
    pub skip_to_content: &'static str,
    /// `aria-label` of the top navigation landmark.
    pub nav_label: &'static str,
    /// `aria-label` of the footer links landmark.
    pub footer_nav_label: &'static str,
    /// Mobile menu toggle button text.
    pub menu: &'static str,
    /// Accessible label of the direct language links.
    pub language_switcher_label: &'static str,
    /// Contact form: name field label.
    pub form_name: &'static str,
    /// Contact form: email field label.
    pub form_email: &'static str,
    /// Contact form: message field label.
    pub form_message: &'static str,
    /// Contact form: honeypot field label (visually hidden; a real visitor
    /// never sees it, but it must read as a plausible field to a bot).
    pub form_website: &'static str,
    /// Contact form: submit button text.
    pub form_send: &'static str,
    /// Contact form: confirmation shown after a successful submission when
    /// the section sets no custom `success_message`.
    pub form_success: &'static str,
    /// Form-result page: heading of the success document `POST /f/…` lands
    /// a no-script submission on.
    pub form_sent_title: &'static str,
    /// Form-result page: heading of every failed-submission document.
    pub form_not_sent_title: &'static str,
    /// Form-result page: text when the submission body could not be read.
    pub form_malformed_text: &'static str,
    /// Form-result page: heading of the rate-limited (429) document.
    pub form_rate_limited_title: &'static str,
    /// Form-result page: text of the rate-limited (429) document.
    pub form_rate_limited_text: &'static str,
    /// Form-result page: appended after a field-level validation message,
    /// telling the visitor how to recover.
    pub form_back_hint: &'static str,
    /// Not-found page: heading (and `<title>` prefix).
    pub not_found_title: &'static str,
    /// Not-found page: explanatory text.
    pub not_found_text: &'static str,
    /// Not-found page: link text back to the site's homepage.
    pub not_found_home: &'static str,
    /// Blog index and breadcrumb: section title.
    pub blog_title: &'static str,
    /// Blog index: text shown when no posts have been published.
    pub blog_empty: &'static str,
    /// Blog card: visible link text into one article.
    pub blog_read_article: &'static str,
    /// Blog article: label before its publication date.
    pub blog_published: &'static str,
    /// Blog chrome: concise link back to the site's homepage.
    pub blog_home: &'static str,
    /// Blog chrome: visible link to the RSS feed.
    pub blog_rss: &'static str,
    /// Blog index: accessible label for page navigation.
    pub blog_pagination_label: &'static str,
    /// Blog index: link to the preceding page.
    pub blog_previous: &'static str,
    /// Blog index: link to the following page.
    pub blog_next: &'static str,
    /// Blog index: current-page label.
    pub blog_page: &'static str,
    /// Blog index: word between current and total page counts.
    pub blog_page_of: &'static str,
    /// Collection section: stable message for a deliberately empty source.
    pub collection_empty: &'static str,
    /// Catalog section: shown when the published catalog holds nothing.
    pub catalog_empty: &'static str,
    /// Catalog item: label on an item that is currently unavailable.
    pub catalog_sold_out: &'static str,
    /// Order form: per-item quantity field label.
    pub order_quantity: &'static str,
    /// Order form: optional phone field label.
    pub order_phone: &'static str,
    /// Order form: optional note field label.
    pub order_note: &'static str,
    /// Order form: the sentence that says an order is a request, not a
    /// purchase — nothing is paid on this page.
    pub order_no_payment: &'static str,
    /// Order form: submit button.
    pub order_send: &'static str,
    /// Order result page: title after an order was recorded.
    pub order_sent_title: &'static str,
    /// Order result page: body after an order was recorded.
    pub order_success: &'static str,
    /// Order result page: title when the order was refused.
    pub order_not_sent_title: &'static str,
    /// Order result page: title under the per-client rate limit.
    pub order_rate_limited_title: &'static str,
    /// Order result page: body under the per-client rate limit.
    pub order_rate_limited_text: &'static str,
    /// Catalog price: what this language writes between whole and decimals.
    pub decimal_separator: &'static str,
    /// Catalog price: what this language writes between thousands.
    pub group_separator: &'static str,
    /// Catalog price: whether the currency leads the amount (`€ 12.50`) or
    /// follows it (`12,50 €`).
    pub price_symbol_leads: bool,
    /// Booking section: label of the day field a visitor picks before seeing
    /// free times.
    pub booking_choose_day: &'static str,
    /// Booking section: button that asks for a day's free times.
    pub booking_see_times: &'static str,
    /// Booking page: heading of the free-times document.
    pub booking_times_title: &'static str,
    /// Booking page: shown when a chosen day has nothing free.
    pub booking_no_times: &'static str,
    /// Booking section/page: the unit an appointment length is written in.
    pub booking_minutes: &'static str,
    /// Booking section/page: label before where the appointment happens.
    pub booking_where: &'static str,
    /// Booking page: label of the group of offered times.
    pub booking_pick_time: &'static str,
    /// Booking page: submit button that takes the appointment.
    pub booking_book: &'static str,
    /// Booking result page: title after an appointment was reserved.
    pub booking_booked_title: &'static str,
    /// Booking result page: body after an appointment was reserved.
    pub booking_booked_text: &'static str,
    /// Booking result page: title when the appointment was not reserved.
    pub booking_not_booked_title: &'static str,
    /// Booking section: shown when the service takes no bookings right now.
    pub booking_closed: &'static str,
    /// Booking result page: title under the per-client rate limit.
    pub booking_rate_limited_title: &'static str,
    /// Booking result page: body under the per-client rate limit.
    pub booking_rate_limited_text: &'static str,
    /// Protected page: heading of the unlock screen (and its `<title>`).
    pub protected_title: &'static str,
    /// Protected page: explanation above the password field.
    pub protected_text: &'static str,
    /// Protected page: label of the password field.
    pub protected_password: &'static str,
    /// Protected page: submit button text.
    pub protected_open: &'static str,
    /// Protected page: shown after a password that did not open the page.
    pub protected_wrong: &'static str,
    /// Protected page: shown when too many attempts came from one visitor.
    pub protected_rate_limited: &'static str,
    /// Assistant widget: launcher button text (also its accessible name).
    pub chat_open: &'static str,
    /// Assistant widget: dialog heading.
    pub chat_title: &'static str,
    /// Assistant widget: accessible label of the close button.
    pub chat_close: &'static str,
    /// Assistant widget: label and placeholder of the question field.
    pub chat_question: &'static str,
    /// Assistant widget: send button text.
    pub chat_send: &'static str,
    /// Assistant widget: the default welcome message, shown as the first bot
    /// bubble when the tenant wrote none — a default is written for them
    /// rather than left blank (ADR 0040 §5).
    pub chat_welcome: &'static str,
    /// Assistant widget: shown while an answer is on its way.
    pub chat_thinking: &'static str,
    /// Assistant widget: prefix before an answer's source links.
    pub chat_sources: &'static str,
    /// Assistant widget: shown when the assistant will not answer — it only
    /// answers what the published site itself supports (ADR 0040 §1).
    pub chat_refusal: &'static str,
    /// Assistant widget: shown when the assistant cannot answer right now.
    pub chat_unavailable: &'static str,
    /// Assistant widget: link text to the site's contact page.
    pub chat_contact: &'static str,
    /// Assistant widget: shown when the visitor asked too quickly.
    pub chat_rate_limited: &'static str,
    /// Assistant widget: shown on a network or server failure.
    pub chat_error: &'static str,
    /// Assistant widget, booking: heading above the offered free times.
    pub chat_book_pick: &'static str,
    /// Assistant widget, booking: link to the service's full booking page.
    pub chat_book_more: &'static str,
    /// Assistant widget, booking: the service currently offers no free time.
    pub chat_book_none: &'static str,
    /// Assistant widget, booking: the confirm button under name and email.
    pub chat_book_confirm: &'static str,
    /// Assistant widget, booking: prefix of the confirmation line.
    pub chat_book_booked: &'static str,
    /// Assistant widget, booking: the picked time was taken meanwhile.
    pub chat_book_taken: &'static str,
    /// Assistant widget, lead capture: the line above the in-chat details
    /// form when the visitor asked to be contacted (ADR 0040 §2).
    pub chat_lead_ask: &'static str,
    /// Assistant widget, lead capture: label of the optional company field.
    pub chat_lead_company: &'static str,
    /// Assistant widget, lead capture: the submit button under the fields.
    pub chat_lead_send: &'static str,
    /// Assistant widget, lead capture: confirmation once the details are
    /// with the business.
    pub chat_lead_saved: &'static str,
    /// Assistant widget, lead capture: the honest answer when the business
    /// already knows this address — nothing was filed twice.
    pub chat_lead_known: &'static str,
    /// The conversation's link to a ticketed event's offer page (S3.04g).
    pub chat_tix_get: &'static str,
    /// Appointment pages and confirmation cards: the .ics download link.
    pub booking_add_calendar: &'static str,
    /// Appointment pages and confirmation cards: the cancel affordance.
    pub booking_cancel: &'static str,
    /// Manage page: heading (and `<title>`) while the appointment stands.
    pub booking_manage_title: &'static str,
    /// Manage page: heading once the appointment is cancelled.
    pub booking_manage_cancelled_title: &'static str,
    /// Manage page: text confirming a cancellation that just happened.
    pub booking_manage_cancelled_text: &'static str,
    /// Manage page: text when the appointment was already cancelled.
    pub booking_manage_already_text: &'static str,
    /// Manage page: text when the appointment already started.
    pub booking_manage_too_late_text: &'static str,
    /// Ticket page: heading (and `<title>`) of the buyer's ticket.
    pub ticket_page_title: &'static str,
    /// Ticket page: label before the number of seats this ticket admits.
    pub ticket_seats_label: &'static str,
    /// Ticket page: label before the holder's name.
    pub ticket_holder_label: &'static str,
    /// Tickets section: the link into the live ticket shop (`/tix`).
    pub tickets_see_offer: &'static str,
    /// Ticket shop: heading (and `<title>`) of the `/tix` listing.
    pub tix_title: &'static str,
    /// Ticket shop: the listing's empty state.
    pub tix_empty: &'static str,
    /// Ticket shop: marker on an event with no seats left.
    pub tix_sold_out: &'static str,
    /// Ticket shop: written after a per-seat price ("… per seat").
    pub tix_per_seat: &'static str,
    /// Ticket shop: the buy-form button — leads to the provider's page.
    pub tix_pay: &'static str,
    /// Ticket shop: shown when no payment provider is wired into this
    /// installation — the shop tells the truth instead of offering a form
    /// that could only fail.
    pub tix_unconfigured: &'static str,
    /// Order return page: heading (and `<title>`).
    pub tix_order_title: &'static str,
    /// Order return page: the payment has not finished yet.
    pub tix_order_open: &'static str,
    /// Order return page: paid, the ticket is still being issued.
    pub tix_order_paid_wait: &'static str,
    /// Order return page: the payment came to nothing; no seats are held.
    pub tix_order_dead: &'static str,
    /// Ticket shop: link back to the event listing.
    pub tix_back: &'static str,
    /// Shop section: the link into the live stock shop (`/shop`).
    pub shop_see_offer: &'static str,
    /// Stock shop: heading (and `<title>`) of the `/shop` listing.
    pub shop_title: &'static str,
    /// Stock shop: the listing's empty state.
    pub shop_empty: &'static str,
    /// Stock shop: label of the how-many field on the buy form.
    pub shop_quantity_label: &'static str,
    /// Stock shop: label of the delivery-address street line.
    pub shop_address_label: &'static str,
    /// Stock shop: label of the delivery-address city field.
    pub shop_city_label: &'static str,
    /// Stock shop: label of the delivery-address postcode field.
    pub shop_postcode_label: &'static str,
    /// Stock shop: label of the two-letter delivery country field.
    pub shop_country_label: &'static str,
    /// Stock shop: written after the flat delivery price ("… delivery").
    pub shop_delivery: &'static str,
    /// Stock shop: shown when no payment provider is wired into this
    /// installation — the shop tells the truth instead of offering a form
    /// that could only fail.
    pub shop_unconfigured: &'static str,
    /// Stock order return page: heading (and `<title>`).
    pub shop_order_title: &'static str,
    /// Stock order return page: paid — the goods are claimed and coming.
    pub shop_order_paid: &'static str,
    /// Stock order return page: the payment came to nothing; nothing is
    /// reserved. (When money did move and could not be honoured, the order's
    /// own stored sentence is shown instead.)
    pub shop_order_dead: &'static str,
    /// Stock shop: link back to the shop listing.
    pub shop_back: &'static str,
}

/// English chrome strings — the v1 default.
pub const EN: UiStrings = UiStrings {
    lang: "en",
    skip_to_content: "Skip to content",
    nav_label: "Main",
    footer_nav_label: "Footer",
    menu: "Menu",
    language_switcher_label: "Languages",
    form_name: "Name",
    form_email: "Email",
    form_message: "Message",
    form_website: "Website",
    form_send: "Send",
    form_success: "Thanks — your message has been sent.",
    form_sent_title: "Message sent",
    form_not_sent_title: "Message not sent",
    form_malformed_text: "The submission could not be read. Please go back and try again.",
    form_rate_limited_title: "Too many messages",
    form_rate_limited_text: "Please wait a few minutes before sending another message.",
    form_back_hint: "Please go back and try again",
    not_found_title: "Page not found",
    not_found_text: "The page you are looking for does not exist or has moved.",
    not_found_home: "Go to the homepage",
    blog_title: "Blog",
    blog_empty: "No articles have been published yet.",
    blog_read_article: "Read article",
    blog_published: "Published",
    blog_home: "Home",
    blog_rss: "RSS",
    blog_pagination_label: "Blog pages",
    blog_previous: "Previous",
    blog_next: "Next",
    blog_page: "Page",
    blog_page_of: "of",
    collection_empty: "Nothing to show yet.",
    catalog_empty: "Nothing on offer yet.",
    catalog_sold_out: "Unavailable",
    order_quantity: "Quantity",
    order_phone: "Phone (optional)",
    order_note: "Note (optional)",
    order_no_payment: "Nothing is paid here — we read every order and get back to you to confirm it.",
    order_send: "Place order",
    order_sent_title: "Order received",
    order_success: "Thanks — we have your order and will be in touch to confirm it.",
    order_not_sent_title: "Order not received",
    order_rate_limited_title: "Too many orders",
    order_rate_limited_text: "Please wait a few minutes before sending another order.",
    decimal_separator: ".",
    group_separator: ",",
    price_symbol_leads: true,
    booking_choose_day: "Choose a day",
    booking_see_times: "See free times",
    booking_times_title: "Free times",
    booking_no_times: "Nothing free on that day. Please try another one.",
    booking_minutes: "minutes",
    booking_where: "Where",
    booking_pick_time: "Pick a time",
    booking_book: "Book this time",
    booking_booked_title: "Appointment booked",
    booking_booked_text: "Thanks — your appointment is in our calendar. Reply to us if anything changes.",
    booking_not_booked_title: "Appointment not booked",
    booking_closed: "This is not taking bookings at the moment.",
    booking_rate_limited_title: "Too many attempts",
    booking_rate_limited_text: "Please wait a few minutes before trying again.",
    protected_title: "This page is protected",
    protected_text: "Enter the password to open this page.",
    protected_password: "Password",
    protected_open: "Open page",
    protected_wrong: "That password does not open this page.",
    protected_rate_limited: "Too many attempts. Please wait a few minutes and try again.",
    chat_open: "Ask us",
    chat_title: "Ask us anything",
    chat_close: "Close",
    chat_question: "Your question",
    chat_send: "Send",
    chat_welcome: "Hello! Ask me anything about what is published on this site.",
    chat_thinking: "Looking that up…",
    chat_sources: "From:",
    chat_refusal: "I can only answer from what is published on this site, and I could not find that.",
    chat_unavailable: "The assistant is not available right now.",
    chat_contact: "Use the contact page instead",
    chat_rate_limited: "Please wait a moment before asking again.",
    chat_error: "Something went wrong. Please try again.",
    chat_book_pick: "Pick a time:",
    chat_book_more: "More times",
    chat_book_none: "No free times in the coming days.",
    chat_book_confirm: "Book",
    chat_book_booked: "Booked:",
    chat_book_taken: "That time has just been taken — pick another.",
    chat_lead_ask: "Leave your details and someone will get back to you.",
    chat_lead_company: "Company (optional)",
    chat_lead_send: "Send my details",
    chat_lead_saved: "Thanks — your details have been passed on. You will hear from us.",
    chat_lead_known: "Thanks — we already have your details, and you will hear from us.",
    chat_tix_get: "Get tickets",
    booking_add_calendar: "Add to your calendar",
    booking_cancel: "Cancel this appointment",
    booking_manage_title: "Your appointment",
    booking_manage_cancelled_title: "Appointment cancelled",
    booking_manage_cancelled_text: "Your appointment has been cancelled. The time is free again.",
    booking_manage_already_text: "This appointment was already cancelled.",
    booking_manage_too_late_text: "This appointment has already started and can no longer be cancelled here.",
    ticket_page_title: "Your ticket",
    ticket_seats_label: "Seats",
    ticket_holder_label: "Issued to",
    tickets_see_offer: "See tickets and dates",
    tix_title: "Tickets",
    tix_empty: "No upcoming events right now.",
    tix_sold_out: "Sold out",
    tix_per_seat: "per seat",
    tix_pay: "Continue to payment",
    tix_unconfigured: "Online ticket sales are not set up on this site yet.",
    tix_order_title: "Your order",
    tix_order_open: "Your payment has not finished yet.",
    tix_order_paid_wait: "Payment received — your ticket is on its way. Reload this page in a moment.",
    tix_order_dead: "The payment did not complete. No seats are held for you.",
    tix_back: "Back to tickets",
    shop_see_offer: "Visit the shop",
    shop_title: "Shop",
    shop_empty: "Nothing is for sale right now.",
    shop_quantity_label: "Quantity",
    shop_address_label: "Street and number",
    shop_city_label: "City",
    shop_postcode_label: "Postal code",
    shop_country_label: "Country code (for example NL)",
    shop_delivery: "delivery per order",
    shop_unconfigured: "Online sales are not set up on this site yet.",
    shop_order_title: "Your order",
    shop_order_paid: "Payment received — your order is confirmed. The shop will send your goods to the address you gave.",
    shop_order_dead: "The payment did not complete. Nothing is reserved for you.",
    shop_back: "Back to the shop",
};

/// French renderer chrome.
pub const FR: UiStrings = UiStrings {
    lang: "fr",
    skip_to_content: "Aller au contenu",
    nav_label: "Principal",
    footer_nav_label: "Pied de page",
    menu: "Menu",
    language_switcher_label: "Langues",
    form_name: "Nom",
    form_email: "E-mail",
    form_message: "Message",
    form_website: "Site web",
    form_send: "Envoyer",
    form_success: "Merci — votre message a été envoyé.",
    form_sent_title: "Message envoyé",
    form_not_sent_title: "Message non envoyé",
    form_malformed_text: "Le formulaire n’a pas pu être lu. Revenez en arrière et réessayez.",
    form_rate_limited_title: "Trop de messages",
    form_rate_limited_text: "Patientez quelques minutes avant d’envoyer un autre message.",
    form_back_hint: "Revenez en arrière et réessayez",
    not_found_title: "Page introuvable",
    not_found_text: "La page recherchée n’existe pas ou a été déplacée.",
    not_found_home: "Aller à l’accueil",
    blog_title: "Blog",
    blog_empty: "Aucun article n’a encore été publié.",
    blog_read_article: "Lire l’article",
    blog_published: "Publié",
    blog_home: "Accueil",
    blog_rss: "RSS",
    blog_pagination_label: "Pages du blog",
    blog_previous: "Précédent",
    blog_next: "Suivant",
    blog_page: "Page",
    blog_page_of: "sur",
    collection_empty: "Rien à afficher pour le moment.",
    catalog_empty: "Rien à proposer pour le moment.",
    catalog_sold_out: "Indisponible",
    order_quantity: "Quantité",
    order_phone: "Téléphone (facultatif)",
    order_note: "Remarque (facultatif)",
    order_no_payment: "Aucun paiement ici — nous lisons chaque commande et vous recontactons pour la confirmer.",
    order_send: "Commander",
    order_sent_title: "Commande reçue",
    order_success: "Merci — nous avons votre commande et vous recontactons pour la confirmer.",
    order_not_sent_title: "Commande non reçue",
    order_rate_limited_title: "Trop de commandes",
    order_rate_limited_text: "Patientez quelques minutes avant d’envoyer une autre commande.",
    decimal_separator: ",",
    group_separator: "\u{202f}",
    price_symbol_leads: false,
    booking_choose_day: "Choisissez un jour",
    booking_see_times: "Voir les heures libres",
    booking_times_title: "Heures libres",
    booking_no_times: "Rien de libre ce jour-là. Essayez un autre jour.",
    booking_minutes: "minutes",
    booking_where: "Lieu",
    booking_pick_time: "Choisissez une heure",
    booking_book: "Réserver cette heure",
    booking_booked_title: "Rendez-vous réservé",
    booking_booked_text: "Merci — votre rendez-vous est dans notre agenda. Répondez-nous si quelque chose change.",
    booking_not_booked_title: "Rendez-vous non réservé",
    booking_closed: "Aucune réservation possible pour le moment.",
    booking_rate_limited_title: "Trop de tentatives",
    booking_rate_limited_text: "Veuillez patienter quelques minutes avant de réessayer.",
    protected_title: "Cette page est protégée",
    protected_text: "Saisissez le mot de passe pour ouvrir cette page.",
    protected_password: "Mot de passe",
    protected_open: "Ouvrir la page",
    protected_wrong: "Ce mot de passe n’ouvre pas cette page.",
    protected_rate_limited: "Trop de tentatives. Patientez quelques minutes et réessayez.",
    chat_open: "Posez votre question",
    chat_title: "Posez-nous vos questions",
    chat_close: "Fermer",
    chat_question: "Votre question",
    chat_send: "Envoyer",
    chat_welcome: "Bonjour ! Posez-moi vos questions sur ce qui est publié sur ce site.",
    chat_thinking: "Je cherche…",
    chat_sources: "Source :",
    chat_refusal: "Je ne peux répondre qu’à partir de ce qui est publié sur ce site, et je n’ai rien trouvé à ce sujet.",
    chat_unavailable: "L’assistant n’est pas disponible pour le moment.",
    chat_contact: "Passez plutôt par la page de contact",
    chat_rate_limited: "Patientez un instant avant de poser une autre question.",
    chat_error: "Une erreur est survenue. Veuillez réessayer.",
    chat_book_pick: "Choisissez une heure :",
    chat_book_more: "Plus d'horaires",
    chat_book_none: "Aucun créneau libre dans les prochains jours.",
    chat_book_confirm: "Réserver",
    chat_book_booked: "Réservé :",
    chat_book_taken: "Ce créneau vient d'être pris — choisissez-en un autre.",
    chat_lead_ask: "Laissez vos coordonnées et quelqu'un vous recontactera.",
    chat_lead_company: "Société (facultatif)",
    chat_lead_send: "Envoyer mes coordonnées",
    chat_lead_saved: "Merci — vos coordonnées ont été transmises. Nous reviendrons vers vous.",
    chat_lead_known: "Merci — nous avons déjà vos coordonnées, nous reviendrons vers vous.",
    chat_tix_get: "Acheter des billets",
    booking_add_calendar: "Ajouter à votre agenda",
    booking_cancel: "Annuler ce rendez-vous",
    booking_manage_title: "Votre rendez-vous",
    booking_manage_cancelled_title: "Rendez-vous annulé",
    booking_manage_cancelled_text: "Votre rendez-vous a été annulé. Le créneau est de nouveau libre.",
    booking_manage_already_text: "Ce rendez-vous était déjà annulé.",
    booking_manage_too_late_text: "Ce rendez-vous a déjà commencé et ne peut plus être annulé ici.",
    ticket_page_title: "Votre billet",
    ticket_seats_label: "Places",
    ticket_holder_label: "Établi au nom de",
    tickets_see_offer: "Voir les billets et les dates",
    tix_title: "Billets",
    tix_empty: "Aucun événement à venir pour le moment.",
    tix_sold_out: "Complet",
    tix_per_seat: "par place",
    tix_pay: "Continuer vers le paiement",
    tix_unconfigured: "La vente de billets en ligne n'est pas encore configurée sur ce site.",
    tix_order_title: "Votre commande",
    tix_order_open: "Votre paiement n'est pas encore terminé.",
    tix_order_paid_wait: "Paiement reçu — votre billet arrive. Rechargez cette page dans un instant.",
    tix_order_dead: "Le paiement n'a pas abouti. Aucune place n'est retenue pour vous.",
    tix_back: "Retour aux billets",
    shop_see_offer: "Visiter la boutique",
    shop_title: "Boutique",
    shop_empty: "Rien n'est en vente pour le moment.",
    shop_quantity_label: "Quantité",
    shop_address_label: "Rue et numéro",
    shop_city_label: "Ville",
    shop_postcode_label: "Code postal",
    shop_country_label: "Code pays (par exemple FR)",
    shop_delivery: "de livraison par commande",
    shop_unconfigured: "La vente en ligne n'est pas encore configurée sur ce site.",
    shop_order_title: "Votre commande",
    shop_order_paid: "Paiement reçu — votre commande est confirmée. La boutique enverra vos articles à l'adresse indiquée.",
    shop_order_dead: "Le paiement n'a pas abouti. Rien n'est réservé pour vous.",
    shop_back: "Retour à la boutique",
};

/// Dutch renderer chrome.
pub const NL: UiStrings = UiStrings {
    lang: "nl",
    skip_to_content: "Naar inhoud",
    nav_label: "Hoofdnavigatie",
    footer_nav_label: "Voettekst",
    menu: "Menu",
    language_switcher_label: "Talen",
    form_name: "Naam",
    form_email: "E-mail",
    form_message: "Bericht",
    form_website: "Website",
    form_send: "Versturen",
    form_success: "Bedankt — je bericht is verstuurd.",
    form_sent_title: "Bericht verstuurd",
    form_not_sent_title: "Bericht niet verstuurd",
    form_malformed_text: "Het formulier kon niet worden gelezen. Ga terug en probeer opnieuw.",
    form_rate_limited_title: "Te veel berichten",
    form_rate_limited_text: "Wacht een paar minuten voordat je nog een bericht verstuurt.",
    form_back_hint: "Ga terug en probeer opnieuw",
    not_found_title: "Pagina niet gevonden",
    not_found_text: "De gezochte pagina bestaat niet of is verplaatst.",
    not_found_home: "Naar de startpagina",
    blog_title: "Blog",
    blog_empty: "Er zijn nog geen artikelen gepubliceerd.",
    blog_read_article: "Lees artikel",
    blog_published: "Gepubliceerd",
    blog_home: "Start",
    blog_rss: "RSS",
    blog_pagination_label: "Blogpagina’s",
    blog_previous: "Vorige",
    blog_next: "Volgende",
    blog_page: "Pagina",
    blog_page_of: "van",
    collection_empty: "Nog niets om te tonen.",
    catalog_empty: "Nog niets in het aanbod.",
    catalog_sold_out: "Niet beschikbaar",
    order_quantity: "Aantal",
    order_phone: "Telefoon (optioneel)",
    order_note: "Opmerking (optioneel)",
    order_no_payment: "Hier wordt niets betaald — we lezen elke bestelling en nemen contact op om te bevestigen.",
    order_send: "Bestelling plaatsen",
    order_sent_title: "Bestelling ontvangen",
    order_success: "Bedankt — we hebben je bestelling en nemen contact op om te bevestigen.",
    order_not_sent_title: "Bestelling niet ontvangen",
    order_rate_limited_title: "Te veel bestellingen",
    order_rate_limited_text: "Wacht een paar minuten voordat je nog een bestelling verstuurt.",
    decimal_separator: ",",
    group_separator: ".",
    price_symbol_leads: true,
    booking_choose_day: "Kies een dag",
    booking_see_times: "Bekijk vrije tijden",
    booking_times_title: "Vrije tijden",
    booking_no_times: "Niets vrij op die dag. Probeer een andere dag.",
    booking_minutes: "minuten",
    booking_where: "Waar",
    booking_pick_time: "Kies een tijd",
    booking_book: "Dit tijdstip boeken",
    booking_booked_title: "Afspraak geboekt",
    booking_booked_text: "Bedankt — uw afspraak staat in onze agenda. Laat het ons weten als er iets verandert.",
    booking_not_booked_title: "Afspraak niet geboekt",
    booking_closed: "Hier kan op dit moment niet geboekt worden.",
    booking_rate_limited_title: "Te veel pogingen",
    booking_rate_limited_text: "Wacht een paar minuten voordat u het opnieuw probeert.",
    protected_title: "Deze pagina is beveiligd",
    protected_text: "Voer het wachtwoord in om deze pagina te openen.",
    protected_password: "Wachtwoord",
    protected_open: "Pagina openen",
    protected_wrong: "Dit wachtwoord opent deze pagina niet.",
    protected_rate_limited: "Te veel pogingen. Wacht een paar minuten en probeer opnieuw.",
    chat_open: "Stel uw vraag",
    chat_title: "Stel ons uw vraag",
    chat_close: "Sluiten",
    chat_question: "Uw vraag",
    chat_send: "Versturen",
    chat_welcome: "Hallo! Stel me gerust een vraag over wat op deze site gepubliceerd is.",
    chat_thinking: "Even opzoeken…",
    chat_sources: "Bron:",
    chat_refusal: "Ik kan alleen antwoorden op basis van wat op deze site gepubliceerd is, en daarover heb ik niets gevonden.",
    chat_unavailable: "De assistent is momenteel niet beschikbaar.",
    chat_contact: "Gebruik liever de contactpagina",
    chat_rate_limited: "Wacht even voordat u een nieuwe vraag stelt.",
    chat_error: "Er ging iets mis. Probeer het opnieuw.",
    chat_book_pick: "Kies een tijdstip:",
    chat_book_more: "Meer tijden",
    chat_book_none: "Geen vrije tijden in de komende dagen.",
    chat_book_confirm: "Boeken",
    chat_book_booked: "Geboekt:",
    chat_book_taken: "Dit tijdstip is net bezet — kies een ander.",
    chat_lead_ask: "Laat uw gegevens achter en iemand neemt contact met u op.",
    chat_lead_company: "Bedrijf (optioneel)",
    chat_lead_send: "Mijn gegevens versturen",
    chat_lead_saved: "Bedankt — uw gegevens zijn doorgegeven. U hoort van ons.",
    chat_lead_known: "Bedankt — wij hebben uw gegevens al, u hoort van ons.",
    chat_tix_get: "Tickets kopen",
    booking_add_calendar: "Toevoegen aan uw agenda",
    booking_cancel: "Deze afspraak annuleren",
    booking_manage_title: "Uw afspraak",
    booking_manage_cancelled_title: "Afspraak geannuleerd",
    booking_manage_cancelled_text: "Uw afspraak is geannuleerd. Het tijdstip is weer vrij.",
    booking_manage_already_text: "Deze afspraak was al geannuleerd.",
    booking_manage_too_late_text: "Deze afspraak is al begonnen en kan hier niet meer worden geannuleerd.",
    ticket_page_title: "Uw ticket",
    ticket_seats_label: "Plaatsen",
    ticket_holder_label: "Op naam van",
    tickets_see_offer: "Bekijk tickets en data",
    tix_title: "Tickets",
    tix_empty: "Geen aankomende evenementen op dit moment.",
    tix_sold_out: "Uitverkocht",
    tix_per_seat: "per plaats",
    tix_pay: "Doorgaan naar betaling",
    tix_unconfigured: "Online ticketverkoop is op deze site nog niet ingesteld.",
    tix_order_title: "Uw bestelling",
    tix_order_open: "Uw betaling is nog niet afgerond.",
    tix_order_paid_wait: "Betaling ontvangen — uw ticket komt eraan. Herlaad deze pagina zo dadelijk.",
    tix_order_dead: "De betaling is niet gelukt. Er worden geen plaatsen voor u vastgehouden.",
    tix_back: "Terug naar tickets",
    shop_see_offer: "Bezoek de winkel",
    shop_title: "Winkel",
    shop_empty: "Er is op dit moment niets te koop.",
    shop_quantity_label: "Aantal",
    shop_address_label: "Straat en huisnummer",
    shop_city_label: "Plaats",
    shop_postcode_label: "Postcode",
    shop_country_label: "Landcode (bijvoorbeeld NL)",
    shop_delivery: "bezorging per bestelling",
    shop_unconfigured: "Online verkoop is op deze site nog niet ingesteld.",
    shop_order_title: "Uw bestelling",
    shop_order_paid: "Betaling ontvangen — uw bestelling is bevestigd. De winkel stuurt uw artikelen naar het opgegeven adres.",
    shop_order_dead: "De betaling is niet gelukt. Er is niets voor u gereserveerd.",
    shop_back: "Terug naar de winkel",
};

/// Renderer chrome for a normalized locale. Region variants inherit their
/// base language; unsupported languages keep usable English chrome.
#[must_use]
pub fn strings_for(locale: &str) -> &'static UiStrings {
    match locale.split('-').next().unwrap_or(locale) {
        "fr" => &FR,
        "nl" => &NL,
        _ => &EN,
    }
}
